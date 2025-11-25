//! exFAT type definitions
//!
//! This module contains the core data structures for parsing exFAT filesystems.

use totalimage_core::Result;

/// exFAT Boot Sector (512 bytes minimum)
#[derive(Debug, Clone)]
pub struct ExfatBootSector {
    /// JumpBoot (3 bytes) - Must be 0xEB7690
    pub jump_boot: [u8; 3],
    /// FileSystemName - Must be "EXFAT   "
    pub fs_name: [u8; 8],
    /// Partition offset in sectors from start of disk
    pub partition_offset: u64,
    /// Volume length in sectors
    pub volume_length: u64,
    /// FAT offset in sectors from start of partition
    pub fat_offset: u32,
    /// FAT length in sectors
    pub fat_length: u32,
    /// Cluster heap offset in sectors from start of partition
    pub cluster_heap_offset: u32,
    /// Total cluster count
    pub cluster_count: u32,
    /// First cluster of root directory
    pub root_dir_cluster: u32,
    /// Volume serial number
    pub volume_serial: u32,
    /// Filesystem revision (e.g., 0x0100 = 1.00)
    pub fs_revision: u16,
    /// Volume flags
    pub volume_flags: u16,
    /// Log2 of bytes per sector (e.g., 9 = 512 bytes)
    pub bytes_per_sector_shift: u8,
    /// Log2 of sectors per cluster
    pub sectors_per_cluster_shift: u8,
    /// Number of FATs (1 or 2)
    pub number_of_fats: u8,
    /// Drive select for INT 13h
    pub drive_select: u8,
    /// Percent of clusters in use
    pub percent_in_use: u8,
}

impl ExfatBootSector {
    /// exFAT filesystem name signature
    pub const FS_NAME: &'static [u8; 8] = b"EXFAT   ";

    /// Boot sector size
    pub const SIZE: usize = 512;

    /// Parse boot sector from bytes
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < Self::SIZE {
            return Err(totalimage_core::Error::invalid_territory(
                "exFAT boot sector too small",
            ));
        }

        // Verify jump boot
        let mut jump_boot = [0u8; 3];
        jump_boot.copy_from_slice(&bytes[0..3]);

        // Verify filesystem name
        let mut fs_name = [0u8; 8];
        fs_name.copy_from_slice(&bytes[3..11]);

        if &fs_name != Self::FS_NAME {
            return Err(totalimage_core::Error::invalid_territory(format!(
                "Invalid exFAT signature: expected 'EXFAT   ', got '{}'",
                String::from_utf8_lossy(&fs_name)
            )));
        }

        // Bytes 11-63 must be zero
        if !bytes[11..64].iter().all(|&b| b == 0) {
            return Err(totalimage_core::Error::invalid_territory(
                "exFAT MustBeZero region is not zero",
            ));
        }

        let partition_offset = u64::from_le_bytes([
            bytes[64], bytes[65], bytes[66], bytes[67],
            bytes[68], bytes[69], bytes[70], bytes[71],
        ]);

        let volume_length = u64::from_le_bytes([
            bytes[72], bytes[73], bytes[74], bytes[75],
            bytes[76], bytes[77], bytes[78], bytes[79],
        ]);

        let fat_offset = u32::from_le_bytes([bytes[80], bytes[81], bytes[82], bytes[83]]);
        let fat_length = u32::from_le_bytes([bytes[84], bytes[85], bytes[86], bytes[87]]);
        let cluster_heap_offset = u32::from_le_bytes([bytes[88], bytes[89], bytes[90], bytes[91]]);
        let cluster_count = u32::from_le_bytes([bytes[92], bytes[93], bytes[94], bytes[95]]);
        let root_dir_cluster = u32::from_le_bytes([bytes[96], bytes[97], bytes[98], bytes[99]]);
        let volume_serial = u32::from_le_bytes([bytes[100], bytes[101], bytes[102], bytes[103]]);
        let fs_revision = u16::from_le_bytes([bytes[104], bytes[105]]);
        let volume_flags = u16::from_le_bytes([bytes[106], bytes[107]]);
        let bytes_per_sector_shift = bytes[108];
        let sectors_per_cluster_shift = bytes[109];
        let number_of_fats = bytes[110];
        let drive_select = bytes[111];
        let percent_in_use = bytes[112];

        // Verify boot signature
        if bytes[510] != 0x55 || bytes[511] != 0xAA {
            return Err(totalimage_core::Error::invalid_territory(
                "Invalid exFAT boot signature",
            ));
        }

        Ok(Self {
            jump_boot,
            fs_name,
            partition_offset,
            volume_length,
            fat_offset,
            fat_length,
            cluster_heap_offset,
            cluster_count,
            root_dir_cluster,
            volume_serial,
            fs_revision,
            volume_flags,
            bytes_per_sector_shift,
            sectors_per_cluster_shift,
            number_of_fats,
            drive_select,
            percent_in_use,
        })
    }

    /// Get bytes per sector
    pub fn bytes_per_sector(&self) -> u32 {
        1 << self.bytes_per_sector_shift
    }

    /// Get sectors per cluster
    pub fn sectors_per_cluster(&self) -> u32 {
        1 << self.sectors_per_cluster_shift
    }

    /// Get bytes per cluster
    pub fn bytes_per_cluster(&self) -> u32 {
        self.bytes_per_sector() * self.sectors_per_cluster()
    }

    /// Check if volume is dirty
    pub fn is_dirty(&self) -> bool {
        (self.volume_flags & 0x02) != 0
    }

    /// Check if media failure occurred
    pub fn media_failure(&self) -> bool {
        (self.volume_flags & 0x04) != 0
    }
}

/// exFAT directory entry types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EntryType {
    /// End of directory marker (unused entry)
    EndOfDirectory = 0x00,
    /// Allocation bitmap entry
    AllocationBitmap = 0x81,
    /// Up-case table entry
    UpCaseTable = 0x82,
    /// Volume label entry
    VolumeLabel = 0x83,
    /// File directory entry
    FileEntry = 0x85,
    /// Volume GUID entry
    VolumeGuid = 0xA0,
    /// Stream extension entry
    StreamExtension = 0xC0,
    /// File name extension entry
    FileName = 0xC1,
    /// Deleted file entry
    DeletedFile = 0x05,
    /// Unknown entry type
    Unknown = 0xFF,
}

impl EntryType {
    /// Parse entry type from byte
    pub fn from_byte(byte: u8) -> Self {
        match byte {
            0x00 => EntryType::EndOfDirectory,
            0x81 => EntryType::AllocationBitmap,
            0x82 => EntryType::UpCaseTable,
            0x83 => EntryType::VolumeLabel,
            0x85 => EntryType::FileEntry,
            0xA0 => EntryType::VolumeGuid,
            0xC0 => EntryType::StreamExtension,
            0xC1 => EntryType::FileName,
            0x05 => EntryType::DeletedFile,
            _ if (byte & 0x80) == 0 => EntryType::EndOfDirectory, // Unused
            _ => EntryType::Unknown,
        }
    }

    /// Check if entry is in use
    pub fn is_in_use(&self) -> bool {
        match self {
            EntryType::EndOfDirectory | EntryType::DeletedFile | EntryType::Unknown => false,
            _ => true,
        }
    }
}

/// exFAT File Attributes
#[derive(Debug, Clone, Copy, Default)]
pub struct FileAttributes(pub u16);

impl FileAttributes {
    pub const READ_ONLY: u16 = 0x01;
    pub const HIDDEN: u16 = 0x02;
    pub const SYSTEM: u16 = 0x04;
    pub const DIRECTORY: u16 = 0x10;
    pub const ARCHIVE: u16 = 0x20;

    /// Create new attributes
    pub fn new(value: u16) -> Self {
        Self(value)
    }

    /// Check if read-only
    pub fn is_read_only(&self) -> bool {
        (self.0 & Self::READ_ONLY) != 0
    }

    /// Check if hidden
    pub fn is_hidden(&self) -> bool {
        (self.0 & Self::HIDDEN) != 0
    }

    /// Check if system
    pub fn is_system(&self) -> bool {
        (self.0 & Self::SYSTEM) != 0
    }

    /// Check if directory
    pub fn is_directory(&self) -> bool {
        (self.0 & Self::DIRECTORY) != 0
    }

    /// Check if archive
    pub fn is_archive(&self) -> bool {
        (self.0 & Self::ARCHIVE) != 0
    }
}

/// exFAT File Directory Entry (32 bytes)
#[derive(Debug, Clone)]
pub struct FileDirectoryEntry {
    /// Entry type (0x85)
    pub entry_type: u8,
    /// Secondary count (number of following entries)
    pub secondary_count: u8,
    /// Set checksum
    pub set_checksum: u16,
    /// File attributes
    pub attributes: FileAttributes,
    /// Reserved
    pub reserved1: u16,
    /// Creation timestamp
    pub create_timestamp: u32,
    /// Last modified timestamp
    pub modify_timestamp: u32,
    /// Last access timestamp
    pub access_timestamp: u32,
    /// Creation time 10ms increment
    pub create_10ms: u8,
    /// Modify time 10ms increment
    pub modify_10ms: u8,
    /// Create UTC offset
    pub create_utc_offset: u8,
    /// Modify UTC offset
    pub modify_utc_offset: u8,
    /// Access UTC offset
    pub access_utc_offset: u8,
    /// Reserved
    pub reserved2: [u8; 7],
}

impl FileDirectoryEntry {
    /// Entry size
    pub const SIZE: usize = 32;

    /// Parse from bytes
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < Self::SIZE {
            return Err(totalimage_core::Error::invalid_territory(
                "File directory entry too small",
            ));
        }

        Ok(Self {
            entry_type: bytes[0],
            secondary_count: bytes[1],
            set_checksum: u16::from_le_bytes([bytes[2], bytes[3]]),
            attributes: FileAttributes::new(u16::from_le_bytes([bytes[4], bytes[5]])),
            reserved1: u16::from_le_bytes([bytes[6], bytes[7]]),
            create_timestamp: u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
            modify_timestamp: u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
            access_timestamp: u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]),
            create_10ms: bytes[20],
            modify_10ms: bytes[21],
            create_utc_offset: bytes[22],
            modify_utc_offset: bytes[23],
            access_utc_offset: bytes[24],
            reserved2: [bytes[25], bytes[26], bytes[27], bytes[28], bytes[29], bytes[30], bytes[31]],
        })
    }

    /// Decode timestamp to (year, month, day, hour, minute, second)
    pub fn decode_timestamp(timestamp: u32) -> (u16, u8, u8, u8, u8, u8) {
        let second = ((timestamp & 0x1F) * 2) as u8;
        let minute = ((timestamp >> 5) & 0x3F) as u8;
        let hour = ((timestamp >> 11) & 0x1F) as u8;
        let day = ((timestamp >> 16) & 0x1F) as u8;
        let month = ((timestamp >> 21) & 0x0F) as u8;
        let year = 1980 + ((timestamp >> 25) & 0x7F) as u16;
        (year, month, day, hour, minute, second)
    }
}

/// exFAT Stream Extension Entry (32 bytes)
#[derive(Debug, Clone)]
pub struct StreamExtensionEntry {
    /// Entry type (0xC0)
    pub entry_type: u8,
    /// General secondary flags
    pub general_flags: u8,
    /// Reserved
    pub reserved1: u8,
    /// Name length in characters
    pub name_length: u8,
    /// Name hash
    pub name_hash: u16,
    /// Reserved
    pub reserved2: u16,
    /// Valid data length
    pub valid_data_length: u64,
    /// Reserved
    pub reserved3: u32,
    /// First cluster
    pub first_cluster: u32,
    /// Data length
    pub data_length: u64,
}

impl StreamExtensionEntry {
    /// Entry size
    pub const SIZE: usize = 32;

    /// Parse from bytes
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < Self::SIZE {
            return Err(totalimage_core::Error::invalid_territory(
                "Stream extension entry too small",
            ));
        }

        Ok(Self {
            entry_type: bytes[0],
            general_flags: bytes[1],
            reserved1: bytes[2],
            name_length: bytes[3],
            name_hash: u16::from_le_bytes([bytes[4], bytes[5]]),
            reserved2: u16::from_le_bytes([bytes[6], bytes[7]]),
            valid_data_length: u64::from_le_bytes([
                bytes[8], bytes[9], bytes[10], bytes[11],
                bytes[12], bytes[13], bytes[14], bytes[15],
            ]),
            reserved3: u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]),
            first_cluster: u32::from_le_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]),
            data_length: u64::from_le_bytes([
                bytes[24], bytes[25], bytes[26], bytes[27],
                bytes[28], bytes[29], bytes[30], bytes[31],
            ]),
        })
    }

    /// Check if allocation is contiguous (no fragmentation)
    pub fn is_contiguous(&self) -> bool {
        (self.general_flags & 0x02) != 0
    }

    /// Check if data allocation is possible (FAT valid)
    pub fn no_fat_chain(&self) -> bool {
        (self.general_flags & 0x02) != 0
    }
}

/// exFAT File Name Entry (32 bytes)
#[derive(Debug, Clone)]
pub struct FileNameEntry {
    /// Entry type (0xC1)
    pub entry_type: u8,
    /// General secondary flags
    pub general_flags: u8,
    /// File name characters (15 UTF-16LE characters)
    pub file_name: [u16; 15],
}

impl FileNameEntry {
    /// Entry size
    pub const SIZE: usize = 32;

    /// Parse from bytes
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < Self::SIZE {
            return Err(totalimage_core::Error::invalid_territory(
                "File name entry too small",
            ));
        }

        let mut file_name = [0u16; 15];
        for (i, chunk) in bytes[2..32].chunks(2).enumerate() {
            if i < 15 {
                file_name[i] = u16::from_le_bytes([chunk[0], chunk[1]]);
            }
        }

        Ok(Self {
            entry_type: bytes[0],
            general_flags: bytes[1],
            file_name,
        })
    }

    /// Get file name as string
    pub fn to_string(&self) -> String {
        String::from_utf16_lossy(
            &self.file_name.iter()
                .take_while(|&&c| c != 0)
                .cloned()
                .collect::<Vec<_>>()
        )
    }
}

/// exFAT Volume Label Entry (32 bytes)
#[derive(Debug, Clone)]
pub struct VolumeLabelEntry {
    /// Entry type (0x83)
    pub entry_type: u8,
    /// Character count
    pub character_count: u8,
    /// Volume label (11 UTF-16LE characters)
    pub volume_label: [u16; 11],
    /// Reserved
    pub reserved: [u8; 8],
}

impl VolumeLabelEntry {
    /// Entry size
    pub const SIZE: usize = 32;

    /// Parse from bytes
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < Self::SIZE {
            return Err(totalimage_core::Error::invalid_territory(
                "Volume label entry too small",
            ));
        }

        let mut volume_label = [0u16; 11];
        for (i, chunk) in bytes[2..24].chunks(2).enumerate() {
            if i < 11 {
                volume_label[i] = u16::from_le_bytes([chunk[0], chunk[1]]);
            }
        }

        let mut reserved = [0u8; 8];
        reserved.copy_from_slice(&bytes[24..32]);

        Ok(Self {
            entry_type: bytes[0],
            character_count: bytes[1],
            volume_label,
            reserved,
        })
    }

    /// Get volume label as string
    pub fn to_string(&self) -> String {
        let count = self.character_count.min(11) as usize;
        String::from_utf16_lossy(&self.volume_label[..count])
    }
}

/// Complete exFAT directory entry (file with name)
#[derive(Debug, Clone)]
pub struct ExfatDirectoryEntry {
    /// File name
    pub name: String,
    /// File attributes
    pub attributes: FileAttributes,
    /// File size in bytes
    pub size: u64,
    /// First cluster
    pub first_cluster: u32,
    /// Creation timestamp
    pub created: u32,
    /// Modified timestamp
    pub modified: u32,
    /// Accessed timestamp
    pub accessed: u32,
    /// Is contiguous allocation
    pub is_contiguous: bool,
}

impl ExfatDirectoryEntry {
    /// Check if entry is a directory
    pub fn is_directory(&self) -> bool {
        self.attributes.is_directory()
    }

    /// Check if entry is a regular file
    pub fn is_file(&self) -> bool {
        !self.is_directory()
    }
}

/// exFAT cluster chain entry values
pub mod cluster {
    /// Free cluster
    pub const FREE: u32 = 0x00000000;
    /// Bad cluster
    pub const BAD: u32 = 0xFFFFFFF7;
    /// End of chain
    pub const END_OF_CHAIN: u32 = 0xFFFFFFFF;

    /// Check if cluster is free
    pub fn is_free(value: u32) -> bool {
        value == FREE
    }

    /// Check if cluster is end of chain
    pub fn is_end(value: u32) -> bool {
        value >= 0xFFFFFFF8
    }

    /// Check if cluster is bad
    pub fn is_bad(value: u32) -> bool {
        value == BAD
    }

    /// Check if cluster points to next cluster
    pub fn is_valid(value: u32, cluster_count: u32) -> bool {
        value >= 2 && value < cluster_count + 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entry_type_parsing() {
        assert_eq!(EntryType::from_byte(0x00), EntryType::EndOfDirectory);
        assert_eq!(EntryType::from_byte(0x85), EntryType::FileEntry);
        assert_eq!(EntryType::from_byte(0xC0), EntryType::StreamExtension);
        assert_eq!(EntryType::from_byte(0xC1), EntryType::FileName);
        assert_eq!(EntryType::from_byte(0x83), EntryType::VolumeLabel);
    }

    #[test]
    fn test_file_attributes() {
        let attrs = FileAttributes::new(0x10);
        assert!(attrs.is_directory());
        assert!(!attrs.is_read_only());

        let attrs = FileAttributes::new(0x21);
        assert!(!attrs.is_directory());
        assert!(attrs.is_read_only());
        assert!(attrs.is_archive());
    }

    #[test]
    fn test_timestamp_decode() {
        // Test timestamp: 2023-06-15 14:30:00
        let timestamp = (43 << 25) | (6 << 21) | (15 << 16) | (14 << 11) | (30 << 5) | 0;
        let (year, month, day, hour, minute, second) = FileDirectoryEntry::decode_timestamp(timestamp);
        assert_eq!(year, 2023);
        assert_eq!(month, 6);
        assert_eq!(day, 15);
        assert_eq!(hour, 14);
        assert_eq!(minute, 30);
        assert_eq!(second, 0);
    }
}
