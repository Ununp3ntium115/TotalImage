//! E01 (EnCase) forensic image format writer
//!
//! This module provides functionality for creating E01 forensic disk images.
//! The E01 format supports:
//! - Compression (zlib/deflate)
//! - Multi-segment files (.E01, .E02, etc.)
//! - Built-in MD5 hash verification
//! - Case metadata (examiner, notes, etc.)
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

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use adler::Adler32;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use md5::{Digest, Md5};
use totalimage_vaults::e01::types::{
    E01FileHeader, E01MediaType, E01SectionDescriptor, SectionType, EVF_SIGNATURE,
};

use crate::error::{AcquireError, Result as AcquireResult};

/// E01 writer configuration
#[derive(Debug, Clone)]
pub struct E01WriterConfig {
    /// Media type (removable, fixed, optical, etc.)
    pub media_type: E01MediaType,
    /// Bytes per sector (typically 512)
    pub bytes_per_sector: u32,
    /// Sectors per chunk (typically 64)
    pub sectors_per_chunk: u32,
    /// Compression method (0=none, 1=deflate)
    pub compression: u8,
    /// Maximum segment size in bytes (default: 2GB)
    pub max_segment_size: u64,
    /// Case metadata (optional)
    pub case_info: Option<String>,
    /// Examiner name (optional)
    pub examiner: Option<String>,
}

impl Default for E01WriterConfig {
    fn default() -> Self {
        Self {
            media_type: E01MediaType::Fixed,
            bytes_per_sector: 512,
            sectors_per_chunk: 64,
            compression: 1,                  // deflate
            max_segment_size: 2_000_000_000, // 2GB
            case_info: None,
            examiner: None,
        }
    }
}

/// E01 writer for creating E01 forensic disk images
pub struct E01Writer {
    /// Output file path (base name, segments will be .E01, .E02, etc.)
    output_path: PathBuf,
    /// Current segment file
    current_file: Option<File>,
    /// Current segment number (1-based)
    current_segment: u16,
    /// Configuration
    config: E01WriterConfig,
    /// Chunk table entries (offset, size, compressed_size)
    chunk_table: Vec<(u64, u32, u32)>,
    /// Current chunk buffer
    current_chunk: Vec<u8>,
    /// Current chunk index
    current_chunk_index: usize,
    /// Total sectors written
    sectors_written: u64,
    /// MD5 hasher for uncompressed data
    md5_hasher: Md5,
    /// Current file position
    file_position: u64,
    /// Volume section written flag
    volume_written: bool,
}

impl E01Writer {
    /// Create a new E01 writer
    ///
    /// # Arguments
    ///
    /// * `output_path` - Base path for output files (e.g., "image" will create "image.E01", "image.E02", etc.)
    /// * `config` - Writer configuration
    ///
    /// # Errors
    ///
    /// Returns an error if the output file cannot be created
    pub fn new(output_path: impl AsRef<Path>, config: E01WriterConfig) -> AcquireResult<Self> {
        let output_path = output_path.as_ref().to_path_buf();
        let mut writer = Self {
            output_path,
            current_file: None,
            current_segment: 1,
            config,
            chunk_table: Vec::new(),
            current_chunk: Vec::new(),
            current_chunk_index: 0,
            sectors_written: 0,
            md5_hasher: Md5::new(),
            file_position: 0,
            volume_written: false,
        };

        writer.start_segment()?;
        Ok(writer)
    }

    /// Start a new segment file
    fn start_segment(&mut self) -> AcquireResult<()> {
        // Close previous segment if any
        if let Some(mut file) = self.current_file.take() {
            self.finalize_segment(&mut file)?;
        }

        // Create segment filename
        let segment_path = if self.current_segment == 1 {
            self.output_path.with_extension("E01")
        } else {
            self.output_path
                .with_extension(format!("E{:02}", self.current_segment))
        };

        let mut file = File::create(&segment_path).map_err(|e| {
            AcquireError::WriteError(format!("Failed to create E01 segment: {}", e))
        })?;

        // Write file header
        self.write_file_header(&mut file)?;
        self.file_position = E01FileHeader::SIZE as u64;

        // Write header section (case metadata)
        self.write_header_section(&mut file)?;

        // Write volume section
        self.write_volume_section(&mut file)?;
        self.volume_written = true;

        self.current_file = Some(file);
        self.chunk_table.clear();
        self.current_chunk_index = 0;

        Ok(())
    }

    /// Write E01 file header (13 bytes)
    fn write_file_header(&mut self, file: &mut File) -> AcquireResult<()> {
        let mut header = [0u8; 13];
        header[0..8].copy_from_slice(&EVF_SIGNATURE);
        header[9..11].copy_from_slice(&self.current_segment.to_le_bytes());
        header[11..13].copy_from_slice(&13u16.to_le_bytes()); // fields_start at 13

        file.write_all(&header)?;

        Ok(())
    }

    /// Write header section (case metadata)
    fn write_header_section(&mut self, file: &mut File) -> AcquireResult<()> {
        // Create header data (XML-like metadata)
        let mut header_data = String::new();
        header_data.push_str("<?xml version=\"1.0\"?>\n");
        header_data.push_str("<header>\n");
        if let Some(ref case) = self.config.case_info {
            header_data.push_str(&format!("  <case>{}</case>\n", case));
        }
        if let Some(ref examiner) = self.config.examiner {
            header_data.push_str(&format!("  <examiner>{}</examiner>\n", examiner));
        }
        header_data.push_str("</header>\n");

        let header_bytes = header_data.as_bytes();

        // Compress header data
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(header_bytes)
            .map_err(|e| AcquireError::WriteError(format!("Failed to compress header: {}", e)))?;
        let compressed = encoder.finish().map_err(|e| {
            AcquireError::WriteError(format!("Failed to finish header compression: {}", e))
        })?;

        // Write section descriptor
        let section_size = E01SectionDescriptor::SIZE as u64 + compressed.len() as u64;
        let next_offset = self.file_position + section_size;
        let checksum = calculate_adler32(&compressed);

        self.write_section_descriptor(
            file,
            SectionType::Header,
            next_offset,
            section_size,
            checksum,
        )?;

        // Write compressed header data
        file.write_all(&compressed)?;

        self.file_position = next_offset;
        Ok(())
    }

    /// Write volume section
    fn write_volume_section(&mut self, file: &mut File) -> AcquireResult<()> {
        // Volume section data (94 bytes)
        let mut vol_data = vec![0u8; 94];
        vol_data[0] = match self.config.media_type {
            E01MediaType::Removable => 0x00,
            E01MediaType::Fixed => 0x01,
            E01MediaType::Optical => 0x03,
            E01MediaType::Logical => 0x0E,
            E01MediaType::Memory => 0x10,
            E01MediaType::Unknown(v) => v,
        };

        // Chunk count will be updated during finalization
        // For now, use placeholder
        vol_data[4..8].copy_from_slice(&0u32.to_le_bytes());
        vol_data[8..12].copy_from_slice(&self.config.sectors_per_chunk.to_le_bytes());
        vol_data[12..16].copy_from_slice(&self.config.bytes_per_sector.to_le_bytes());
        // Sector count will be updated during finalization
        vol_data[16..24].copy_from_slice(&0u64.to_le_bytes());
        vol_data[88] = self.config.compression;

        // Write section descriptor
        let section_size = E01SectionDescriptor::SIZE as u64 + vol_data.len() as u64;
        let next_offset = self.file_position + section_size;
        let checksum = calculate_adler32(&vol_data);

        self.write_section_descriptor(
            file,
            SectionType::Volume,
            next_offset,
            section_size,
            checksum,
        )?;

        // Write volume data
        file.write_all(&vol_data)?;

        self.file_position = next_offset;
        Ok(())
    }

    /// Write a section descriptor (76 bytes)
    fn write_section_descriptor(
        &mut self,
        file: &mut File,
        section_type: SectionType,
        next_offset: u64,
        section_size: u64,
        checksum: u32,
    ) -> AcquireResult<()> {
        Self::write_section_descriptor_internal(
            file,
            section_type,
            next_offset,
            section_size,
            checksum,
        )
    }

    /// Write a sector of data
    ///
    /// # Arguments
    ///
    /// * `sector_data` - Sector data (must match bytes_per_sector)
    ///
    /// # Errors
    ///
    /// Returns an error if the sector size doesn't match or write fails
    pub fn write_sector(&mut self, sector_data: &[u8]) -> AcquireResult<()> {
        if sector_data.len() != self.config.bytes_per_sector as usize {
            return Err(AcquireError::SizeMismatch {
                expected: self.config.bytes_per_sector as u64,
                actual: sector_data.len() as u64,
            });
        }

        // Add to current chunk
        self.current_chunk.extend_from_slice(sector_data);
        self.md5_hasher.update(sector_data);
        self.sectors_written += 1;

        let chunk_size = (self.config.sectors_per_chunk * self.config.bytes_per_sector) as usize;

        // If chunk is full, write it
        if self.current_chunk.len() >= chunk_size {
            self.flush_chunk()?;
        }

        // Check if we need a new segment
        if self.file_position > self.config.max_segment_size {
            self.current_segment += 1;
            self.start_segment()?;
        }

        Ok(())
    }

    /// Flush current chunk to file
    fn flush_chunk(&mut self) -> AcquireResult<()> {
        if self.current_chunk.is_empty() {
            return Ok(());
        }

        let file = self
            .current_file
            .as_mut()
            .ok_or_else(|| AcquireError::Internal("No current file".to_string()))?;

        // Start sectors section if needed
        if self.chunk_table.is_empty() {
            Self::write_sectors_section_start_internal(file, &mut self.file_position)?;
        }

        // Compress chunk
        let compressed = if self.config.compression == 1 {
            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(&self.current_chunk).map_err(|e| {
                AcquireError::WriteError(format!("Failed to compress chunk: {}", e))
            })?;
            encoder.finish().map_err(|e| {
                AcquireError::WriteError(format!("Failed to finish compression: {}", e))
            })?
        } else {
            self.current_chunk.clone()
        };

        let chunk_offset = self.file_position;
        let uncompressed_size = self.current_chunk.len() as u32;
        let compressed_size = compressed.len() as u32;

        // Write compressed chunk
        file.write_all(&compressed)?;

        self.file_position += compressed_size as u64;

        // Record in chunk table
        self.chunk_table
            .push((chunk_offset, uncompressed_size, compressed_size));

        // Clear current chunk
        self.current_chunk.clear();
        self.current_chunk_index += 1;

        Ok(())
    }

    /// Internal helper for writing sectors section start
    fn write_sectors_section_start_internal(
        file: &mut File,
        file_position: &mut u64,
    ) -> AcquireResult<()> {
        // Sectors section descriptor (size will be updated later)
        let section_size = E01SectionDescriptor::SIZE as u64; // Will be updated
        let next_offset = *file_position + section_size; // Placeholder
        let checksum = 0; // Will be updated

        Self::write_section_descriptor_internal(
            file,
            SectionType::Sectors,
            next_offset,
            section_size,
            checksum,
        )?;

        *file_position += E01SectionDescriptor::SIZE as u64;
        Ok(())
    }

    /// Internal helper for writing section descriptor
    fn write_section_descriptor_internal(
        file: &mut File,
        section_type: SectionType,
        next_offset: u64,
        section_size: u64,
        checksum: u32,
    ) -> AcquireResult<()> {
        let mut desc = vec![0u8; E01SectionDescriptor::SIZE];
        let type_bytes = section_type.to_bytes();
        desc[0..16].copy_from_slice(&type_bytes);
        desc[16..24].copy_from_slice(&next_offset.to_le_bytes());
        desc[24..32].copy_from_slice(&section_size.to_le_bytes());
        // Padding (40 bytes) already zeroed
        desc[72..76].copy_from_slice(&checksum.to_le_bytes());

        file.write_all(&desc)?;
        Ok(())
    }

    /// Finalize the E01 image
    ///
    /// This writes the table section, hash section, and done section.
    /// Must be called before the writer is dropped.
    ///
    /// # Errors
    ///
    /// Returns an error if finalization fails
    pub fn finalize(mut self) -> AcquireResult<()> {
        // Flush any remaining chunk
        self.flush_chunk()?;

        // Finalize current segment
        if let Some(mut file) = self.current_file.take() {
            self.finalize_segment(&mut file)?;
        }

        Ok(())
    }

    /// Finalize a segment (write table, hash, done sections)
    fn finalize_segment(&mut self, file: &mut File) -> AcquireResult<()> {
        // Update volume section with actual counts
        self.update_volume_section(file)?;

        // Write table section
        self.write_table_section(file)?;

        // Write hash section
        self.write_hash_section(file)?;

        // Write done section
        self.write_done_section(file)?;

        Ok(())
    }

    /// Update volume section with actual counts
    fn update_volume_section(&mut self, _file: &mut File) -> AcquireResult<()> {
        // Volume section is at offset 13 + header section size
        // For simplicity, we'll seek to volume section and update it
        // In a real implementation, we'd track the exact offset
        // For now, this is a placeholder - full implementation would require
        // tracking section offsets more carefully
        Ok(())
    }

    /// Write table section (chunk offset table)
    fn write_table_section(&mut self, file: &mut File) -> AcquireResult<()> {
        // Table section contains chunk offsets
        let mut table_data = Vec::new();

        for (offset, _uncompressed_size, compressed_size) in &self.chunk_table {
            // E01 table format: offset (u64), size (u32)
            // MSB of offset indicates compression
            let table_offset = if self.config.compression == 1 {
                *offset | 0x8000000000000000 // Set MSB for compressed
            } else {
                *offset
            };

            table_data.extend_from_slice(&table_offset.to_le_bytes());
            table_data.extend_from_slice(&compressed_size.to_le_bytes());
        }

        // Write section descriptor
        let section_size = E01SectionDescriptor::SIZE as u64 + table_data.len() as u64;
        let next_offset = self.file_position + section_size;
        let checksum = calculate_adler32(&table_data);

        self.write_section_descriptor(
            file,
            SectionType::Table,
            next_offset,
            section_size,
            checksum,
        )?;

        // Write table data
        file.write_all(&table_data)?;

        self.file_position = next_offset;
        Ok(())
    }

    /// Write hash section (MD5 hash of uncompressed data)
    fn write_hash_section(&mut self, file: &mut File) -> AcquireResult<()> {
        let hash_result = self.md5_hasher.finalize_reset();
        let mut hash_data = vec![0u8; 20];
        hash_data[0..16].copy_from_slice(&hash_result);
        // Checksum (last 4 bytes) - typically 0 or CRC32 of hash
        hash_data[16..20].copy_from_slice(&0u32.to_le_bytes());

        // Write section descriptor
        let section_size = E01SectionDescriptor::SIZE as u64 + hash_data.len() as u64;
        let next_offset = self.file_position + section_size;
        let checksum = calculate_adler32(&hash_data);

        self.write_section_descriptor(
            file,
            SectionType::Hash,
            next_offset,
            section_size,
            checksum,
        )?;

        // Write hash data
        file.write_all(&hash_data)?;

        self.file_position = next_offset;
        Ok(())
    }

    /// Write done section (end marker)
    fn write_done_section(&mut self, file: &mut File) -> AcquireResult<()> {
        // Done section is just a descriptor with no data
        let section_size = E01SectionDescriptor::SIZE as u64;
        let next_offset = 0; // No next section
        let checksum = 0;

        self.write_section_descriptor(
            file,
            SectionType::Done,
            next_offset,
            section_size,
            checksum,
        )?;

        Ok(())
    }
}

/// Calculate Adler-32 checksum
fn calculate_adler32(data: &[u8]) -> u32 {
    let mut adler = Adler32::new();
    adler.write_slice(data);
    adler.checksum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_e01_writer_creation() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("test.E01");

        let config = E01WriterConfig::default();
        let writer = E01Writer::new(&output_path, config);
        assert!(writer.is_ok());
    }

    #[test]
    fn test_e01_write_sectors() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("test");

        let config = E01WriterConfig {
            bytes_per_sector: 512,
            sectors_per_chunk: 2, // Small chunks for testing
            ..Default::default()
        };

        let mut writer = E01Writer::new(&output_path, config).unwrap();

        // Write a few sectors
        let sector_data = vec![0xAA; 512];
        for _ in 0..4 {
            writer.write_sector(&sector_data).unwrap();
        }

        // Finalize
        writer.finalize().unwrap();

        // Verify file was created
        let e01_path = temp_dir.path().join("test.E01");
        assert!(e01_path.exists());
        assert!(fs::metadata(&e01_path).unwrap().len() > 0);
    }

    #[test]
    fn test_e01_sector_size_mismatch() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("test");

        let config = E01WriterConfig {
            bytes_per_sector: 512,
            ..Default::default()
        };

        let mut writer = E01Writer::new(&output_path, config).unwrap();

        // Try to write wrong-sized sector
        let wrong_data = vec![0xAA; 256];
        let result = writer.write_sector(&wrong_data);
        assert!(result.is_err());
    }
}
