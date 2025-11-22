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

    /// Parse directory entry from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::ENTRY_SIZE {
            return None;
        }

        // Check for end of directory or deleted entry
        if bytes[0] == 0x00 || bytes[0] == 0xE5 {
            return None;
        }

        // Parse name (8 bytes + 3 extension)
        let name_bytes = &bytes[0..11];
        let name = Self::parse_name(name_bytes);

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

    /// Parse 8.3 filename from bytes
    fn parse_name(bytes: &[u8]) -> String {
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
