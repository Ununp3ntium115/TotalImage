//! E01 (EnCase) format types
//!
//! The E01 format is a forensic disk image format created by Guidance Software.
//! It uses segment files (.E01, .E02, etc.) and supports compression and hashing.

use totalimage_core::{Error, Result};

/// E01 signature bytes ("EVF" and version)
pub const EVF_SIGNATURE: [u8; 8] = [0x45, 0x56, 0x46, 0x09, 0x0D, 0x0A, 0xFF, 0x00];

/// Legacy EWF signature (EnCase 1-6)
pub const EWF_SIGNATURE: [u8; 8] = [0x45, 0x56, 0x46, 0x09, 0x0D, 0x0A, 0x00, 0x00];

/// Section type identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionType {
    /// Header section with case info
    Header,
    /// Volume section with media info
    Volume,
    /// Disk section (alternative volume)
    Disk,
    /// Sectors section with compressed data
    Sectors,
    /// Table section with chunk offsets
    Table,
    /// Table2 section (alternative table)
    Table2,
    /// Hash section with verification hashes
    Hash,
    /// Done section (end of segment)
    Done,
    /// Next section (continue to next segment)
    Next,
    /// Data section (uncompressed)
    Data,
    /// Unknown section type
    Unknown(u16),
}

impl SectionType {
    /// Parse section type from 16-byte type field
    pub fn from_bytes(bytes: &[u8; 16]) -> Self {
        // Section types are ASCII strings, null-padded
        let type_str = std::str::from_utf8(&bytes[..])
            .unwrap_or("")
            .trim_end_matches('\0');

        match type_str {
            "header" => Self::Header,
            "header2" => Self::Header,
            "volume" => Self::Volume,
            "disk" => Self::Disk,
            "sectors" => Self::Sectors,
            "table" => Self::Table,
            "table2" => Self::Table2,
            "hash" => Self::Hash,
            "done" => Self::Done,
            "next" => Self::Next,
            "data" => Self::Data,
            _ => Self::Unknown(0),
        }
    }

    /// Convert to bytes for writing
    pub fn to_bytes(&self) -> [u8; 16] {
        let mut bytes = [0u8; 16];
        let s = match self {
            Self::Header => "header",
            Self::Volume => "volume",
            Self::Disk => "disk",
            Self::Sectors => "sectors",
            Self::Table => "table",
            Self::Table2 => "table2",
            Self::Hash => "hash",
            Self::Done => "done",
            Self::Next => "next",
            Self::Data => "data",
            Self::Unknown(_) => "unknown",
        };
        bytes[..s.len()].copy_from_slice(s.as_bytes());
        bytes
    }
}

/// E01 file header (13 bytes)
#[derive(Debug, Clone)]
pub struct E01FileHeader {
    /// Signature (EVF or EWF)
    pub signature: [u8; 8],
    /// Segment number (1-based)
    pub segment_number: u16,
    /// Fields start offset
    pub fields_start: u16,
}

impl E01FileHeader {
    /// Size of the file header
    pub const SIZE: usize = 13;

    /// Parse file header from bytes
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < Self::SIZE {
            return Err(Error::invalid_vault("E01 header too short"));
        }

        let mut signature = [0u8; 8];
        signature.copy_from_slice(&data[0..8]);

        // Validate signature
        if signature != EVF_SIGNATURE && signature != EWF_SIGNATURE {
            // Check for partial match (at least EVF prefix)
            if &signature[0..3] != b"EVF" {
                return Err(Error::invalid_vault("Invalid E01 signature"));
            }
        }

        let segment_number = u16::from_le_bytes([data[9], data[10]]);
        let fields_start = u16::from_le_bytes([data[11], data[12]]);

        Ok(Self {
            signature,
            segment_number,
            fields_start,
        })
    }

    /// Check if this is a modern EVF format
    pub fn is_evf(&self) -> bool {
        self.signature == EVF_SIGNATURE
    }
}

/// E01 section descriptor (76 bytes)
#[derive(Debug, Clone)]
pub struct E01SectionDescriptor {
    /// Section type
    pub section_type: SectionType,
    /// Next section offset (absolute)
    pub next_offset: u64,
    /// Section size (including header)
    pub section_size: u64,
    /// Padding/reserved
    pub padding: [u8; 40],
    /// Checksum (Adler-32)
    pub checksum: u32,
}

impl E01SectionDescriptor {
    /// Size of section descriptor
    pub const SIZE: usize = 76;

    /// Parse section descriptor from bytes
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < Self::SIZE {
            return Err(Error::invalid_vault("E01 section descriptor too short"));
        }

        let mut type_bytes = [0u8; 16];
        type_bytes.copy_from_slice(&data[0..16]);
        let section_type = SectionType::from_bytes(&type_bytes);

        let next_offset = u64::from_le_bytes([
            data[16], data[17], data[18], data[19],
            data[20], data[21], data[22], data[23],
        ]);

        let section_size = u64::from_le_bytes([
            data[24], data[25], data[26], data[27],
            data[28], data[29], data[30], data[31],
        ]);

        let mut padding = [0u8; 40];
        padding.copy_from_slice(&data[32..72]);

        let checksum = u32::from_le_bytes([data[72], data[73], data[74], data[75]]);

        Ok(Self {
            section_type,
            next_offset,
            section_size,
            padding,
            checksum,
        })
    }
}

/// E01 volume section data
#[derive(Debug, Clone)]
pub struct E01VolumeSection {
    /// Media type
    pub media_type: u8,
    /// Chunk count
    pub chunk_count: u32,
    /// Sectors per chunk
    pub sectors_per_chunk: u32,
    /// Bytes per sector
    pub bytes_per_sector: u32,
    /// Sector count
    pub sector_count: u64,
    /// Compression method (0=none, 1=deflate)
    pub compression: u8,
}

impl E01VolumeSection {
    /// Parse volume section from bytes
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 94 {
            return Err(Error::invalid_vault("E01 volume section too short"));
        }

        // Skip reserved bytes at start
        let media_type = data[0];

        let chunk_count = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        let sectors_per_chunk = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
        let bytes_per_sector = u32::from_le_bytes([data[12], data[13], data[14], data[15]]);
        let sector_count = u64::from_le_bytes([
            data[16], data[17], data[18], data[19],
            data[20], data[21], data[22], data[23],
        ]);

        // Compression is at offset 88
        let compression = if data.len() > 88 { data[88] } else { 1 };

        Ok(Self {
            media_type,
            chunk_count,
            sectors_per_chunk,
            bytes_per_sector,
            sector_count,
            compression,
        })
    }

    /// Get total media size in bytes
    pub fn media_size(&self) -> u64 {
        self.sector_count * self.bytes_per_sector as u64
    }

    /// Get chunk size in bytes
    pub fn chunk_size(&self) -> u32 {
        self.sectors_per_chunk * self.bytes_per_sector
    }
}

/// E01 table entry (chunk offset)
#[derive(Debug, Clone, Copy)]
pub struct E01TableEntry {
    /// Offset to chunk data (compressed)
    pub offset: u64,
    /// Size of chunk data
    pub size: u32,
}

/// E01 hash section data
#[derive(Debug, Clone)]
pub struct E01HashSection {
    /// MD5 hash of uncompressed data
    pub md5_hash: [u8; 16],
    /// Checksum (CRC32 or Adler-32)
    pub checksum: u32,
}

impl E01HashSection {
    /// Parse hash section from bytes
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 20 {
            return Err(Error::invalid_vault("E01 hash section too short"));
        }

        let mut md5_hash = [0u8; 16];
        md5_hash.copy_from_slice(&data[0..16]);

        let checksum = u32::from_le_bytes([data[16], data[17], data[18], data[19]]);

        Ok(Self { md5_hash, checksum })
    }

    /// Get MD5 hash as hex string
    pub fn md5_hex(&self) -> String {
        self.md5_hash
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect()
    }
}

/// Compression method for E01 chunks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum E01Compression {
    /// No compression
    None,
    /// Deflate (zlib) compression
    Deflate,
    /// Unknown compression
    Unknown(u8),
}

impl From<u8> for E01Compression {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::None,
            1 => Self::Deflate,
            v => Self::Unknown(v),
        }
    }
}

/// Media type for E01 images
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum E01MediaType {
    /// Removable media (floppy, USB, etc.)
    Removable,
    /// Fixed disk (hard drive)
    Fixed,
    /// Optical media (CD, DVD)
    Optical,
    /// Logical volume
    Logical,
    /// Memory (RAM)
    Memory,
    /// Unknown media type
    Unknown(u8),
}

impl From<u8> for E01MediaType {
    fn from(value: u8) -> Self {
        match value {
            0x00 => Self::Removable,
            0x01 => Self::Fixed,
            0x03 => Self::Optical,
            0x0E => Self::Logical,
            0x10 => Self::Memory,
            v => Self::Unknown(v),
        }
    }
}

impl std::fmt::Display for E01MediaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Removable => write!(f, "Removable"),
            Self::Fixed => write!(f, "Fixed Disk"),
            Self::Optical => write!(f, "Optical"),
            Self::Logical => write!(f, "Logical Volume"),
            Self::Memory => write!(f, "Memory"),
            Self::Unknown(v) => write!(f, "Unknown (0x{:02X})", v),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_section_type_from_bytes() {
        let mut bytes = [0u8; 16];
        bytes[..6].copy_from_slice(b"header");
        assert_eq!(SectionType::from_bytes(&bytes), SectionType::Header);

        bytes = [0u8; 16];
        bytes[..7].copy_from_slice(b"sectors");
        assert_eq!(SectionType::from_bytes(&bytes), SectionType::Sectors);

        bytes = [0u8; 16];
        bytes[..4].copy_from_slice(b"done");
        assert_eq!(SectionType::from_bytes(&bytes), SectionType::Done);
    }

    #[test]
    fn test_section_type_roundtrip() {
        let types = [
            SectionType::Header,
            SectionType::Volume,
            SectionType::Sectors,
            SectionType::Table,
            SectionType::Hash,
            SectionType::Done,
        ];

        for typ in types {
            let bytes = typ.to_bytes();
            let parsed = SectionType::from_bytes(&bytes);
            assert_eq!(parsed, typ);
        }
    }

    #[test]
    fn test_e01_file_header_parse() {
        let mut data = vec![0u8; 13];
        data[0..8].copy_from_slice(&EVF_SIGNATURE);
        data[9] = 1; // segment 1
        data[10] = 0;
        data[11] = 13; // fields start at offset 13
        data[12] = 0;

        let header = E01FileHeader::parse(&data).unwrap();
        assert_eq!(header.signature, EVF_SIGNATURE);
        assert_eq!(header.segment_number, 1);
        assert_eq!(header.fields_start, 13);
        assert!(header.is_evf());
    }

    #[test]
    fn test_e01_hash_hex() {
        let hash = E01HashSection {
            md5_hash: [
                0xd4, 0x1d, 0x8c, 0xd9, 0x8f, 0x00, 0xb2, 0x04,
                0xe9, 0x80, 0x09, 0x98, 0xec, 0xf8, 0x42, 0x7e,
            ],
            checksum: 0,
        };

        assert_eq!(hash.md5_hex(), "d41d8cd98f00b204e9800998ecf8427e");
    }

    #[test]
    fn test_volume_section_calculations() {
        let volume = E01VolumeSection {
            media_type: 0x01,
            chunk_count: 100,
            sectors_per_chunk: 64,
            bytes_per_sector: 512,
            sector_count: 6400,
            compression: 1,
        };

        assert_eq!(volume.chunk_size(), 32768); // 64 * 512
        assert_eq!(volume.media_size(), 3_276_800); // 6400 * 512
    }

    #[test]
    fn test_media_type_display() {
        assert_eq!(E01MediaType::Fixed.to_string(), "Fixed Disk");
        assert_eq!(E01MediaType::Removable.to_string(), "Removable");
        assert_eq!(E01MediaType::Optical.to_string(), "Optical");
    }
}
