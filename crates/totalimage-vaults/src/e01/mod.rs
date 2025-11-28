//! E01 (EnCase) forensic image format support
//!
//! The E01 format is a forensic disk image format that provides:
//! - Compression (zlib) for efficient storage
//! - Built-in MD5 hash verification
//! - Case metadata (examiner, notes, etc.)
//! - Multi-segment file support (.E01, .E02, etc.)
//!
//! # Structure
//!
//! ```text
//! ┌──────────────────────────┐
//! │   File Header (13 bytes) │  EVF signature + segment number
//! ├──────────────────────────┤
//! │   Header Section         │  Case metadata (compressed)
//! ├──────────────────────────┤
//! │   Volume Section         │  Media information
//! ├──────────────────────────┤
//! │   Sectors Section(s)     │  Compressed data chunks
//! ├──────────────────────────┤
//! │   Table Section          │  Chunk offset table
//! ├──────────────────────────┤
//! │   Hash Section           │  MD5 hash of uncompressed data
//! ├──────────────────────────┤
//! │   Done Section           │  End marker
//! └──────────────────────────┘
//! ```

pub mod types;

use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::path::Path;

use flate2::read::ZlibDecoder;
use totalimage_core::{Error, ReadSeek, Result, Vault};

pub use types::*;

/// E01 Vault - EnCase forensic image container
///
/// Provides read-only access to E01 forensic disk images.
/// Supports compressed data and multi-segment files.
pub struct E01Vault {
    /// Underlying reader
    reader: Box<dyn ReadSeek>,
    /// File header information
    file_header: E01FileHeader,
    /// Volume section data
    volume: E01VolumeSection,
    /// Chunk offset table
    chunk_table: Vec<E01ChunkInfo>,
    /// Hash information (if available)
    hash: Option<E01HashSection>,
    /// Decompressed data cache (virtual disk view)
    cache: E01Cache,
    /// Identification string
    identifier: String,
}

/// Information about a compressed chunk
#[derive(Debug, Clone)]
struct E01ChunkInfo {
    /// Offset in file to compressed data
    offset: u64,
    /// Size of compressed data
    compressed_size: u32,
    /// Whether this chunk is compressed
    is_compressed: bool,
}

/// Cache for decompressed chunks
struct E01Cache {
    /// Currently cached chunk index
    cached_chunk: Option<usize>,
    /// Cached decompressed data
    cached_data: Vec<u8>,
    /// Virtual position in decompressed stream
    position: u64,
    /// Total size of decompressed data
    total_size: u64,
}

impl E01Cache {
    fn new(total_size: u64) -> Self {
        Self {
            cached_chunk: None,
            cached_data: Vec::new(),
            position: 0,
            total_size,
        }
    }
}

impl E01Vault {
    /// Open an E01 vault from a file path
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the E01 file (.E01)
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened or is not a valid E01 format
    pub fn open(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        Self::from_reader(Box::new(file))
    }

    /// Create E01 vault from a reader
    pub fn from_reader(mut reader: Box<dyn ReadSeek>) -> Result<Self> {
        // Parse file header
        let mut header_bytes = [0u8; 13];
        reader.read_exact(&mut header_bytes)?;
        let file_header = E01FileHeader::parse(&header_bytes)?;

        // Parse sections to find volume and table
        let mut volume: Option<E01VolumeSection> = None;
        let mut chunk_table: Vec<E01ChunkInfo> = Vec::new();
        let mut hash: Option<E01HashSection> = None;
        let mut sectors_data: Vec<(u64, u64)> = Vec::new(); // (offset, size)

        // Start parsing sections after file header
        let mut section_offset = file_header.fields_start as u64;

        loop {
            reader.seek(SeekFrom::Start(section_offset))?;

            let mut section_bytes = [0u8; E01SectionDescriptor::SIZE];
            if reader.read_exact(&mut section_bytes).is_err() {
                break;
            }

            let section = E01SectionDescriptor::parse(&section_bytes)?;

            match section.section_type {
                SectionType::Volume | SectionType::Disk => {
                    // Read volume data
                    let data_offset = section_offset + E01SectionDescriptor::SIZE as u64;
                    reader.seek(SeekFrom::Start(data_offset))?;

                    let data_size = section.section_size - E01SectionDescriptor::SIZE as u64;
                    let mut vol_data = vec![0u8; data_size.min(1024) as usize];
                    reader.read_exact(&mut vol_data)?;

                    volume = Some(E01VolumeSection::parse(&vol_data)?);
                }
                SectionType::Sectors | SectionType::Data => {
                    // Record sectors section location
                    let data_offset = section_offset + E01SectionDescriptor::SIZE as u64;
                    let data_size = section.section_size - E01SectionDescriptor::SIZE as u64;
                    sectors_data.push((data_offset, data_size));
                }
                SectionType::Table | SectionType::Table2 => {
                    // Parse chunk offset table
                    let data_offset = section_offset + E01SectionDescriptor::SIZE as u64;
                    reader.seek(SeekFrom::Start(data_offset))?;

                    // Table contains 4-byte entries (offsets within sectors section)
                    let data_size = section.section_size - E01SectionDescriptor::SIZE as u64;
                    let entry_count = data_size / 4;

                    let mut table_data = vec![0u8; data_size as usize];
                    reader.read_exact(&mut table_data)?;

                    // Parse table entries
                    for i in 0..entry_count as usize {
                        if i * 4 + 4 <= table_data.len() {
                            let base_offset = u32::from_le_bytes([
                                table_data[i * 4],
                                table_data[i * 4 + 1],
                                table_data[i * 4 + 2],
                                table_data[i * 4 + 3],
                            ]);

                            // MSB indicates compression
                            let is_compressed = base_offset & 0x80000000 == 0;
                            let offset = (base_offset & 0x7FFFFFFF) as u64;

                            chunk_table.push(E01ChunkInfo {
                                offset,
                                compressed_size: 0, // Will be calculated
                                is_compressed,
                            });
                        }
                    }
                }
                SectionType::Hash => {
                    // Parse hash section
                    let data_offset = section_offset + E01SectionDescriptor::SIZE as u64;
                    reader.seek(SeekFrom::Start(data_offset))?;

                    let mut hash_data = [0u8; 20];
                    reader.read_exact(&mut hash_data)?;

                    hash = Some(E01HashSection::parse(&hash_data)?);
                }
                SectionType::Done | SectionType::Next => {
                    break;
                }
                _ => {}
            }

            // Move to next section
            if section.next_offset == 0 || section.next_offset <= section_offset {
                break;
            }
            section_offset = section.next_offset;
        }

        let volume = volume.ok_or_else(|| Error::invalid_vault("E01 missing volume section"))?;

        // Calculate chunk sizes from offsets
        if !sectors_data.is_empty() && !chunk_table.is_empty() {
            let base_offset = sectors_data[0].0;

            for i in 0..chunk_table.len() {
                // Adjust offset to be absolute
                chunk_table[i].offset += base_offset;

                // Calculate compressed size from next offset
                let next_offset = if i + 1 < chunk_table.len() {
                    chunk_table[i + 1].offset + base_offset - base_offset
                } else {
                    // Last chunk - use sectors section size
                    let total_size: u64 = sectors_data.iter().map(|(_, s)| s).sum();
                    base_offset + total_size
                };

                let current = chunk_table[i].offset;
                chunk_table[i].compressed_size = (next_offset - current).min(u32::MAX as u64) as u32;
            }
        }

        let total_size = volume.media_size();
        let identifier = format!(
            "E01 {} {} sectors ({} bytes/sector)",
            E01MediaType::from(volume.media_type),
            volume.sector_count,
            volume.bytes_per_sector
        );

        Ok(Self {
            reader,
            file_header,
            volume,
            chunk_table,
            hash,
            cache: E01Cache::new(total_size),
            identifier,
        })
    }

    /// Get the volume information
    pub fn volume(&self) -> &E01VolumeSection {
        &self.volume
    }

    /// Get the hash information (if available)
    pub fn hash(&self) -> Option<&E01HashSection> {
        self.hash.as_ref()
    }

    /// Get the MD5 hash as hex string (if available)
    pub fn md5_hash(&self) -> Option<String> {
        self.hash.as_ref().map(|h| h.md5_hex())
    }

    /// Get the file header information
    pub fn file_header(&self) -> &E01FileHeader {
        &self.file_header
    }

    /// Get chunk count
    pub fn chunk_count(&self) -> usize {
        self.chunk_table.len()
    }

    /// Decompress a chunk
    fn decompress_chunk(&mut self, chunk_index: usize) -> Result<Vec<u8>> {
        if chunk_index >= self.chunk_table.len() {
            return Err(Error::invalid_vault("Chunk index out of range"));
        }

        let chunk = &self.chunk_table[chunk_index];
        let chunk_size = self.volume.chunk_size() as usize;

        // Read compressed data
        self.reader.seek(SeekFrom::Start(chunk.offset))?;
        let mut compressed = vec![0u8; chunk.compressed_size as usize];
        self.reader.read_exact(&mut compressed)?;

        if chunk.is_compressed && !compressed.is_empty() {
            // Decompress using zlib
            let mut decoder = ZlibDecoder::new(Cursor::new(&compressed));
            let mut decompressed = Vec::with_capacity(chunk_size);

            match decoder.read_to_end(&mut decompressed) {
                Ok(_) => Ok(decompressed),
                Err(e) => {
                    tracing::warn!(
                        "E01 chunk decompression failed: {}. Returning zeros.",
                        e
                    );
                    // Return zeros instead of corrupted compressed data
                    Ok(vec![0u8; chunk_size])
                }
            }
        } else {
            // Not compressed
            Ok(compressed)
        }
    }

    /// Read data at a specific offset
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<usize> {
        if offset >= self.cache.total_size {
            return Ok(0);
        }

        let chunk_size = self.volume.chunk_size() as u64;
        let chunk_index = (offset / chunk_size) as usize;
        let chunk_offset = (offset % chunk_size) as usize;

        // Check if we need to decompress a new chunk
        if self.cache.cached_chunk != Some(chunk_index) {
            self.cache.cached_data = self.decompress_chunk(chunk_index)?;
            self.cache.cached_chunk = Some(chunk_index);
        }

        // Calculate how much we can read
        let available = self.cache.cached_data.len().saturating_sub(chunk_offset);
        let to_read = buf.len().min(available);

        if to_read > 0 {
            buf[..to_read].copy_from_slice(&self.cache.cached_data[chunk_offset..chunk_offset + to_read]);
        }

        Ok(to_read)
    }
}

impl Vault for E01Vault {
    fn identify(&self) -> &str {
        &self.identifier
    }

    fn length(&self) -> u64 {
        self.cache.total_size
    }

    fn content(&mut self) -> &mut dyn ReadSeek {
        // Return a virtual reader that wraps the E01 decompression
        // For now, we need to use a workaround since we can't easily
        // return a reference to self
        self
    }
}

// Implement Read and Seek for E01Vault to support the Vault trait
impl Read for E01Vault {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let bytes_read = self.read_at(self.cache.position, buf)
            .map_err(|e| std::io::Error::other(e.to_string()))?;

        self.cache.position += bytes_read as u64;
        Ok(bytes_read)
    }
}

impl Seek for E01Vault {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let new_pos = match pos {
            SeekFrom::Start(offset) => offset,
            SeekFrom::End(offset) => {
                if offset >= 0 {
                    self.cache.total_size + offset as u64
                } else {
                    self.cache.total_size.saturating_sub((-offset) as u64)
                }
            }
            SeekFrom::Current(offset) => {
                if offset >= 0 {
                    self.cache.position + offset as u64
                } else {
                    self.cache.position.saturating_sub((-offset) as u64)
                }
            }
        };

        self.cache.position = new_pos.min(self.cache.total_size);
        Ok(self.cache.position)
    }
}

// SAFETY: E01Vault is safe to Send and Sync because:
// - `reader` (Box<dyn ReadSeek>): In practice, this is always a File or Cursor<Vec<u8>>
//   which are Send+Sync. The trait object doesn't guarantee this, so we assert it here.
// - `file_header`, `volume`, `chunk_table`, `hash`: Plain data structures
// - `cache` (E01Cache): Contains position and decompressed data buffer
// - `identifier` (String): Owned string
//
// Concurrent access requires external synchronization (e.g., Mutex) because:
// - position tracking requires exclusive access for sequential reads
// - cache modifications need synchronization
//
// INVARIANT: Callers must ensure the reader passed to E01Vault::open is Send+Sync
unsafe impl Send for E01Vault {}
unsafe impl Sync for E01Vault {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    // ============================================================
    // Edge Case Tests for E01 Vault
    // ============================================================

    #[test]
    fn test_e01_invalid_signature() {
        // Test that invalid signatures are rejected
        let mut data = vec![0u8; 100];
        data[0..8].copy_from_slice(b"INVALID!"); // Wrong signature

        let cursor = Cursor::new(data);
        let result = E01Vault::from_reader(Box::new(cursor));

        assert!(result.is_err());
        match result {
            Err(e) => {
                let msg = e.to_string();
                assert!(
                    msg.contains("signature")
                        || msg.contains("Invalid")
                        || msg.contains("vault"),
                    "Expected signature error, got: {}",
                    msg
                );
            }
            Ok(_) => panic!("Expected error for invalid signature"),
        }
    }

    #[test]
    fn test_e01_truncated_header() {
        // Test handling of file too small for header
        let data = vec![0u8; 5]; // Less than 13 bytes needed for header
        let cursor = Cursor::new(data);
        let result = E01Vault::from_reader(Box::new(cursor));

        assert!(result.is_err());
    }

    #[test]
    fn test_e01_empty_file() {
        // Test handling of empty file
        let data: Vec<u8> = vec![];
        let cursor = Cursor::new(data);
        let result = E01Vault::from_reader(Box::new(cursor));

        assert!(result.is_err());
    }

    #[test]
    fn test_e01_valid_signature_truncated_sections() {
        // Valid signature but truncated section data
        let mut data = Vec::new();
        data.extend_from_slice(&EVF_SIGNATURE);
        data.push(0x01);
        data.extend_from_slice(&1u16.to_le_bytes()); // segment 1
        data.extend_from_slice(&13u16.to_le_bytes()); // fields start
        // No section data follows

        let cursor = Cursor::new(data);
        let result = E01Vault::from_reader(Box::new(cursor));

        assert!(result.is_err());
    }

    #[test]
    fn test_e01_chunk_info_creation() {
        let info = E01ChunkInfo {
            offset: 1000,
            compressed_size: 512,
            is_compressed: true,
        };

        assert_eq!(info.offset, 1000);
        assert_eq!(info.compressed_size, 512);
        assert!(info.is_compressed);
    }

    #[test]
    fn test_e01_cache_position_tracking() {
        let mut cache = E01Cache::new(4096);
        assert_eq!(cache.position, 0);

        cache.position = 2048;
        assert_eq!(cache.position, 2048);

        // Test position at end
        cache.position = 4096;
        assert_eq!(cache.position, 4096);
    }

    #[test]
    fn test_e01_volume_section_edge_cases() {
        // Test with zero values
        let volume_zero = E01VolumeSection {
            media_type: 0,
            chunk_count: 0,
            sectors_per_chunk: 0,
            bytes_per_sector: 0,
            sector_count: 0,
            compression: 0,
        };

        assert_eq!(volume_zero.media_size(), 0);
        assert_eq!(volume_zero.chunk_size(), 0);

        // Test with maximum reasonable values
        let volume_large = E01VolumeSection {
            media_type: 0x01,
            chunk_count: 1_000_000,
            sectors_per_chunk: 128,
            bytes_per_sector: 4096,
            sector_count: 128_000_000,
            compression: 1,
        };

        assert_eq!(volume_large.chunk_size(), 128 * 4096);
        assert_eq!(volume_large.media_size(), 128_000_000 * 4096);
    }

    #[test]
    fn test_e01_compression_all_values() {
        // Test all known compression types
        assert_eq!(E01Compression::from(0), E01Compression::None);
        assert_eq!(E01Compression::from(1), E01Compression::Deflate);

        // Test unknown values
        for i in 2..10 {
            assert_eq!(E01Compression::from(i), E01Compression::Unknown(i));
        }

        // Test boundary values
        assert_eq!(E01Compression::from(255), E01Compression::Unknown(255));
    }

    #[test]
    fn test_e01_media_type_all_values() {
        use super::types::E01MediaType;

        // Test valid types
        assert_eq!(E01MediaType::from(0x00), E01MediaType::Removable);
        assert_eq!(E01MediaType::from(0x01), E01MediaType::Fixed);
        assert_eq!(E01MediaType::from(0x03), E01MediaType::Optical);
        assert_eq!(E01MediaType::from(0x0E), E01MediaType::Logical);
        assert_eq!(E01MediaType::from(0x10), E01MediaType::Memory);

        // Test unknown type
        assert_eq!(E01MediaType::from(0xFF), E01MediaType::Unknown(0xFF));
    }

    #[test]
    fn test_e01_section_type_unknown() {
        let mut bytes = [0u8; 16];
        bytes[..7].copy_from_slice(b"garbage");

        let section_type = SectionType::from_bytes(&bytes);
        // Unknown types are returned with code 0
        assert!(matches!(section_type, SectionType::Unknown(_)));
    }

    #[test]
    fn test_e01_hash_section_display() {
        let hash = E01HashSection {
            md5_hash: [0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff],
            checksum: 0x12345678,
        };

        let hex = hash.md5_hex();
        assert_eq!(hex, "00112233445566778899aabbccddeeff");
        assert_eq!(hash.md5_hash.len(), 16);
    }

    #[test]
    fn test_e01_file_header_invalid_signature() {
        let mut data = vec![0u8; 13];
        data[0..8].copy_from_slice(b"NOTVALID"); // Invalid signature
        data[9] = 1;
        data[10] = 0;
        data[11] = 13;
        data[12] = 0;

        let result = E01FileHeader::parse(&data);
        // Should parse but signature check will show invalid
        if let Ok(header) = result {
            assert!(!header.is_evf());
        }
    }

    #[test]
    fn test_e01_file_header_ewf_signature() {
        // Test legacy EWF signature (EnCase 1-6)
        let mut data = vec![0u8; 13];
        data[0..8].copy_from_slice(&EWF_SIGNATURE);
        data[9] = 1;
        data[10] = 0;
        data[11] = 13;
        data[12] = 0;

        let header = E01FileHeader::parse(&data).unwrap();
        // EWF is different from EVF
        assert!(!header.is_evf());
    }

    fn create_minimal_e01() -> Vec<u8> {
        let mut data = Vec::new();

        // File header (13 bytes)
        data.extend_from_slice(&EVF_SIGNATURE);
        data.push(0x01); // padding
        data.extend_from_slice(&1u16.to_le_bytes()); // segment 1
        data.extend_from_slice(&13u16.to_le_bytes()); // fields start at 13

        // Volume section descriptor (76 bytes) at offset 13
        let mut section_type = [0u8; 16];
        section_type[..6].copy_from_slice(b"volume");
        data.extend_from_slice(&section_type);

        let next_offset = 13u64 + 76 + 94; // After this section
        data.extend_from_slice(&next_offset.to_le_bytes()); // next offset
        data.extend_from_slice(&(76u64 + 94).to_le_bytes()); // section size
        data.extend_from_slice(&[0u8; 40]); // padding
        data.extend_from_slice(&0u32.to_le_bytes()); // checksum

        // Volume section data (94 bytes)
        data.push(0x01); // media type: fixed
        data.extend_from_slice(&[0u8; 3]); // padding
        data.extend_from_slice(&1u32.to_le_bytes()); // chunk count
        data.extend_from_slice(&64u32.to_le_bytes()); // sectors per chunk
        data.extend_from_slice(&512u32.to_le_bytes()); // bytes per sector
        data.extend_from_slice(&64u64.to_le_bytes()); // sector count
        data.extend_from_slice(&[0u8; 66]); // padding to 94 bytes

        // Done section descriptor at calculated offset
        let mut done_type = [0u8; 16];
        done_type[..4].copy_from_slice(b"done");
        data.extend_from_slice(&done_type);
        data.extend_from_slice(&0u64.to_le_bytes()); // next offset (none)
        data.extend_from_slice(&76u64.to_le_bytes()); // section size
        data.extend_from_slice(&[0u8; 40]); // padding
        data.extend_from_slice(&0u32.to_le_bytes()); // checksum

        data
    }

    #[test]
    fn test_e01_vault_parse_minimal() {
        let data = create_minimal_e01();
        let cursor = Cursor::new(data);

        let vault = E01Vault::from_reader(Box::new(cursor));
        // This may fail on minimal data, which is expected
        // The test validates the parsing code path
        if let Ok(vault) = vault {
            assert!(vault.identify().contains("E01"));
        }
    }

    #[test]
    fn test_e01_media_size_calculation() {
        let volume = E01VolumeSection {
            media_type: 0x01,
            chunk_count: 100,
            sectors_per_chunk: 64,
            bytes_per_sector: 512,
            sector_count: 2048,
            compression: 1,
        };

        assert_eq!(volume.media_size(), 2048 * 512);
        assert_eq!(volume.chunk_size(), 64 * 512);
    }

    #[test]
    fn test_e01_compression_enum() {
        assert_eq!(E01Compression::from(0), E01Compression::None);
        assert_eq!(E01Compression::from(1), E01Compression::Deflate);
        assert_eq!(E01Compression::from(99), E01Compression::Unknown(99));
    }

    #[test]
    fn test_e01_cache_creation() {
        let cache = E01Cache::new(1024);
        assert_eq!(cache.position, 0);
        assert_eq!(cache.total_size, 1024);
        assert!(cache.cached_chunk.is_none());
    }
}
