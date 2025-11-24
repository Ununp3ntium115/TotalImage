//! GPT partition types and structures

use std::fmt;

/// GPT partition type GUID
///
/// Well-known partition type GUIDs used in GPT partition tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartitionTypeGuid(pub [u8; 16]);

impl PartitionTypeGuid {
    /// Unused entry
    pub const UNUSED: Self = Self([0; 16]);

    /// EFI System Partition
    pub const EFI_SYSTEM: Self = Self([
        0x28, 0x73, 0x2a, 0xc1, 0x1f, 0xf8, 0xd2, 0x11,
        0xba, 0x4b, 0x00, 0xa0, 0xc9, 0x3e, 0xc9, 0x3b,
    ]);

    /// Microsoft Basic Data (FAT, NTFS, exFAT)
    pub const MICROSOFT_BASIC_DATA: Self = Self([
        0xa2, 0xa0, 0xd0, 0xeb, 0xe5, 0xb9, 0x33, 0x44,
        0x87, 0xc0, 0x68, 0xb6, 0xb7, 0x26, 0x99, 0xc7,
    ]);

    /// Linux filesystem
    pub const LINUX_FILESYSTEM: Self = Self([
        0xaf, 0x3d, 0xc6, 0x0f, 0x83, 0x84, 0x72, 0x47,
        0x8e, 0x79, 0x3d, 0x69, 0xd8, 0x47, 0x7d, 0xe4,
    ]);

    /// Linux swap
    pub const LINUX_SWAP: Self = Self([
        0x6d, 0xfd, 0x57, 0x06, 0xab, 0xa4, 0xc4, 0x43,
        0x84, 0xe5, 0x09, 0x33, 0xc8, 0x4b, 0x4f, 0x4f,
    ]);

    /// Get a human-readable name for this partition type
    pub fn name(&self) -> &str {
        match *self {
            Self::UNUSED => "Unused",
            Self::EFI_SYSTEM => "EFI System",
            Self::MICROSOFT_BASIC_DATA => "Microsoft Basic Data",
            Self::LINUX_FILESYSTEM => "Linux filesystem",
            Self::LINUX_SWAP => "Linux swap",
            _ => "Unknown",
        }
    }
}

impl fmt::Display for PartitionTypeGuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// GPT partition entry
///
/// Each partition entry is 128 bytes and describes one partition on the disk.
#[derive(Debug, Clone)]
pub struct GptPartitionEntry {
    /// Partition type GUID
    pub partition_type_guid: PartitionTypeGuid,
    /// Unique partition GUID
    pub unique_partition_guid: [u8; 16],
    /// First LBA (inclusive)
    pub first_lba: u64,
    /// Last LBA (inclusive)
    pub last_lba: u64,
    /// Attribute flags
    pub attributes: u64,
    /// Partition name (UTF-16LE, 72 bytes = 36 characters)
    pub name: String,
}

impl GptPartitionEntry {
    /// Size of a partition entry in bytes
    pub const ENTRY_SIZE: usize = 128;

    /// Parse a partition entry from bytes
    pub fn from_bytes(bytes: &[u8]) -> Self {
        assert!(bytes.len() >= Self::ENTRY_SIZE);

        // Parse partition type GUID (bytes 0-15)
        let mut partition_type_guid = [0u8; 16];
        partition_type_guid.copy_from_slice(&bytes[0..16]);
        let partition_type_guid = PartitionTypeGuid(partition_type_guid);

        // Parse unique partition GUID (bytes 16-31)
        let mut unique_partition_guid = [0u8; 16];
        unique_partition_guid.copy_from_slice(&bytes[16..32]);

        // Parse LBA values (bytes 32-47)
        let first_lba = u64::from_le_bytes([
            bytes[32], bytes[33], bytes[34], bytes[35],
            bytes[36], bytes[37], bytes[38], bytes[39],
        ]);
        let last_lba = u64::from_le_bytes([
            bytes[40], bytes[41], bytes[42], bytes[43],
            bytes[44], bytes[45], bytes[46], bytes[47],
        ]);

        // Parse attributes (bytes 48-55)
        let attributes = u64::from_le_bytes([
            bytes[48], bytes[49], bytes[50], bytes[51],
            bytes[52], bytes[53], bytes[54], bytes[55],
        ]);

        // Parse partition name (bytes 56-127, UTF-16LE)
        let name = Self::parse_name(&bytes[56..128]);

        Self {
            partition_type_guid,
            unique_partition_guid,
            first_lba,
            last_lba,
            attributes,
            name,
        }
    }

    /// Check if this entry is unused
    pub fn is_unused(&self) -> bool {
        self.partition_type_guid == PartitionTypeGuid::UNUSED
    }

    /// Get the size of this partition in LBA sectors
    pub fn size_lba(&self) -> u64 {
        if self.last_lba >= self.first_lba {
            self.last_lba - self.first_lba + 1
        } else {
            0
        }
    }

    /// Parse UTF-16LE partition name from bytes
    fn parse_name(bytes: &[u8]) -> String {
        // Convert bytes to u16 values (UTF-16LE)
        let mut utf16_chars = Vec::new();
        for i in (0..bytes.len()).step_by(2) {
            let char_code = u16::from_le_bytes([bytes[i], bytes[i + 1]]);
            if char_code == 0 {
                break; // Null terminator
            }
            utf16_chars.push(char_code);
        }

        String::from_utf16_lossy(&utf16_chars)
    }
}

/// GPT header
///
/// The GPT header contains metadata about the partition table.
#[derive(Debug, Clone)]
pub struct GptHeader {
    /// Header signature ("EFI PART")
    pub signature: [u8; 8],
    /// GPT revision (usually 0x00010000)
    pub revision: u32,
    /// Header size in bytes (usually 92)
    pub header_size: u32,
    /// CRC32 checksum of header
    pub header_crc32: u32,
    /// Reserved (must be zero)
    pub reserved: u32,
    /// Current LBA (location of this header)
    pub current_lba: u64,
    /// Backup LBA (location of backup header)
    pub backup_lba: u64,
    /// First usable LBA for partitions
    pub first_usable_lba: u64,
    /// Last usable LBA for partitions
    pub last_usable_lba: u64,
    /// Disk GUID
    pub disk_guid: [u8; 16],
    /// Starting LBA of partition entries
    pub partition_entries_lba: u64,
    /// Number of partition entries
    pub num_partition_entries: u32,
    /// Size of each partition entry
    pub partition_entry_size: u32,
    /// CRC32 of partition entries array
    pub partition_entries_crc32: u32,
}

impl GptHeader {
    /// GPT header signature
    pub const SIGNATURE: &'static [u8; 8] = b"EFI PART";

    /// Typical GPT header size
    pub const HEADER_SIZE: usize = 92;

    /// Parse GPT header from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::HEADER_SIZE {
            return None;
        }

        // Parse signature
        let mut signature = [0u8; 8];
        signature.copy_from_slice(&bytes[0..8]);

        // Verify signature
        if &signature != Self::SIGNATURE {
            return None;
        }

        // Parse revision (bytes 8-11)
        let revision = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);

        // Parse header size (bytes 12-15)
        let header_size = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);

        // Parse CRC32 (bytes 16-19)
        let header_crc32 = u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);

        // Parse reserved (bytes 20-23)
        let reserved = u32::from_le_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);

        // Parse LBA values
        let current_lba = u64::from_le_bytes([
            bytes[24], bytes[25], bytes[26], bytes[27],
            bytes[28], bytes[29], bytes[30], bytes[31],
        ]);
        let backup_lba = u64::from_le_bytes([
            bytes[32], bytes[33], bytes[34], bytes[35],
            bytes[36], bytes[37], bytes[38], bytes[39],
        ]);
        let first_usable_lba = u64::from_le_bytes([
            bytes[40], bytes[41], bytes[42], bytes[43],
            bytes[44], bytes[45], bytes[46], bytes[47],
        ]);
        let last_usable_lba = u64::from_le_bytes([
            bytes[48], bytes[49], bytes[50], bytes[51],
            bytes[52], bytes[53], bytes[54], bytes[55],
        ]);

        // Parse disk GUID (bytes 56-71)
        let mut disk_guid = [0u8; 16];
        disk_guid.copy_from_slice(&bytes[56..72]);

        // Parse partition entries info
        let partition_entries_lba = u64::from_le_bytes([
            bytes[72], bytes[73], bytes[74], bytes[75],
            bytes[76], bytes[77], bytes[78], bytes[79],
        ]);
        let num_partition_entries = u32::from_le_bytes([bytes[80], bytes[81], bytes[82], bytes[83]]);
        let partition_entry_size = u32::from_le_bytes([bytes[84], bytes[85], bytes[86], bytes[87]]);
        let partition_entries_crc32 = u32::from_le_bytes([bytes[88], bytes[89], bytes[90], bytes[91]]);

        Some(Self {
            signature,
            revision,
            header_size,
            header_crc32,
            reserved,
            current_lba,
            backup_lba,
            first_usable_lba,
            last_usable_lba,
            disk_guid,
            partition_entries_lba,
            num_partition_entries,
            partition_entry_size,
            partition_entries_crc32,
        })
    }

    /// Verify the header CRC32 checksum
    ///
    /// # Security
    /// Validates header integrity to detect corruption or tampering
    ///
    /// # Arguments
    /// * `header_bytes` - The raw header bytes (with CRC32 field zeroed for calculation)
    pub fn verify_header_crc32(&self, header_bytes: &[u8]) -> bool {
        if header_bytes.len() < self.header_size as usize {
            return false;
        }

        // Create a copy of header bytes with CRC32 field zeroed
        let mut header_for_crc = header_bytes[..self.header_size as usize].to_vec();

        // Zero out the CRC32 field (bytes 16-19)
        header_for_crc[16] = 0;
        header_for_crc[17] = 0;
        header_for_crc[18] = 0;
        header_for_crc[19] = 0;

        // Calculate CRC32
        let calculated_crc = crc32fast::hash(&header_for_crc);

        calculated_crc == self.header_crc32
    }

    /// Verify the partition entries array CRC32 checksum
    ///
    /// # Security
    /// Validates partition table integrity
    ///
    /// # Arguments
    /// * `partition_entries_bytes` - The raw partition entries array
    pub fn verify_partition_entries_crc32(&self, partition_entries_bytes: &[u8]) -> bool {
        let expected_size = self.num_partition_entries as usize * self.partition_entry_size as usize;

        if partition_entries_bytes.len() < expected_size {
            return false;
        }

        // Calculate CRC32 over the partition entries
        let calculated_crc = crc32fast::hash(&partition_entries_bytes[..expected_size]);

        calculated_crc == self.partition_entries_crc32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_partition_type_guid_names() {
        assert_eq!(PartitionTypeGuid::UNUSED.name(), "Unused");
        assert_eq!(PartitionTypeGuid::EFI_SYSTEM.name(), "EFI System");
        assert_eq!(PartitionTypeGuid::LINUX_FILESYSTEM.name(), "Linux filesystem");
    }

    #[test]
    fn test_partition_entry_is_unused() {
        let mut entry_bytes = vec![0u8; GptPartitionEntry::ENTRY_SIZE];
        let entry = GptPartitionEntry::from_bytes(&entry_bytes);
        assert!(entry.is_unused());

        // Set non-zero GUID
        entry_bytes[0] = 0x01;
        let entry = GptPartitionEntry::from_bytes(&entry_bytes);
        assert!(!entry.is_unused());
    }

    #[test]
    fn test_partition_entry_size_lba() {
        let mut entry_bytes = vec![0u8; GptPartitionEntry::ENTRY_SIZE];

        // Set first_lba = 100, last_lba = 199 (100 sectors)
        entry_bytes[32..40].copy_from_slice(&100u64.to_le_bytes());
        entry_bytes[40..48].copy_from_slice(&199u64.to_le_bytes());

        let entry = GptPartitionEntry::from_bytes(&entry_bytes);
        assert_eq!(entry.size_lba(), 100);
    }

    #[test]
    fn test_gpt_header_signature_validation() {
        let mut header_bytes = vec![0u8; GptHeader::HEADER_SIZE];

        // Invalid signature
        let result = GptHeader::from_bytes(&header_bytes);
        assert!(result.is_none());

        // Valid signature
        header_bytes[0..8].copy_from_slice(b"EFI PART");
        let result = GptHeader::from_bytes(&header_bytes);
        assert!(result.is_some());
    }
}
