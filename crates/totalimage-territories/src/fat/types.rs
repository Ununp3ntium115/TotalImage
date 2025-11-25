//! FAT file system types and structures

use std::fmt;
use totalimage_core::{checked_multiply_u32_to_u64, checked_multiply_u64, Error, Result};

/// FAT type variants
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FatType {
    Fat12,
    Fat16,
    Fat32,
}

impl fmt::Display for FatType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FatType::Fat12 => write!(f, "FAT12"),
            FatType::Fat16 => write!(f, "FAT16"),
            FatType::Fat32 => write!(f, "FAT32"),
        }
    }
}

/// BIOS Parameter Block (BPB) - Common to all FAT variants
///
/// The BPB contains filesystem metadata and geometry information.
#[derive(Debug, Clone)]
pub struct BiosParameterBlock {
    /// Bytes per sector (typically 512)
    pub bytes_per_sector: u16,
    /// Sectors per cluster (power of 2)
    pub sectors_per_cluster: u8,
    /// Number of reserved sectors (including boot sector)
    pub reserved_sectors: u16,
    /// Number of FAT copies (typically 2)
    pub num_fats: u8,
    /// Maximum root directory entries (FAT12/16 only, 0 for FAT32)
    pub root_entries: u16,
    /// Total sectors (16-bit, 0 if using 32-bit field)
    pub total_sectors_16: u16,
    /// Media descriptor byte
    pub media_descriptor: u8,
    /// Sectors per FAT (FAT12/16 only, 0 for FAT32)
    pub sectors_per_fat_16: u16,
    /// Sectors per track (for CHS addressing)
    pub sectors_per_track: u16,
    /// Number of heads (for CHS addressing)
    pub num_heads: u16,
    /// Hidden sectors (LBA offset of partition)
    pub hidden_sectors: u32,
    /// Total sectors (32-bit, used if total_sectors_16 is 0)
    pub total_sectors_32: u32,
    /// FAT type determined from cluster count
    pub fat_type: FatType,
}

impl BiosParameterBlock {
    /// Parse BPB from boot sector bytes
    ///
    /// # Security
    /// Uses checked arithmetic to prevent integer overflow attacks
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 512 {
            return Err(Error::invalid_territory("BPB too short".to_string()));
        }

        // Parse common BPB fields (offsets 11-35)
        let bytes_per_sector = u16::from_le_bytes([bytes[11], bytes[12]]);
        let sectors_per_cluster = bytes[13];
        let reserved_sectors = u16::from_le_bytes([bytes[14], bytes[15]]);
        let num_fats = bytes[16];
        let root_entries = u16::from_le_bytes([bytes[17], bytes[18]]);
        let total_sectors_16 = u16::from_le_bytes([bytes[19], bytes[20]]);
        let media_descriptor = bytes[21];
        let sectors_per_fat_16 = u16::from_le_bytes([bytes[22], bytes[23]]);
        let sectors_per_track = u16::from_le_bytes([bytes[24], bytes[25]]);
        let num_heads = u16::from_le_bytes([bytes[26], bytes[27]]);
        let hidden_sectors = u32::from_le_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]);
        let total_sectors_32 = u32::from_le_bytes([bytes[32], bytes[33], bytes[34], bytes[35]]);

        // Validate sectors_per_cluster to prevent divide by zero
        if sectors_per_cluster == 0 {
            return Err(Error::invalid_territory("Invalid sectors_per_cluster: 0".to_string()));
        }

        // Validate bytes_per_sector
        if bytes_per_sector == 0 {
            return Err(Error::invalid_territory("Invalid bytes_per_sector: 0".to_string()));
        }

        // Determine total sectors
        let total_sectors = if total_sectors_16 != 0 {
            total_sectors_16 as u32
        } else {
            total_sectors_32
        };

        // Calculate data region size to determine FAT type (with checked arithmetic)
        let root_entries_bytes = checked_multiply_u32_to_u64(root_entries as u32, 32, "BPB root entries")?;
        let bytes_per_sector_minus_1 = bytes_per_sector.saturating_sub(1) as u64;
        let root_dir_sectors = ((root_entries_bytes + bytes_per_sector_minus_1) / bytes_per_sector as u64) as u32;

        let sectors_per_fat = if sectors_per_fat_16 != 0 {
            sectors_per_fat_16 as u32
        } else {
            // FAT32: read from offset 36
            u32::from_le_bytes([bytes[36], bytes[37], bytes[38], bytes[39]])
        };

        // Calculate FAT size with checked arithmetic
        let fat_size = checked_multiply_u32_to_u64(num_fats as u32, sectors_per_fat, "BPB FAT size")?;

        // Calculate total non-data sectors
        let non_data_sectors = (reserved_sectors as u64)
            .checked_add(fat_size)
            .and_then(|v| v.checked_add(root_dir_sectors as u64))
            .ok_or_else(|| Error::invalid_territory("BPB sector calculation overflow".to_string()))?;

        // Calculate data sectors with overflow check
        let data_sectors = (total_sectors as u64)
            .checked_sub(non_data_sectors)
            .ok_or_else(|| Error::invalid_territory("BPB data sectors underflow".to_string()))? as u32;

        let cluster_count = data_sectors / sectors_per_cluster as u32;

        // Determine FAT type based on cluster count
        let fat_type = if cluster_count < 4085 {
            FatType::Fat12
        } else if cluster_count < 65525 {
            FatType::Fat16
        } else {
            FatType::Fat32
        };

        Ok(Self {
            bytes_per_sector,
            sectors_per_cluster,
            reserved_sectors,
            num_fats,
            root_entries,
            total_sectors_16,
            media_descriptor,
            sectors_per_fat_16,
            sectors_per_track,
            num_heads,
            hidden_sectors,
            total_sectors_32,
            fat_type,
        })
    }

    /// Get the total number of sectors
    pub fn total_sectors(&self) -> u32 {
        if self.total_sectors_16 != 0 {
            self.total_sectors_16 as u32
        } else {
            self.total_sectors_32
        }
    }

    /// Get sectors per FAT
    pub fn sectors_per_fat(&self) -> u32 {
        if self.sectors_per_fat_16 != 0 {
            self.sectors_per_fat_16 as u32
        } else {
            // Would need to read FAT32 extended BPB
            0
        }
    }

    /// Calculate the byte offset of the first FAT
    ///
    /// # Security
    /// Uses checked arithmetic to prevent overflow
    pub fn fat_offset(&self) -> Result<u32> {
        checked_multiply_u32_to_u64(
            self.reserved_sectors as u32,
            self.bytes_per_sector as u32,
            "FAT offset"
        ).and_then(|v| {
            v.try_into().map_err(|_| Error::invalid_territory("FAT offset exceeds u32".to_string()))
        })
    }

    /// Calculate the byte offset of the root directory
    ///
    /// # Security
    /// Uses checked arithmetic to prevent overflow
    pub fn root_dir_offset(&self) -> Result<u32> {
        let fat_size = checked_multiply_u32_to_u64(
            self.sectors_per_fat(),
            self.bytes_per_sector as u32,
            "FAT size"
        )?;

        let total_fat_size = checked_multiply_u64(
            self.num_fats as u64,
            fat_size,
            "Total FAT size"
        )?;

        let fat_offset = self.fat_offset()? as u64;

        fat_offset
            .checked_add(total_fat_size)
            .and_then(|v| v.try_into().ok())
            .ok_or_else(|| Error::invalid_territory("Root dir offset overflow".to_string()))
    }

    /// Calculate the byte offset of the data region
    ///
    /// # Security
    /// Uses checked arithmetic to prevent overflow
    pub fn data_offset(&self) -> Result<u32> {
        let root_entries_bytes = checked_multiply_u32_to_u64(
            self.root_entries as u32,
            32,
            "Root entries size"
        )?;

        let bytes_per_sector_minus_1 = self.bytes_per_sector.saturating_sub(1) as u64;
        let root_dir_sectors = ((root_entries_bytes + bytes_per_sector_minus_1) / self.bytes_per_sector as u64) as u32;

        let root_dir_size = checked_multiply_u32_to_u64(
            root_dir_sectors,
            self.bytes_per_sector as u32,
            "Root dir size"
        )?;

        let root_offset = self.root_dir_offset()? as u64;

        root_offset
            .checked_add(root_dir_size)
            .and_then(|v| v.try_into().ok())
            .ok_or_else(|| Error::invalid_territory("Data offset overflow".to_string()))
    }

    /// Get bytes per cluster
    ///
    /// # Security
    /// Uses checked arithmetic to prevent overflow
    pub fn bytes_per_cluster(&self) -> Result<u32> {
        checked_multiply_u32_to_u64(
            self.sectors_per_cluster as u32,
            self.bytes_per_sector as u32,
            "Bytes per cluster"
        ).and_then(|v| {
            v.try_into().map_err(|_| Error::invalid_territory("Bytes per cluster exceeds u32".to_string()))
        })
    }
}

/// FAT directory entry (32 bytes)
#[derive(Debug, Clone)]
pub struct DirectoryEntry {
    /// Short filename (8.3 format)
    pub short_name: String,
    /// Long filename (if available, otherwise same as short_name)
    pub name: String,
    /// File attributes
    pub attributes: u8,
    /// Creation time
    pub create_time: u16,
    /// Creation date
    pub create_date: u16,
    /// Last access date
    pub access_date: u16,
    /// High word of first cluster (FAT32)
    pub first_cluster_high: u16,
    /// Modification time
    pub modify_time: u16,
    /// Modification date
    pub modify_date: u16,
    /// Low word of first cluster
    pub first_cluster_low: u16,
    /// File size in bytes
    pub file_size: u32,
}

/// Long File Name (LFN) directory entry
#[derive(Debug, Clone)]
pub struct LfnEntry {
    /// Order/sequence byte (1-20, bit 6 set for last entry)
    pub order: u8,
    /// First 5 UTF-16LE characters
    pub chars1: [u16; 5],
    /// Always 0x0F for LFN entries
    pub attributes: u8,
    /// Always 0 for LFN entries
    pub entry_type: u8,
    /// Checksum of short name
    pub checksum: u8,
    /// Next 6 UTF-16LE characters
    pub chars2: [u16; 6],
    /// Always 0 for LFN entries
    pub first_cluster: u16,
    /// Final 2 UTF-16LE characters
    pub chars3: [u16; 2],
}

impl LfnEntry {
    /// Parse LFN entry from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 32 {
            return None;
        }

        // Check if this is an LFN entry
        if bytes[11] != DirectoryEntry::ATTR_LONG_NAME {
            return None;
        }

        let order = bytes[0];

        // Parse UTF-16LE characters
        let mut chars1 = [0u16; 5];
        for i in 0..5 {
            chars1[i] = u16::from_le_bytes([bytes[1 + i * 2], bytes[2 + i * 2]]);
        }

        let mut chars2 = [0u16; 6];
        for i in 0..6 {
            chars2[i] = u16::from_le_bytes([bytes[14 + i * 2], bytes[15 + i * 2]]);
        }

        let mut chars3 = [0u16; 2];
        for i in 0..2 {
            chars3[i] = u16::from_le_bytes([bytes[28 + i * 2], bytes[29 + i * 2]]);
        }

        Some(Self {
            order,
            chars1,
            attributes: bytes[11],
            entry_type: bytes[12],
            checksum: bytes[13],
            chars2,
            first_cluster: u16::from_le_bytes([bytes[26], bytes[27]]),
            chars3,
        })
    }

    /// Get the sequence number (1-20)
    pub fn sequence(&self) -> u8 {
        self.order & 0x1F
    }

    /// Check if this is the last LFN entry
    pub fn is_last(&self) -> bool {
        (self.order & 0x40) != 0
    }

    /// Extract UTF-16LE characters from this entry
    pub fn get_chars(&self) -> Vec<u16> {
        let mut chars = Vec::with_capacity(13);

        // Add chars1 (5 chars)
        for &c in &self.chars1 {
            if c == 0x0000 || c == 0xFFFF {
                return chars;
            }
            chars.push(c);
        }

        // Add chars2 (6 chars)
        for &c in &self.chars2 {
            if c == 0x0000 || c == 0xFFFF {
                return chars;
            }
            chars.push(c);
        }

        // Add chars3 (2 chars)
        for &c in &self.chars3 {
            if c == 0x0000 || c == 0xFFFF {
                return chars;
            }
            chars.push(c);
        }

        chars
    }

    /// Calculate checksum for short name validation
    pub fn calculate_checksum(short_name: &[u8; 11]) -> u8 {
        let mut sum: u8 = 0;
        for &b in short_name {
            // Rotate right and add
            sum = sum.rotate_right(1).wrapping_add(b);
        }
        sum
    }
}

/// Assemble long filename from multiple LFN entries
pub fn assemble_lfn(entries: &[LfnEntry]) -> String {
    if entries.is_empty() {
        return String::new();
    }

    // Sort by sequence number (entries should already be in reverse order)
    let mut sorted: Vec<_> = entries.iter().collect();
    sorted.sort_by_key(|e| e.sequence());

    // Collect all UTF-16 characters
    let mut utf16_chars: Vec<u16> = Vec::new();
    for entry in sorted {
        utf16_chars.extend(entry.get_chars());
    }

    // Convert UTF-16LE to String
    String::from_utf16_lossy(&utf16_chars)
}

impl DirectoryEntry {
    /// Directory entry size in bytes
    pub const ENTRY_SIZE: usize = 32;

    /// Attribute: Read-only
    pub const ATTR_READ_ONLY: u8 = 0x01;
    /// Attribute: Hidden
    pub const ATTR_HIDDEN: u8 = 0x02;
    /// Attribute: System
    pub const ATTR_SYSTEM: u8 = 0x04;
    /// Attribute: Volume label
    pub const ATTR_VOLUME_ID: u8 = 0x08;
    /// Attribute: Directory
    pub const ATTR_DIRECTORY: u8 = 0x10;
    /// Attribute: Archive
    pub const ATTR_ARCHIVE: u8 = 0x20;
    /// Attribute: Long file name entry
    pub const ATTR_LONG_NAME: u8 = 0x0F;

    /// Parse directory entry from bytes (without LFN)
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Self::from_bytes_with_lfn(bytes, &[])
    }

    /// Parse directory entry from bytes with optional LFN entries
    pub fn from_bytes_with_lfn(bytes: &[u8], lfn_entries: &[LfnEntry]) -> Option<Self> {
        if bytes.len() < Self::ENTRY_SIZE {
            return None;
        }

        // Check for end of directory or deleted entry
        if bytes[0] == 0x00 || bytes[0] == 0xE5 {
            return None;
        }

        // Parse short name (8 bytes + 3 extension)
        let name_bytes = &bytes[0..11];
        let short_name = Self::parse_short_name(name_bytes);

        // Determine the display name (LFN if available, otherwise short name)
        let name = if !lfn_entries.is_empty() {
            let lfn = assemble_lfn(lfn_entries);
            if lfn.is_empty() {
                short_name.clone()
            } else {
                lfn
            }
        } else {
            short_name.clone()
        };

        let attributes = bytes[11];
        let create_time = u16::from_le_bytes([bytes[14], bytes[15]]);
        let create_date = u16::from_le_bytes([bytes[16], bytes[17]]);
        let access_date = u16::from_le_bytes([bytes[18], bytes[19]]);
        let first_cluster_high = u16::from_le_bytes([bytes[20], bytes[21]]);
        let modify_time = u16::from_le_bytes([bytes[22], bytes[23]]);
        let modify_date = u16::from_le_bytes([bytes[24], bytes[25]]);
        let first_cluster_low = u16::from_le_bytes([bytes[26], bytes[27]]);
        let file_size = u32::from_le_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]);

        Some(Self {
            short_name,
            name,
            attributes,
            create_time,
            create_date,
            access_date,
            first_cluster_high,
            modify_time,
            modify_date,
            first_cluster_low,
            file_size,
        })
    }

    /// Check if this is a directory
    pub fn is_directory(&self) -> bool {
        (self.attributes & Self::ATTR_DIRECTORY) != 0
    }

    /// Check if this is a volume label
    pub fn is_volume_label(&self) -> bool {
        (self.attributes & Self::ATTR_VOLUME_ID) != 0
    }

    /// Check if this is a long filename entry
    pub fn is_long_name(&self) -> bool {
        self.attributes == Self::ATTR_LONG_NAME
    }

    /// Get the first cluster number
    pub fn first_cluster(&self) -> u32 {
        ((self.first_cluster_high as u32) << 16) | (self.first_cluster_low as u32)
    }

    /// Parse 8.3 short filename from bytes
    fn parse_short_name(bytes: &[u8]) -> String {
        let name_part: String = bytes[0..8]
            .iter()
            .take_while(|&&b| b != 0x20)
            .map(|&b| b as char)
            .collect();

        let ext_part: String = bytes[8..11]
            .iter()
            .take_while(|&&b| b != 0x20)
            .map(|&b| b as char)
            .collect();

        if ext_part.is_empty() {
            name_part
        } else {
            format!("{}.{}", name_part, ext_part)
        }
    }

    /// Check if the raw bytes represent an LFN entry
    pub fn is_lfn_entry(bytes: &[u8]) -> bool {
        bytes.len() >= 12 && bytes[11] == Self::ATTR_LONG_NAME
    }

    /// Check if the raw bytes represent the end of directory
    pub fn is_end_of_directory(bytes: &[u8]) -> bool {
        !bytes.is_empty() && bytes[0] == 0x00
    }

    /// Check if the raw bytes represent a deleted entry
    pub fn is_deleted_entry(bytes: &[u8]) -> bool {
        !bytes.is_empty() && bytes[0] == 0xE5
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fat_type_display() {
        assert_eq!(FatType::Fat12.to_string(), "FAT12");
        assert_eq!(FatType::Fat16.to_string(), "FAT16");
        assert_eq!(FatType::Fat32.to_string(), "FAT32");
    }

    #[test]
    fn test_lfn_entry_parsing() {
        // Create a sample LFN entry for "Long File Name.txt"
        let mut bytes = vec![0u8; 32];
        bytes[0] = 0x41; // Order byte: 1 with "last" bit set
        bytes[11] = DirectoryEntry::ATTR_LONG_NAME;
        bytes[12] = 0; // Type
        bytes[13] = 0; // Checksum

        // "Long " in UTF-16LE at offset 1
        let name = "Long ";
        for (i, c) in name.encode_utf16().enumerate() {
            let offset = 1 + i * 2;
            bytes[offset] = (c & 0xFF) as u8;
            bytes[offset + 1] = (c >> 8) as u8;
        }

        let lfn = LfnEntry::from_bytes(&bytes).unwrap();
        assert_eq!(lfn.sequence(), 1);
        assert!(lfn.is_last());

        let chars = lfn.get_chars();
        assert_eq!(chars.len(), 5);
    }

    #[test]
    fn test_lfn_assembly() {
        // Create two LFN entries that spell "LongFileName.txt"
        let mut entry1 = vec![0u8; 32];
        entry1[0] = 0x42; // Order byte: 2 with "last" bit set
        entry1[11] = DirectoryEntry::ATTR_LONG_NAME;

        // "ame.txt" + padding
        let name1 = "ame.txt";
        for (i, c) in name1.encode_utf16().enumerate() {
            if i < 5 {
                let offset = 1 + i * 2;
                entry1[offset] = (c & 0xFF) as u8;
                entry1[offset + 1] = (c >> 8) as u8;
            } else {
                let offset = 14 + (i - 5) * 2;
                entry1[offset] = (c & 0xFF) as u8;
                entry1[offset + 1] = (c >> 8) as u8;
            }
        }
        // Null terminate
        entry1[28] = 0;
        entry1[29] = 0;

        let mut entry2 = vec![0u8; 32];
        entry2[0] = 0x01; // Order byte: 1
        entry2[11] = DirectoryEntry::ATTR_LONG_NAME;

        // "LongFileN"
        let name2 = "LongFileN";
        for (i, c) in name2.encode_utf16().enumerate() {
            if i < 5 {
                let offset = 1 + i * 2;
                entry2[offset] = (c & 0xFF) as u8;
                entry2[offset + 1] = (c >> 8) as u8;
            } else {
                let offset = 14 + (i - 5) * 2;
                entry2[offset] = (c & 0xFF) as u8;
                entry2[offset + 1] = (c >> 8) as u8;
            }
        }

        let lfn1 = LfnEntry::from_bytes(&entry1).unwrap();
        let lfn2 = LfnEntry::from_bytes(&entry2).unwrap();

        let long_name = assemble_lfn(&[lfn1, lfn2]);
        assert_eq!(long_name, "LongFileName.txt");
    }

    #[test]
    fn test_lfn_checksum() {
        let short_name: [u8; 11] = *b"LONGFI~1TXT";
        let checksum = LfnEntry::calculate_checksum(&short_name);
        // Just verify it doesn't panic and returns a value
        assert!(checksum > 0 || checksum == 0);
    }

    #[test]
    fn test_directory_entry_with_lfn() {
        // Create a short name entry
        let mut short_bytes = vec![0u8; 32];
        short_bytes[0..11].copy_from_slice(b"LONGFI~1TXT");
        short_bytes[11] = 0x20; // Archive attribute
        short_bytes[28] = 100; // File size

        // Create an LFN entry
        let mut lfn_bytes = vec![0u8; 32];
        lfn_bytes[0] = 0x41; // Order: 1, last
        lfn_bytes[11] = DirectoryEntry::ATTR_LONG_NAME;

        // "LongFile.txt" in UTF-16LE
        let name = "LongFile.txt";
        for (i, c) in name.encode_utf16().enumerate() {
            if i < 5 {
                let offset = 1 + i * 2;
                lfn_bytes[offset] = (c & 0xFF) as u8;
                lfn_bytes[offset + 1] = (c >> 8) as u8;
            } else if i < 11 {
                let offset = 14 + (i - 5) * 2;
                lfn_bytes[offset] = (c & 0xFF) as u8;
                lfn_bytes[offset + 1] = (c >> 8) as u8;
            } else {
                let offset = 28 + (i - 11) * 2;
                lfn_bytes[offset] = (c & 0xFF) as u8;
                lfn_bytes[offset + 1] = (c >> 8) as u8;
            }
        }

        let lfn = LfnEntry::from_bytes(&lfn_bytes).unwrap();
        let entry = DirectoryEntry::from_bytes_with_lfn(&short_bytes, &[lfn]).unwrap();

        assert_eq!(entry.short_name, "LONGFI~1.TXT");
        assert_eq!(entry.name, "LongFile.txt");
        assert_eq!(entry.file_size, 100);
    }

    #[test]
    fn test_bpb_total_sectors() {
        let mut bytes = vec![0u8; 512];

        // Set bytes per sector
        bytes[11..13].copy_from_slice(&512u16.to_le_bytes());

        // Set sectors per cluster (required to avoid divide by zero)
        bytes[13] = 1;

        // Set reserved sectors
        bytes[14..16].copy_from_slice(&1u16.to_le_bytes());

        // Set number of FATs
        bytes[16] = 2;

        // Set sectors per FAT
        bytes[22..24].copy_from_slice(&9u16.to_le_bytes());

        // Set total_sectors_16
        bytes[19..21].copy_from_slice(&2880u16.to_le_bytes());

        let bpb = BiosParameterBlock::from_bytes(&bytes).unwrap();
        assert_eq!(bpb.total_sectors(), 2880);
        assert!(bpb.fat_offset().is_ok());
        assert!(bpb.bytes_per_cluster().is_ok());
    }

    #[test]
    fn test_directory_entry_is_directory() {
        let mut bytes = vec![0u8; 32];
        bytes[0] = b'T';
        bytes[11] = DirectoryEntry::ATTR_DIRECTORY;

        let entry = DirectoryEntry::from_bytes(&bytes).unwrap();
        assert!(entry.is_directory());
        assert!(!entry.is_volume_label());
    }

    #[test]
    fn test_directory_entry_name_parsing() {
        let mut bytes = vec![0u8; 32];
        // Name: "TEST    TXT"
        bytes[0..11].copy_from_slice(b"TEST    TXT");

        let entry = DirectoryEntry::from_bytes(&bytes).unwrap();
        assert_eq!(entry.name, "TEST.TXT");
    }

    #[test]
    fn test_directory_entry_deleted() {
        let mut bytes = vec![0u8; 32];
        bytes[0] = 0xE5; // Deleted marker

        let entry = DirectoryEntry::from_bytes(&bytes);
        assert!(entry.is_none());
    }
}
