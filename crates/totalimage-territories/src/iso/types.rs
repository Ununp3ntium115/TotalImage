//! ISO-9660 file system types and structures

use std::fmt;

/// ISO-9660 sector size (2048 bytes)
pub const SECTOR_SIZE: usize = 2048;

/// Volume descriptors start at sector 16
pub const VOLUME_DESCRIPTOR_START: u64 = 16 * SECTOR_SIZE as u64;

/// Volume descriptor types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum VolumeDescriptorType {
    BootRecord = 0,
    PrimaryVolumeDescriptor = 1,
    SupplementaryVolumeDescriptor = 2,
    VolumePartitionDescriptor = 3,
    VolumeDescriptorSetTerminator = 255,
}

impl VolumeDescriptorType {
    /// Try to convert from a u8
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::BootRecord),
            1 => Some(Self::PrimaryVolumeDescriptor),
            2 => Some(Self::SupplementaryVolumeDescriptor),
            3 => Some(Self::VolumePartitionDescriptor),
            255 => Some(Self::VolumeDescriptorSetTerminator),
            _ => None,
        }
    }
}

/// Both-endian integer (ISO stores as both little and big endian)
#[derive(Debug, Clone, Copy)]
pub struct BothEndian<T> {
    pub little: T,
    pub big: T,
}

impl BothEndian<u16> {
    /// Parse from bytes (4 bytes: 2 little-endian + 2 big-endian)
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 4 {
            return None;
        }
        Some(Self {
            little: u16::from_le_bytes([bytes[0], bytes[1]]),
            big: u16::from_be_bytes([bytes[2], bytes[3]]),
        })
    }

    /// Get the value (prefer little-endian)
    pub fn get(&self) -> u16 {
        self.little
    }
}

impl BothEndian<u32> {
    /// Parse from bytes (8 bytes: 4 little-endian + 4 big-endian)
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 8 {
            return None;
        }
        Some(Self {
            little: u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            big: u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
        })
    }

    /// Get the value (prefer little-endian)
    pub fn get(&self) -> u32 {
        self.little
    }
}

/// ISO-9660 date/time format (7 bytes)
#[derive(Debug, Clone, Copy)]
pub struct IsoDateTime {
    pub year: u8,      // Years since 1900
    pub month: u8,     // 1-12
    pub day: u8,       // 1-31
    pub hour: u8,      // 0-23
    pub minute: u8,    // 0-59
    pub second: u8,    // 0-59
    pub gmt_offset: i8, // GMT offset in 15-minute intervals
}

impl IsoDateTime {
    /// Parse from 7 bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 7 {
            return None;
        }
        Some(Self {
            year: bytes[0],
            month: bytes[1],
            day: bytes[2],
            hour: bytes[3],
            minute: bytes[4],
            second: bytes[5],
            gmt_offset: bytes[6] as i8,
        })
    }
}

/// ISO-9660 ASCII date/time format (17 bytes)
#[derive(Debug, Clone)]
pub struct IsoAsciiDateTime {
    pub year: [u8; 4],     // YYYY
    pub month: [u8; 2],    // MM
    pub day: [u8; 2],      // DD
    pub hour: [u8; 2],     // HH
    pub minute: [u8; 2],   // MM
    pub second: [u8; 2],   // SS
    pub hundredths: [u8; 2], // Hundredths of second
    pub gmt_offset: i8,    // GMT offset in 15-minute intervals
}

impl IsoAsciiDateTime {
    /// Parse from 17 bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 17 {
            return None;
        }
        let mut year = [0u8; 4];
        let mut month = [0u8; 2];
        let mut day = [0u8; 2];
        let mut hour = [0u8; 2];
        let mut minute = [0u8; 2];
        let mut second = [0u8; 2];
        let mut hundredths = [0u8; 2];

        year.copy_from_slice(&bytes[0..4]);
        month.copy_from_slice(&bytes[4..6]);
        day.copy_from_slice(&bytes[6..8]);
        hour.copy_from_slice(&bytes[8..10]);
        minute.copy_from_slice(&bytes[10..12]);
        second.copy_from_slice(&bytes[12..14]);
        hundredths.copy_from_slice(&bytes[14..16]);

        Some(Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
            hundredths,
            gmt_offset: bytes[16] as i8,
        })
    }
}

/// Primary Volume Descriptor (sector 16 onwards)
#[derive(Debug, Clone)]
pub struct PrimaryVolumeDescriptor {
    pub descriptor_type: u8,
    pub identifier: [u8; 5],           // "CD001"
    pub version: u8,
    pub system_identifier: [u8; 32],
    pub volume_identifier: [u8; 32],
    pub volume_space_size: BothEndian<u32>,  // Total number of logical blocks
    pub volume_set_size: BothEndian<u16>,
    pub volume_sequence_number: BothEndian<u16>,
    pub logical_block_size: BothEndian<u16>, // Usually 2048
    pub path_table_size: BothEndian<u32>,
    pub l_path_table: u32,                   // Little-endian path table location
    pub m_path_table: u32,                   // Big-endian path table location
    pub root_directory_record: DirectoryRecord,
    pub volume_set_identifier: [u8; 128],
    pub publisher_identifier: [u8; 128],
    pub data_preparer_identifier: [u8; 128],
    pub application_identifier: [u8; 128],
    pub copyright_file_identifier: [u8; 37],
    pub abstract_file_identifier: [u8; 37],
    pub bibliographic_file_identifier: [u8; 37],
    pub volume_creation_date: IsoAsciiDateTime,
    pub volume_modification_date: IsoAsciiDateTime,
    pub volume_expiration_date: IsoAsciiDateTime,
    pub volume_effective_date: IsoAsciiDateTime,
    pub file_structure_version: u8,
}

impl PrimaryVolumeDescriptor {
    /// Parse from a 2048-byte sector
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < SECTOR_SIZE {
            return None;
        }

        let descriptor_type = bytes[0];
        let mut identifier = [0u8; 5];
        identifier.copy_from_slice(&bytes[1..6]);
        let version = bytes[6];

        // Check for valid ISO-9660 identifier
        if &identifier != b"CD001" {
            return None;
        }

        let mut system_identifier = [0u8; 32];
        system_identifier.copy_from_slice(&bytes[8..40]);

        let mut volume_identifier = [0u8; 32];
        volume_identifier.copy_from_slice(&bytes[40..72]);

        let volume_space_size = BothEndian::<u32>::from_bytes(&bytes[80..88])?;
        let volume_set_size = BothEndian::<u16>::from_bytes(&bytes[120..124])?;
        let volume_sequence_number = BothEndian::<u16>::from_bytes(&bytes[124..128])?;
        let logical_block_size = BothEndian::<u16>::from_bytes(&bytes[128..132])?;
        let path_table_size = BothEndian::<u32>::from_bytes(&bytes[132..140])?;

        let l_path_table = u32::from_le_bytes([bytes[140], bytes[141], bytes[142], bytes[143]]);
        let m_path_table = u32::from_be_bytes([bytes[151], bytes[152], bytes[153], bytes[154]]);

        // Parse root directory record (34 bytes at offset 156)
        let root_directory_record = DirectoryRecord::from_bytes(&bytes[156..190])?;

        let mut volume_set_identifier = [0u8; 128];
        volume_set_identifier.copy_from_slice(&bytes[190..318]);

        let mut publisher_identifier = [0u8; 128];
        publisher_identifier.copy_from_slice(&bytes[318..446]);

        let mut data_preparer_identifier = [0u8; 128];
        data_preparer_identifier.copy_from_slice(&bytes[446..574]);

        let mut application_identifier = [0u8; 128];
        application_identifier.copy_from_slice(&bytes[574..702]);

        let mut copyright_file_identifier = [0u8; 37];
        copyright_file_identifier.copy_from_slice(&bytes[702..739]);

        let mut abstract_file_identifier = [0u8; 37];
        abstract_file_identifier.copy_from_slice(&bytes[739..776]);

        let mut bibliographic_file_identifier = [0u8; 37];
        bibliographic_file_identifier.copy_from_slice(&bytes[776..813]);

        let volume_creation_date = IsoAsciiDateTime::from_bytes(&bytes[813..830])?;
        let volume_modification_date = IsoAsciiDateTime::from_bytes(&bytes[830..847])?;
        let volume_expiration_date = IsoAsciiDateTime::from_bytes(&bytes[847..864])?;
        let volume_effective_date = IsoAsciiDateTime::from_bytes(&bytes[864..881])?;

        let file_structure_version = bytes[881];

        Some(Self {
            descriptor_type,
            identifier,
            version,
            system_identifier,
            volume_identifier,
            volume_space_size,
            volume_set_size,
            volume_sequence_number,
            logical_block_size,
            path_table_size,
            l_path_table,
            m_path_table,
            root_directory_record,
            volume_set_identifier,
            publisher_identifier,
            data_preparer_identifier,
            application_identifier,
            copyright_file_identifier,
            abstract_file_identifier,
            bibliographic_file_identifier,
            volume_creation_date,
            volume_modification_date,
            volume_expiration_date,
            volume_effective_date,
            file_structure_version,
        })
    }

    /// Get the volume label as a trimmed string
    pub fn volume_label(&self) -> String {
        String::from_utf8_lossy(&self.volume_identifier)
            .trim()
            .to_string()
    }
}

/// Directory Record (variable length)
#[derive(Debug, Clone)]
pub struct DirectoryRecord {
    pub length: u8,                        // Length of this record
    pub extended_attr_length: u8,
    pub extent_location: BothEndian<u32>,  // LBA of file data
    pub data_length: BothEndian<u32>,      // Size of file in bytes
    pub recording_date: IsoDateTime,
    pub file_flags: u8,                    // Bit flags (hidden, directory, etc.)
    pub file_unit_size: u8,
    pub interleave_gap_size: u8,
    pub volume_sequence_number: BothEndian<u16>,
    pub file_identifier_length: u8,
    pub file_identifier: Vec<u8>,          // File name (variable length)
}

impl DirectoryRecord {
    /// File flag: Hidden
    pub const FLAG_HIDDEN: u8 = 0x01;
    /// File flag: Directory
    pub const FLAG_DIRECTORY: u8 = 0x02;
    /// File flag: Associated file
    pub const FLAG_ASSOCIATED: u8 = 0x04;
    /// File flag: Record format specified
    pub const FLAG_RECORD: u8 = 0x08;
    /// File flag: Protection attributes specified
    pub const FLAG_PROTECTION: u8 = 0x10;
    /// File flag: Not final directory record
    pub const FLAG_NOT_FINAL: u8 = 0x80;

    /// Parse from bytes (minimum length checks included)
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.is_empty() {
            return None;
        }

        let length = bytes[0];
        if length == 0 || bytes.len() < length as usize {
            return None;
        }

        // Minimum directory record is 34 bytes (33 + 1 for identifier)
        if length < 33 {
            return None;
        }

        let extended_attr_length = bytes[1];
        let extent_location = BothEndian::<u32>::from_bytes(&bytes[2..10])?;
        let data_length = BothEndian::<u32>::from_bytes(&bytes[10..18])?;
        let recording_date = IsoDateTime::from_bytes(&bytes[18..25])?;
        let file_flags = bytes[25];
        let file_unit_size = bytes[26];
        let interleave_gap_size = bytes[27];
        let volume_sequence_number = BothEndian::<u16>::from_bytes(&bytes[28..32])?;
        let file_identifier_length = bytes[32];

        // Extract file identifier
        let id_start = 33;
        let id_end = id_start + file_identifier_length as usize;
        if id_end > length as usize {
            return None;
        }

        let file_identifier = bytes[id_start..id_end].to_vec();

        Some(Self {
            length,
            extended_attr_length,
            extent_location,
            data_length,
            recording_date,
            file_flags,
            file_unit_size,
            interleave_gap_size,
            volume_sequence_number,
            file_identifier_length,
            file_identifier,
        })
    }

    /// Check if this is a directory
    pub fn is_directory(&self) -> bool {
        (self.file_flags & Self::FLAG_DIRECTORY) != 0
    }

    /// Check if this is hidden
    pub fn is_hidden(&self) -> bool {
        (self.file_flags & Self::FLAG_HIDDEN) != 0
    }

    /// Get the file name as a string
    pub fn file_name(&self) -> String {
        if self.file_identifier.is_empty() {
            return String::from(".");
        }

        // Special cases: 0x00 = current dir, 0x01 = parent dir
        if self.file_identifier.len() == 1 {
            match self.file_identifier[0] {
                0x00 => return String::from("."),
                0x01 => return String::from(".."),
                _ => {}
            }
        }

        // Parse ISO filename (may include version number like ";1")
        let name = String::from_utf8_lossy(&self.file_identifier).to_string();

        // Remove version number if present (e.g., "FILE.TXT;1" -> "FILE.TXT")
        if let Some(semicolon_pos) = name.find(';') {
            name[..semicolon_pos].to_string()
        } else {
            name
        }
    }
}

impl fmt::Display for DirectoryRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({} bytes at LBA {})",
            self.file_name(),
            self.data_length.get(),
            self.extent_location.get()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volume_descriptor_type() {
        assert_eq!(VolumeDescriptorType::from_u8(0), Some(VolumeDescriptorType::BootRecord));
        assert_eq!(VolumeDescriptorType::from_u8(1), Some(VolumeDescriptorType::PrimaryVolumeDescriptor));
        assert_eq!(VolumeDescriptorType::from_u8(255), Some(VolumeDescriptorType::VolumeDescriptorSetTerminator));
        assert_eq!(VolumeDescriptorType::from_u8(99), None);
    }

    #[test]
    fn test_both_endian_u16() {
        let bytes = [0x34, 0x12, 0x12, 0x34]; // LE: 0x1234, BE: 0x1234
        let both = BothEndian::<u16>::from_bytes(&bytes).unwrap();
        assert_eq!(both.little, 0x1234);
        assert_eq!(both.big, 0x1234);
        assert_eq!(both.get(), 0x1234);
    }

    #[test]
    fn test_both_endian_u32() {
        let bytes = [0x78, 0x56, 0x34, 0x12, 0x12, 0x34, 0x56, 0x78]; // LE: 0x12345678, BE: 0x12345678
        let both = BothEndian::<u32>::from_bytes(&bytes).unwrap();
        assert_eq!(both.little, 0x12345678);
        assert_eq!(both.big, 0x12345678);
        assert_eq!(both.get(), 0x12345678);
    }

    #[test]
    fn test_iso_datetime() {
        let bytes = [70, 1, 15, 12, 30, 45, 0]; // Year 1970, Jan 15, 12:30:45, GMT
        let dt = IsoDateTime::from_bytes(&bytes).unwrap();
        assert_eq!(dt.year, 70);
        assert_eq!(dt.month, 1);
        assert_eq!(dt.day, 15);
        assert_eq!(dt.hour, 12);
        assert_eq!(dt.minute, 30);
        assert_eq!(dt.second, 45);
    }

    #[test]
    fn test_directory_record_flags() {
        let mut bytes = vec![0u8; 34];
        bytes[0] = 34; // length
        bytes[25] = DirectoryRecord::FLAG_DIRECTORY;
        bytes[32] = 1; // identifier length
        bytes[33] = 0x00; // current directory

        let record = DirectoryRecord::from_bytes(&bytes).unwrap();
        assert!(record.is_directory());
        assert!(!record.is_hidden());
    }

    #[test]
    fn test_directory_record_filename() {
        let mut bytes = vec![0u8; 40];
        bytes[0] = 40; // length
        bytes[32] = 7; // identifier length
        bytes[33..40].copy_from_slice(b"TEST;1\x00");

        let record = DirectoryRecord::from_bytes(&bytes).unwrap();
        assert_eq!(record.file_name(), "TEST");
    }

    #[test]
    fn test_directory_record_special_names() {
        // Test "." (current directory)
        let mut bytes = vec![0u8; 34];
        bytes[0] = 34;
        bytes[32] = 1;
        bytes[33] = 0x00;

        let record = DirectoryRecord::from_bytes(&bytes).unwrap();
        assert_eq!(record.file_name(), ".");

        // Test ".." (parent directory)
        bytes[33] = 0x01;
        let record = DirectoryRecord::from_bytes(&bytes).unwrap();
        assert_eq!(record.file_name(), "..");
    }
}
