//! AFF4 (Advanced Forensic Format 4) vault implementation
//!
//! AFF4 is a modern forensic disk image format that provides:
//! - ZIP-based container for easy archiving
//! - RDF metadata for semantic information
//! - Multiple compression options (deflate, snappy, lz4)
//! - Sparse image support via map streams
//!
//! # Structure
//!
//! ```text
//! container.aff4 (ZIP file)
//! ├── container.description    (RDF/Turtle metadata)
//! ├── aff4%3A//volume-urn/
//! │   ├── information.turtle   (Volume metadata)
//! │   └── image-urn/
//! │       ├── 00000000         (Bevy segment 0)
//! │       ├── 00000000.index   (Bevy index 0)
//! │       ├── 00000001         (Bevy segment 1)
//! │       └── ...
//! └── ...
//! ```

pub mod types;

use std::collections::HashMap;
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::path::Path;

use flate2::read::ZlibDecoder;
use totalimage_core::{Error, ReadSeek, Result, Vault};

pub use types::*;

/// AFF4 Vault - Advanced Forensic Format container
///
/// Provides read-only access to AFF4 forensic disk images.
pub struct Aff4Vault {
    /// ZIP archive reader
    archive: zip::ZipArchive<File>,
    /// Volume metadata
    volume: Aff4Volume,
    /// Primary image stream
    stream: Aff4ImageStream,
    /// Bevy index (chunk offsets)
    bevy_index: Vec<Aff4BevyIndexEntry>,
    /// Cached decompressed chunks
    chunk_cache: HashMap<usize, Vec<u8>>,
    /// Current read position
    position: u64,
    /// Identification string
    identifier: String,
}

impl Aff4Vault {
    /// Open an AFF4 vault from a file path
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the AFF4 file (.aff4)
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened or is not a valid AFF4 format
    pub fn open(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| Error::invalid_vault(format!("Invalid AFF4 ZIP container: {}", e)))?;

        // Find and parse metadata
        let volume = Self::parse_metadata(&mut archive)?;

        // Get primary image stream
        let stream = volume
            .streams
            .first()
            .cloned()
            .ok_or_else(|| Error::invalid_vault("AFF4 container has no image streams"))?;

        // Load bevy index
        let bevy_index = Self::load_bevy_index(&mut archive, &stream)?;

        let identifier = format!(
            "AFF4 Image ({} bytes, {} chunks)",
            stream.size,
            bevy_index.len()
        );

        Ok(Self {
            archive,
            volume,
            stream,
            bevy_index,
            chunk_cache: HashMap::new(),
            position: 0,
            identifier,
        })
    }

    /// Parse metadata from the container
    fn parse_metadata(archive: &mut zip::ZipArchive<File>) -> Result<Aff4Volume> {
        // Look for container.description or information.turtle
        let metadata_paths = [
            "container.description",
            "information.turtle",
        ];

        let mut statements = Vec::new();

        for path in &metadata_paths {
            if let Ok(mut file) = archive.by_name(path) {
                let mut content = String::new();
                file.read_to_string(&mut content)
                    .map_err(|e| Error::invalid_vault(format!("Failed to read metadata: {}", e)))?;
                statements.extend(TurtleParser::parse(&content));
            }
        }

        // Collect turtle file names first to avoid borrow issues
        let turtle_files: Vec<String> = (0..archive.len())
            .filter_map(|i| {
                archive.by_index(i).ok().and_then(|file| {
                    let name = file.name().to_string();
                    if name.ends_with(".turtle") || name.ends_with(".description") {
                        Some(name)
                    } else {
                        None
                    }
                })
            })
            .collect();

        // Now read the turtle files
        for name in turtle_files {
            if let Ok(mut f) = archive.by_name(&name) {
                let mut content = String::new();
                if f.read_to_string(&mut content).is_ok() {
                    statements.extend(TurtleParser::parse(&content));
                }
            }
        }

        // Build volume from statements
        let mut volume = Aff4Volume::default();
        let mut streams: HashMap<String, Aff4ImageStream> = HashMap::new();

        for stmt in &statements {
            // Find image streams
            if stmt.predicate.contains("type") && stmt.object.contains("ImageStream") {
                let stream = streams.entry(stmt.subject.clone()).or_insert_with(|| {
                    Aff4ImageStream {
                        urn: stmt.subject.clone(),
                        ..Default::default()
                    }
                });
                stream.urn = stmt.subject.clone();
            }

            // Parse stream properties
            if stmt.predicate.contains("size") {
                if let Some(stream) = streams.get_mut(&stmt.subject) {
                    stream.size = stmt.object.parse().unwrap_or(0);
                }
            }

            if stmt.predicate.contains("chunkSize") {
                if let Some(stream) = streams.get_mut(&stmt.subject) {
                    stream.chunk_size = stmt.object.parse().unwrap_or(32768);
                }
            }

            if stmt.predicate.contains("chunksInSegment") {
                if let Some(stream) = streams.get_mut(&stmt.subject) {
                    stream.chunks_per_segment = stmt.object.parse().unwrap_or(2048);
                }
            }

            if stmt.predicate.contains("compressionMethod") {
                if let Some(stream) = streams.get_mut(&stmt.subject) {
                    stream.compression = Aff4Compression::from_uri(&stmt.object);
                }
            }

            // Volume properties
            if stmt.predicate.contains("creationTime") {
                volume.creation_time = Some(stmt.object.clone());
            }

            if stmt.predicate.contains("tool") && !stmt.predicate.contains("Version") {
                volume.tool = Some(stmt.object.clone());
            }

            if stmt.predicate.contains("toolVersion") {
                volume.tool_version = Some(stmt.object.clone());
            }
        }

        volume.streams = streams.into_values().collect();

        // Collect all file names first
        let all_files: Vec<String> = (0..archive.len())
            .filter_map(|i| archive.by_index(i).ok().map(|f| f.name().to_string()))
            .collect();

        // Try to find data paths for streams
        for stream in &mut volume.streams {
            // Convert URN to file path in ZIP
            let urn_path = stream.urn
                .replace("aff4://", "aff4%3A//")
                .replace(':', "%3A");

            // Look for bevy files
            for name in &all_files {
                if name.contains(&urn_path) || name.contains(&stream.urn) {
                    if name.ends_with(".index") {
                        stream.index_path = Some(name.clone());
                    } else if !name.ends_with(".turtle") && !name.ends_with(".description") {
                        // Check if it looks like a bevy segment
                        let basename = name.rsplit('/').next().unwrap_or(name);
                        if basename.chars().all(|c| c.is_ascii_hexdigit()) {
                            stream.data_path = Some(name.clone());
                        }
                    }
                }
            }
        }

        Ok(volume)
    }

    /// Load the bevy index for a stream
    fn load_bevy_index(
        archive: &mut zip::ZipArchive<File>,
        stream: &Aff4ImageStream,
    ) -> Result<Vec<Aff4BevyIndexEntry>> {
        let mut index_entries = Vec::new();

        // Find all index files for this stream
        let urn_path = stream.urn
            .replace("aff4://", "aff4%3A//")
            .replace(':', "%3A");

        // Collect index file names first to avoid borrow issues
        let index_files: Vec<String> = (0..archive.len())
            .filter_map(|i| {
                archive.by_index(i).ok().and_then(|file| {
                    let name = file.name().to_string();
                    if name.ends_with(".index") && (name.contains(&urn_path) || name.contains(&stream.urn)) {
                        Some(name)
                    } else {
                        None
                    }
                })
            })
            .collect();

        // Now read the index files
        for name in index_files {
            if let Ok(mut f) = archive.by_name(&name) {
                let mut index_data = Vec::new();
                f.read_to_end(&mut index_data)
                    .map_err(|e| Error::invalid_vault(format!("Failed to read index: {}", e)))?;

                // Parse index entries
                for chunk in index_data.chunks_exact(Aff4BevyIndexEntry::SIZE) {
                    if let Ok(entry) = Aff4BevyIndexEntry::parse(chunk) {
                        index_entries.push(entry);
                    }
                }
            }
        }

        // If no index found, calculate from stream size
        if index_entries.is_empty() && stream.size > 0 {
            let chunk_count = (stream.size + stream.chunk_size as u64 - 1) / stream.chunk_size as u64;
            for i in 0..chunk_count {
                index_entries.push(Aff4BevyIndexEntry {
                    offset: i * stream.chunk_size as u64,
                    length: stream.chunk_size,
                });
            }
        }

        Ok(index_entries)
    }

    /// Read and decompress a chunk
    fn read_chunk(&mut self, chunk_index: usize) -> Result<Vec<u8>> {
        // Check cache
        if let Some(cached) = self.chunk_cache.get(&chunk_index) {
            return Ok(cached.clone());
        }

        if chunk_index >= self.bevy_index.len() {
            return Err(Error::invalid_vault("Chunk index out of range"));
        }

        let entry = &self.bevy_index[chunk_index];
        let chunk_size = self.stream.chunk_size as usize;

        // Find and read the bevy segment containing this chunk
        let segment_index = chunk_index / self.stream.chunks_per_segment as usize;
        let segment_name = format!("{:08x}", segment_index);

        // Find the segment file
        let urn_path = self.stream.urn
            .replace("aff4://", "aff4%3A//")
            .replace(':', "%3A");
        let stream_urn = self.stream.urn.clone();

        // First find the segment file name
        let segment_file: Option<String> = (0..self.archive.len())
            .find_map(|i| {
                self.archive.by_index(i).ok().and_then(|file| {
                    let name = file.name().to_string();
                    if (name.contains(&urn_path) || name.contains(&stream_urn))
                        && name.ends_with(&segment_name)
                        && !name.ends_with(".index")
                    {
                        Some(name)
                    } else {
                        None
                    }
                })
            });

        // Now read the segment file
        let segment_data: Option<Vec<u8>> = if let Some(ref name) = segment_file {
            if let Ok(mut f) = self.archive.by_name(name) {
                let mut data = Vec::new();
                f.read_to_end(&mut data)
                    .map_err(|e| Error::invalid_vault(format!("Failed to read segment: {}", e)))?;
                Some(data)
            } else {
                None
            }
        } else {
            None
        };

        let segment = segment_data
            .ok_or_else(|| Error::invalid_vault("Bevy segment not found"))?;

        // Extract and decompress the chunk
        let chunk_offset = entry.offset as usize % segment.len().max(1);
        let chunk_len = (entry.length as usize).min(segment.len().saturating_sub(chunk_offset));

        if chunk_offset + chunk_len > segment.len() {
            // Return zeros for invalid offsets
            return Ok(vec![0u8; chunk_size]);
        }

        let compressed = &segment[chunk_offset..chunk_offset + chunk_len];

        let decompressed = match self.stream.compression {
            Aff4Compression::None => compressed.to_vec(),
            Aff4Compression::Deflate => {
                let mut decoder = ZlibDecoder::new(Cursor::new(compressed));
                let mut data = Vec::with_capacity(chunk_size);
                match decoder.read_to_end(&mut data) {
                    Ok(_) => data,
                    Err(e) => {
                        tracing::warn!(
                            "AFF4 chunk {} decompression failed: {}. Returning zeros.",
                            chunk_index, e
                        );
                        // Return zeros instead of corrupted data
                        vec![0u8; chunk_size]
                    }
                }
            }
            compression => {
                // Snappy/LZ4 not yet implemented - return error
                tracing::warn!(
                    "AFF4 chunk {} uses unsupported compression: {:?}",
                    chunk_index, compression
                );
                return Err(Error::invalid_vault(format!(
                    "Unsupported compression type: {:?}",
                    compression
                )));
            }
        };

        // Cache the chunk with LRU eviction (max 16 entries, ~16MB at 1MB chunks)
        const MAX_CACHE_ENTRIES: usize = 16;
        if self.chunk_cache.len() >= MAX_CACHE_ENTRIES {
            // Simple eviction: clear oldest entries (keep most recent half)
            let mut keys: Vec<_> = self.chunk_cache.keys().copied().collect();
            keys.sort_unstable();
            for key in keys.iter().take(MAX_CACHE_ENTRIES / 2) {
                self.chunk_cache.remove(key);
            }
        }
        self.chunk_cache.insert(chunk_index, decompressed.clone());

        Ok(decompressed)
    }

    /// Get volume metadata
    pub fn volume(&self) -> &Aff4Volume {
        &self.volume
    }

    /// Get the image stream metadata
    pub fn stream(&self) -> &Aff4ImageStream {
        &self.stream
    }

    /// Get chunk count
    pub fn chunk_count(&self) -> usize {
        self.bevy_index.len()
    }
}

impl Read for Aff4Vault {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.position >= self.stream.size {
            return Ok(0);
        }

        let chunk_size = self.stream.chunk_size as u64;
        let remaining = (self.stream.size - self.position) as usize;
        let to_read = buf.len().min(remaining);

        let mut total_read = 0;

        while total_read < to_read {
            let current_pos = self.position + total_read as u64;
            let chunk_index = (current_pos / chunk_size) as usize;
            let chunk_offset = (current_pos % chunk_size) as usize;

            // Read and decompress chunk
            let chunk_data = self.read_chunk(chunk_index)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

            // Calculate how much to copy
            let available = chunk_data.len().saturating_sub(chunk_offset);
            let to_copy = (to_read - total_read).min(available);

            if to_copy == 0 {
                break;
            }

            buf[total_read..total_read + to_copy]
                .copy_from_slice(&chunk_data[chunk_offset..chunk_offset + to_copy]);

            total_read += to_copy;
        }

        self.position += total_read as u64;
        Ok(total_read)
    }
}

impl Seek for Aff4Vault {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let new_pos = match pos {
            SeekFrom::Start(offset) => offset as i64,
            SeekFrom::End(offset) => self.stream.size as i64 + offset,
            SeekFrom::Current(offset) => self.position as i64 + offset,
        };

        if new_pos < 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Seek before beginning of stream",
            ));
        }

        self.position = (new_pos as u64).min(self.stream.size);
        Ok(self.position)
    }
}

impl Vault for Aff4Vault {
    fn identify(&self) -> &str {
        &self.identifier
    }

    fn length(&self) -> u64 {
        self.stream.size
    }

    fn content(&mut self) -> &mut dyn ReadSeek {
        self
    }
}

// Required for ReadSeek trait
unsafe impl Send for Aff4Vault {}
unsafe impl Sync for Aff4Vault {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aff4_volume_default() {
        let volume = Aff4Volume::default();
        assert!(volume.urn.is_empty());
        assert!(volume.streams.is_empty());
    }

    #[test]
    fn test_aff4_stream_default() {
        let stream = Aff4ImageStream::default();
        assert_eq!(stream.chunk_size, 32768);
        assert_eq!(stream.compression, Aff4Compression::Deflate);
    }

    #[test]
    fn test_turtle_parser_basic() {
        let content = r#"
@prefix aff4: <http://aff4.org/Schema#> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .

<aff4://test-image> rdf:type aff4:ImageStream .
<aff4://test-image> aff4:size "1048576" .
<aff4://test-image> aff4:chunkSize "32768" .
"#;

        let statements = TurtleParser::parse(content);
        assert!(statements.len() >= 3);

        // Check that we found the image stream
        let has_image_stream = statements.iter().any(|s| {
            s.subject.contains("test-image") && s.object.contains("ImageStream")
        });
        assert!(has_image_stream);
    }

    #[test]
    fn test_aff4_container_info() {
        let container = Aff4Container {
            volume: Aff4Volume::default(),
            statements: vec![],
        };
        assert!(container.statements.is_empty());
    }
}
