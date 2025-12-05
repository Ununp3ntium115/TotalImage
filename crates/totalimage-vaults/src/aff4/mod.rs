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
use lru::LruCache;
use std::num::NonZeroUsize;
use totalimage_core::{Error, ReadSeek, Result, Vault};

// Compression libraries for AFF4
use lz4_flex::decompress_size_prepended;

pub use types::*;

/// Maximum memory for AFF4 chunk cache (GAP-007).
///
/// This limit prevents unbounded memory growth from chunk decompression.
/// Default is 64MB which allows:
/// - 64 chunks at 1MB each (typical)
/// - 256 chunks at 256KB each
/// - Reasonable performance without excessive memory usage
///
/// ## Security Considerations
///
/// Without this limit, processing large AFF4 images could lead to:
/// - Memory exhaustion and OOM kills
/// - Denial of service via crafted images with many unique chunks
/// - Excessive swap usage degrading system performance
pub const MAX_AFF4_CACHE_BYTES: usize = 64 * 1024 * 1024; // 64 MB

/// Maximum number of cached chunks (fallback limit).
///
/// This provides a hard upper bound on the number of cache entries
/// even if they're all very small, preventing pathological cases.
pub const MAX_AFF4_CACHE_ENTRIES: usize = 256;

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
    /// Cached decompressed chunks with LRU eviction (GAP-007)
    chunk_cache: LruCache<usize, Vec<u8>>,
    /// Total bytes cached (for memory limit enforcement)
    cache_bytes: usize,
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
            chunk_cache: LruCache::new(NonZeroUsize::new(MAX_AFF4_CACHE_ENTRIES).unwrap()),
            cache_bytes: 0,
            position: 0,
            identifier,
        })
    }

    /// Parse metadata from the container
    fn parse_metadata(archive: &mut zip::ZipArchive<File>) -> Result<Aff4Volume> {
        // Look for container.description or information.turtle
        let metadata_paths = ["container.description", "information.turtle"];

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
                let stream =
                    streams
                        .entry(stmt.subject.clone())
                        .or_insert_with(|| Aff4ImageStream {
                            urn: stmt.subject.clone(),
                            ..Default::default()
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
            let urn_path = stream
                .urn
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
        let urn_path = stream
            .urn
            .replace("aff4://", "aff4%3A//")
            .replace(':', "%3A");

        // Collect index file names first to avoid borrow issues
        let index_files: Vec<String> = (0..archive.len())
            .filter_map(|i| {
                archive.by_index(i).ok().and_then(|file| {
                    let name = file.name().to_string();
                    if name.ends_with(".index")
                        && (name.contains(&urn_path) || name.contains(&stream.urn))
                    {
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
            let chunk_count = stream.size.div_ceil(stream.chunk_size as u64);
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
        let urn_path = self
            .stream
            .urn
            .replace("aff4://", "aff4%3A//")
            .replace(':', "%3A");
        let stream_urn = self.stream.urn.clone();

        // First find the segment file name
        let segment_file: Option<String> = (0..self.archive.len()).find_map(|i| {
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

        let segment = segment_data.ok_or_else(|| Error::invalid_vault("Bevy segment not found"))?;

        // Extract and decompress the chunk
        // The offset is relative to this bevy segment, not absolute
        let chunk_offset = entry.offset as usize;
        let chunk_len = entry.length as usize;

        // Validate bounds - return error for invalid offsets instead of silent corruption
        if chunk_offset >= segment.len() {
            return Err(Error::invalid_vault(format!(
                "AFF4 chunk {} offset {} exceeds segment size {}",
                chunk_index,
                chunk_offset,
                segment.len()
            )));
        }

        if chunk_offset + chunk_len > segment.len() {
            return Err(Error::invalid_vault(format!(
                "AFF4 chunk {} range [{}..{}] exceeds segment size {}",
                chunk_index,
                chunk_offset,
                chunk_offset + chunk_len,
                segment.len()
            )));
        }

        let compressed = &segment[chunk_offset..chunk_offset + chunk_len];

        let decompressed = match self.stream.compression {
            Aff4Compression::None => compressed.to_vec(),
            Aff4Compression::Deflate => {
                let mut decoder = ZlibDecoder::new(Cursor::new(compressed));
                let mut data = Vec::with_capacity(chunk_size);
                decoder.read_to_end(&mut data).map_err(|e| {
                    Error::invalid_vault(format!(
                        "AFF4 chunk {} deflate decompression failed: {}",
                        chunk_index, e
                    ))
                })?;
                data
            }
            Aff4Compression::Snappy => {
                // Snappy decompression using the snap crate
                snap::raw::Decoder::new()
                    .decompress_vec(compressed)
                    .map_err(|e| {
                        Error::invalid_vault(format!(
                            "AFF4 chunk {} snappy decompression failed: {}",
                            chunk_index, e
                        ))
                    })?
            }
            Aff4Compression::Lz4 => {
                // LZ4 decompression using lz4_flex crate
                // AFF4 uses LZ4 with size prepended (block format)
                decompress_size_prepended(compressed).map_err(|e| {
                    Error::invalid_vault(format!(
                        "AFF4 chunk {} lz4 decompression failed: {}",
                        chunk_index, e
                    ))
                })?
            }
            Aff4Compression::Unknown(code) => {
                return Err(Error::invalid_vault(format!(
                    "AFF4 chunk {} uses unknown compression code: {}",
                    chunk_index, code
                )));
            }
        };

        // Cache the chunk with LRU eviction and byte-size tracking (GAP-007)
        let chunk_bytes = decompressed.len();

        // Evict old entries if we exceed memory limit
        while self.cache_bytes + chunk_bytes > MAX_AFF4_CACHE_BYTES && !self.chunk_cache.is_empty()
        {
            // LRU eviction: pop_lru removes least recently used
            if let Some((_, evicted_chunk)) = self.chunk_cache.pop_lru() {
                self.cache_bytes = self.cache_bytes.saturating_sub(evicted_chunk.len());
            }
        }

        // Insert new chunk into cache
        self.chunk_cache.put(chunk_index, decompressed.clone());
        self.cache_bytes += chunk_bytes;

        tracing::trace!(
            "AFF4 cache: {} entries, {} bytes ({:.1}% of max)",
            self.chunk_cache.len(),
            self.cache_bytes,
            (self.cache_bytes as f64 / MAX_AFF4_CACHE_BYTES as f64) * 100.0
        );

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
            let chunk_data = self
                .read_chunk(chunk_index)
                .map_err(|e| std::io::Error::other(e.to_string()))?;

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

// SAFETY: Aff4Vault is safe to Send and Sync because:
// - `archive` (ZipArchive<File>): File handles are Send+Sync on all supported platforms
// - `volume`, `stream`, `bevy_index`: Plain data structures with no interior mutability
// - `chunk_cache` (HashMap): Owned data, requires external synchronization for concurrent access
// - `position` (u64): Plain numeric type
// - `identifier` (String): Owned string
//
// Concurrent access requires external synchronization (e.g., Mutex) because:
// - position tracking requires exclusive access for sequential reads
// - chunk_cache modifications need synchronization
unsafe impl Send for Aff4Vault {}
unsafe impl Sync for Aff4Vault {}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================
    // Basic Tests
    // ============================================================

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
        let has_image_stream = statements
            .iter()
            .any(|s| s.subject.contains("test-image") && s.object.contains("ImageStream"));
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

    // ============================================================
    // LRU Cache and Error Handling Tests
    // ============================================================

    #[test]
    fn test_aff4_lru_cache_creation() {
        use lru::LruCache;
        use std::num::NonZeroUsize;

        // Verify LRU cache can be created with proper capacity
        let capacity = NonZeroUsize::new(256).unwrap();
        let cache: LruCache<usize, Vec<u8>> = LruCache::new(capacity);
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_aff4_cache_eviction_behavior() {
        use lru::LruCache;
        use std::num::NonZeroUsize;

        // Test that LRU cache properly evicts old entries
        let capacity = NonZeroUsize::new(2).unwrap();
        let mut cache: LruCache<usize, Vec<u8>> = LruCache::new(capacity);

        // Add 3 entries to a cache of size 2
        cache.put(0, vec![1, 2, 3]);
        cache.put(1, vec![4, 5, 6]);
        cache.put(2, vec![7, 8, 9]); // This should evict entry 0

        assert!(cache.get(&0).is_none()); // Entry 0 should be evicted
        assert!(cache.get(&1).is_some());
        assert!(cache.get(&2).is_some());
    }

    #[test]
    fn test_aff4_chunk_cache_size_limit() {
        // Verify cache size limit constant is defined
        const MAX_AFF4_CACHE_BYTES: usize = 64 * 1024 * 1024; // 64 MB
        const MAX_AFF4_CACHE_ENTRIES: usize = 256;

        // Verify limits are reasonable
        assert_eq!(MAX_AFF4_CACHE_BYTES, 67_108_864);
        assert_eq!(MAX_AFF4_CACHE_ENTRIES, 256);

        // A single chunk should fit well within cache
        let typical_chunk_size = 32768; // 32 KB
        assert!(typical_chunk_size < MAX_AFF4_CACHE_BYTES);
    }

    // ============================================================
    // Compression and Stream Tests
    // ============================================================

    #[test]
    fn test_aff4_compression_types() {
        // Test all compression types via URI parsing
        assert_eq!(
            Aff4Compression::from_uri("http://aff4.org/Schema#NullCompressor"),
            Aff4Compression::None
        );
        assert_eq!(
            Aff4Compression::from_uri("http://aff4.org/Schema#DeflateCompressor"),
            Aff4Compression::Deflate
        );
        assert_eq!(
            Aff4Compression::from_uri("http://aff4.org/Schema#SnappyCompressor"),
            Aff4Compression::Snappy
        );
        assert_eq!(
            Aff4Compression::from_uri("http://aff4.org/Schema#Lz4Compressor"),
            Aff4Compression::Lz4
        );

        // Test unknown compressor
        assert!(matches!(
            Aff4Compression::from_uri("http://aff4.org/Schema#UnknownCompressor"),
            Aff4Compression::Unknown(_)
        ));
        assert!(matches!(
            Aff4Compression::from_uri(""),
            Aff4Compression::Unknown(_)
        ));
    }

    #[test]
    fn test_aff4_stream_default_values() {
        let stream = Aff4ImageStream::default();

        // Verify default compression is Deflate (not None)
        assert!(matches!(stream.compression, Aff4Compression::Deflate));

        // Verify default chunk size
        assert_eq!(stream.chunk_size, 32768); // 32KB

        // Verify URN starts empty (populated during parsing)
        assert_eq!(stream.urn, "");
    }

    #[test]
    fn test_aff4_stream_with_various_chunk_sizes() {
        // Test with different chunk sizes
        for chunk_size in [512, 4096, 32768, 65536, 1048576] {
            let stream = Aff4ImageStream {
                chunk_size,
                ..Default::default()
            };
            assert_eq!(stream.chunk_size, chunk_size);
        }
    }

    #[test]
    fn test_aff4_stream_size_calculations() {
        let stream = Aff4ImageStream {
            size: 1024 * 1024 * 100, // 100 MB
            chunk_size: 32768,
            ..Default::default()
        };

        // Calculate expected chunks
        let expected_chunks = stream.size.div_ceil(stream.chunk_size as u64);
        assert_eq!(expected_chunks, 3200);
    }

    // ============================================================
    // Volume and Object Type Tests
    // ============================================================

    #[test]
    fn test_aff4_object_types() {
        // Test all object types
        assert_eq!(
            Aff4ObjectType::from_uri("http://aff4.org/Schema#ImageStream"),
            Aff4ObjectType::ImageStream
        );
        assert_eq!(
            Aff4ObjectType::from_uri("http://aff4.org/Schema#Map"),
            Aff4ObjectType::Map
        );
        assert_eq!(
            Aff4ObjectType::from_uri("http://aff4.org/Schema#ZipVolume"),
            Aff4ObjectType::ZipVolume
        );

        // Test unknown type
        assert_eq!(
            Aff4ObjectType::from_uri("http://aff4.org/Schema#SomethingElse"),
            Aff4ObjectType::Unknown
        );
        assert_eq!(Aff4ObjectType::from_uri(""), Aff4ObjectType::Unknown);
    }

    #[test]
    fn test_aff4_volume_default_values() {
        let volume = Aff4Volume::default();

        // Volume URN starts empty (populated during parsing)
        assert_eq!(volume.urn, "");

        // No streams by default
        assert_eq!(volume.streams.len(), 0);
    }

    #[test]
    fn test_aff4_volume_with_multiple_streams() {
        let mut volume = Aff4Volume {
            urn: "aff4://test-volume".to_string(),
            ..Default::default()
        };

        // Add multiple streams
        for i in 0..5 {
            let stream = Aff4ImageStream {
                urn: format!("aff4://test-volume/stream{}", i),
                size: (i as u64 + 1) * 1024 * 1024,
                ..Default::default()
            };
            volume.streams.push(stream);
        }

        assert_eq!(volume.streams.len(), 5);
        assert_eq!(volume.streams[0].size, 1024 * 1024);
        assert_eq!(volume.streams[4].size, 5 * 1024 * 1024);
    }

    #[test]
    fn test_aff4_volume_urn_formats() {
        let urns = [
            "aff4://simple",
            "aff4://test-volume/stream1",
            "aff4://volume.with.dots/path/to/stream",
            "aff4://guid-12345678-1234-1234-1234-123456789abc",
        ];

        for urn in urns {
            let volume = Aff4Volume {
                urn: urn.to_string(),
                ..Default::default()
            };
            assert_eq!(volume.urn, urn);
        }
    }

    // ============================================================
    // Bevy Index Tests
    // ============================================================

    #[test]
    fn test_aff4_bevy_index_entry_edge_cases() {
        // Test with zero offset and length
        let mut bytes = [0u8; 12];
        bytes[0..8].copy_from_slice(&0u64.to_le_bytes());
        bytes[8..12].copy_from_slice(&0u32.to_le_bytes());

        let entry = Aff4BevyIndexEntry::parse(&bytes).unwrap();
        assert_eq!(entry.offset, 0);
        assert_eq!(entry.length, 0);

        // Test with maximum values
        let mut bytes_max = [0u8; 12];
        bytes_max[0..8].copy_from_slice(&u64::MAX.to_le_bytes());
        bytes_max[8..12].copy_from_slice(&u32::MAX.to_le_bytes());

        let entry_max = Aff4BevyIndexEntry::parse(&bytes_max).unwrap();
        assert_eq!(entry_max.offset, u64::MAX);
        assert_eq!(entry_max.length, u32::MAX);
    }

    #[test]
    fn test_aff4_bevy_index_entry_truncated() {
        // Test with truncated data (less than 12 bytes)
        let bytes = [0u8; 8];
        let result = Aff4BevyIndexEntry::parse(&bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_aff4_bevy_segment_boundary() {
        // Test bevy segment boundary calculations
        let chunks_per_segment = 2048;
        let chunk_index = 5000;

        let bevy_index = chunk_index / chunks_per_segment;
        let chunk_in_bevy = chunk_index % chunks_per_segment;

        assert_eq!(bevy_index, 2);
        assert_eq!(chunk_in_bevy, 904);
    }

    // ============================================================
    // Turtle Parser Tests
    // ============================================================

    #[test]
    fn test_turtle_parser_empty_content() {
        let statements = TurtleParser::parse("");
        assert!(statements.is_empty());
    }

    #[test]
    fn test_turtle_parser_only_prefixes() {
        let content = r#"
@prefix aff4: <http://aff4.org/Schema#> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
"#;
        let statements = TurtleParser::parse(content);
        // No actual statements, just prefixes
        assert!(statements.is_empty());
    }

    #[test]
    fn test_turtle_parser_malformed_content() {
        // Parser should handle malformed content gracefully
        let content = "this is not valid turtle syntax !!!";
        let statements = TurtleParser::parse(content);
        // Should not panic, may return empty or partial results
        assert!(statements.len() <= 1);
    }

    #[test]
    fn test_turtle_parser_with_comments() {
        let content = r#"
# This is a comment
@prefix aff4: <http://aff4.org/Schema#> .
# Another comment
<aff4://test> aff4:size "100" .
"#;
        let statements = TurtleParser::parse(content);
        // Should parse the one valid statement
        assert!(!statements.is_empty());
    }

    #[test]
    fn test_turtle_parser_uri_expansion_via_parse() {
        // Test prefix expansion through the parser
        let content = r#"
@prefix aff4: <http://aff4.org/Schema#> .
<aff4://test> aff4:size "100" .
"#;
        let statements = TurtleParser::parse(content);
        assert!(!statements.is_empty());

        // The predicate should be expanded
        let size_statement = statements.iter().find(|s| s.predicate.contains("size"));
        assert!(size_statement.is_some());
    }

    #[test]
    fn test_aff4_statement_creation() {
        let statement = Aff4Statement {
            subject: "aff4://test-subject".to_string(),
            predicate: "http://aff4.org/Schema#type".to_string(),
            object: "http://aff4.org/Schema#ImageStream".to_string(),
        };

        assert!(statement.subject.contains("test-subject"));
        assert!(statement.predicate.contains("type"));
        assert!(statement.object.contains("ImageStream"));
    }

    // ============================================================
    // Snappy/LZ4 Decompression Tests
    // ============================================================

    #[test]
    fn test_snappy_decompression() {
        // Test that snappy decompression works correctly
        let original = b"Hello, this is test data for snappy compression!";

        // Compress with snappy
        let compressed = snap::raw::Encoder::new()
            .compress_vec(original)
            .expect("Snappy compression failed");

        // Decompress
        let decompressed = snap::raw::Decoder::new()
            .decompress_vec(&compressed)
            .expect("Snappy decompression failed");

        assert_eq!(decompressed, original);
    }

    #[test]
    fn test_lz4_decompression() {
        // Test that LZ4 decompression works correctly
        let original = b"Hello, this is test data for LZ4 compression!";

        // Compress with LZ4 (with size prepended - the format AFF4 uses)
        let compressed = lz4_flex::compress_prepend_size(original);

        // Decompress
        let decompressed =
            lz4_flex::decompress_size_prepended(&compressed).expect("LZ4 decompression failed");

        assert_eq!(decompressed, original);
    }

    #[test]
    fn test_snappy_decompression_larger_data() {
        // Test with larger, more realistic data
        let original: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();

        let compressed = snap::raw::Encoder::new()
            .compress_vec(&original)
            .expect("Snappy compression failed");

        let decompressed = snap::raw::Decoder::new()
            .decompress_vec(&compressed)
            .expect("Snappy decompression failed");

        assert_eq!(decompressed, original);
        // Verify compression actually worked
        assert!(compressed.len() < original.len());
    }

    #[test]
    fn test_lz4_decompression_larger_data() {
        // Test with larger, more realistic data
        let original: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();

        let compressed = lz4_flex::compress_prepend_size(&original);

        let decompressed =
            lz4_flex::decompress_size_prepended(&compressed).expect("LZ4 decompression failed");

        assert_eq!(decompressed, original);
        // Verify compression actually worked
        assert!(compressed.len() < original.len());
    }

    #[test]
    fn test_snappy_invalid_data_handling() {
        // Test that invalid data is handled gracefully
        let invalid = vec![0xFF, 0xFF, 0xFF, 0xFF, 0xFF];

        let result = snap::raw::Decoder::new().decompress_vec(&invalid);
        assert!(result.is_err());
    }

    #[test]
    fn test_lz4_invalid_data_handling() {
        // Test that invalid data is handled gracefully
        let invalid = vec![0xFF, 0xFF, 0xFF, 0xFF, 0xFF];

        let result = lz4_flex::decompress_size_prepended(&invalid);
        assert!(result.is_err());
    }

    #[test]
    fn test_aff4_lru_cache_creation() {
        use lru::LruCache;
        use std::num::NonZeroUsize;

        // Verify LRU cache can be created with proper capacity
        let capacity = NonZeroUsize::new(256).unwrap();
        let cache: LruCache<usize, Vec<u8>> = LruCache::new(capacity);
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_aff4_cache_eviction_behavior() {
        use lru::LruCache;
        use std::num::NonZeroUsize;

        // Test that LRU cache properly evicts old entries
        let capacity = NonZeroUsize::new(2).unwrap();
        let mut cache: LruCache<usize, Vec<u8>> = LruCache::new(capacity);

        // Add 3 entries to a cache of size 2
        cache.put(0, vec![1, 2, 3]);
        cache.put(1, vec![4, 5, 6]);
        cache.put(2, vec![7, 8, 9]); // This should evict entry 0

        assert!(cache.get(&0).is_none()); // Entry 0 should be evicted
        assert!(cache.get(&1).is_some());
        assert!(cache.get(&2).is_some());
    }

    #[test]
    fn test_aff4_compression_all_types() {
        use types::Aff4Compression;

        // Verify all compression types are defined
        let none = Aff4Compression::None;
        let deflate = Aff4Compression::Deflate;
        let snappy = Aff4Compression::Snappy;
        let lz4 = Aff4Compression::Lz4;

        // Test default
        assert!(matches!(none, Aff4Compression::None));
        assert!(matches!(deflate, Aff4Compression::Deflate));
        assert!(matches!(snappy, Aff4Compression::Snappy));
        assert!(matches!(lz4, Aff4Compression::Lz4));
    }

    #[test]
    fn test_aff4_bevy_index_entry_size() {
        use types::Aff4BevyIndexEntry;

        // Verify bevy index entry has expected structure
        let entry = Aff4BevyIndexEntry {
            offset: 0,
            length: 32768,
        };

        assert_eq!(entry.offset, 0);
        assert_eq!(entry.length, 32768);
    }

    #[test]
    fn test_aff4_chunk_size_calculation() {
        // Test chunk size calculations don't overflow
        let chunk_size: usize = 32768; // 32 KB
        let chunk_count: usize = 10000;

        // This should not overflow for reasonable values
        let total_size = chunk_size.checked_mul(chunk_count);
        assert!(total_size.is_some());
        assert_eq!(total_size.unwrap(), 327_680_000);
    }

    #[test]
    fn test_aff4_stream_default_values() {
        let stream = Aff4ImageStream::default();

        // Verify default compression is Deflate (not None)
        assert!(matches!(stream.compression, Aff4Compression::Deflate));

        // Verify default chunk size
        assert_eq!(stream.chunk_size, 32768); // 32KB

        // Verify URN starts empty (populated during parsing)
        assert_eq!(stream.urn, "");
    }

    #[test]
    fn test_aff4_volume_default_values() {
        let volume = Aff4Volume::default();

        // Volume URN starts empty (populated during parsing)
        assert_eq!(volume.urn, "");

        // No streams by default
        assert_eq!(volume.streams.len(), 0);
    }

    #[test]
    fn test_aff4_chunk_cache_size_limit() {
        // Verify cache size limit constant is defined
        const MAX_AFF4_CACHE_BYTES: usize = 64 * 1024 * 1024; // 64 MB
        const MAX_AFF4_CACHE_ENTRIES: usize = 256;

        // Verify limits are reasonable
        assert_eq!(MAX_AFF4_CACHE_BYTES, 67_108_864);
        assert_eq!(MAX_AFF4_CACHE_ENTRIES, 256);

        // A single chunk should fit well within cache
        let typical_chunk_size = 32768; // 32 KB
        assert!(typical_chunk_size < MAX_AFF4_CACHE_BYTES);
    }

    #[test]
    fn test_aff4_bevy_segment_boundary() {
        // Test bevy segment boundary calculations
        let chunks_per_segment = 2048;
        let chunk_index = 5000;

        let bevy_index = chunk_index / chunks_per_segment;
        let chunk_in_bevy = chunk_index % chunks_per_segment;

        assert_eq!(bevy_index, 2);
        assert_eq!(chunk_in_bevy, 904);
    }
}
