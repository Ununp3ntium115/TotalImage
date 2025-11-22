//! MBR partition types and CHS addressing

use std::fmt;

/// MBR partition type codes
///
/// These are the standard partition type identifiers used in the MBR partition table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MbrPartitionType {
    /// Empty/unused partition entry
    Empty = 0x00,
    /// FAT12, CHS
    Fat12 = 0x01,
    /// FAT16 < 32MB, CHS
    Fat16Small = 0x04,
    /// Extended partition, CHS
    Extended = 0x05,
    /// FAT16 >= 32MB, CHS
    Fat16 = 0x06,
    /// NTFS/exFAT/HPFS
    Ntfs = 0x07,
    /// FAT32, CHS
    Fat32Chs = 0x0B,
    /// FAT32, LBA
    Fat32Lba = 0x0C,
    /// FAT16, LBA
    Fat16Lba = 0x0E,
    /// Extended partition, LBA
    ExtendedLba = 0x0F,
    /// Linux swap
    LinuxSwap = 0x82,
    /// Linux native (ext2/ext3/ext4)
    LinuxNative = 0x83,
    /// GPT protective MBR
    GptProtective = 0xEE,
    /// EFI system partition
    EfiSystem = 0xEF,
    /// Unknown partition type
    Unknown(u8),
}

impl MbrPartitionType {
    /// Create a partition type from a byte value
    pub fn from_byte(b: u8) -> Self {
        match b {
            0x00 => Self::Empty,
            0x01 => Self::Fat12,
            0x04 => Self::Fat16Small,
            0x05 => Self::Extended,
            0x06 => Self::Fat16,
            0x07 => Self::Ntfs,
            0x0B => Self::Fat32Chs,
            0x0C => Self::Fat32Lba,
            0x0E => Self::Fat16Lba,
            0x0F => Self::ExtendedLba,
            0x82 => Self::LinuxSwap,
            0x83 => Self::LinuxNative,
            0xEE => Self::GptProtective,
            0xEF => Self::EfiSystem,
            _ => Self::Unknown(b),
        }
    }

    /// Get the byte value of this partition type
    pub fn to_byte(self) -> u8 {
        match self {
            Self::Unknown(b) => b,
            _ => self as u8,
        }
    }

    /// Get a human-readable name for this partition type
    pub fn name(&self) -> &str {
        match self {
            Self::Empty => "Empty",
            Self::Fat12 => "FAT12",
            Self::Fat16Small => "FAT16 (<32MB)",
            Self::Extended => "Extended",
            Self::Fat16 => "FAT16",
            Self::Ntfs => "NTFS/exFAT",
            Self::Fat32Chs => "FAT32 (CHS)",
            Self::Fat32Lba => "FAT32 (LBA)",
            Self::Fat16Lba => "FAT16 (LBA)",
            Self::ExtendedLba => "Extended (LBA)",
            Self::LinuxSwap => "Linux swap",
            Self::LinuxNative => "Linux",
            Self::GptProtective => "GPT Protective",
            Self::EfiSystem => "EFI System",
            Self::Unknown(b) => return "Unknown",
        }
    }
}

impl fmt::Display for MbrPartitionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// CHS (Cylinder-Head-Sector) address
///
/// Traditional disk addressing using physical geometry.
/// Maximum values: 1023 cylinders, 255 heads, 63 sectors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CHSAddress {
    pub cylinder: u16,
    pub head: u8,
    pub sector: u8,
}

impl CHSAddress {
    /// Parse CHS address from 3 bytes
    ///
    /// Format:
    /// - Byte 0: Head (0-255)
    /// - Byte 1: Sector (bits 0-5) + Cylinder high (bits 6-7)
    /// - Byte 2: Cylinder low (bits 0-7)
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let head = bytes[0];
        let sector = bytes[1] & 0x3F; // Lower 6 bits
        let cyl_high = ((bytes[1] & 0xC0) as u16) << 2; // Upper 2 bits
        let cyl_low = bytes[2] as u16;
        let cylinder = cyl_high | cyl_low;

        Self {
            cylinder,
            head,
            sector,
        }
    }

    /// Convert CHS to bytes
    pub fn to_bytes(&self) -> [u8; 3] {
        let cyl_high = ((self.cylinder >> 8) & 0x03) as u8;
        let cyl_low = (self.cylinder & 0xFF) as u8;

        [
            self.head,
            (self.sector & 0x3F) | (cyl_high << 6),
            cyl_low,
        ]
    }

    /// Convert CHS to LBA (approximate, requires disk geometry)
    pub fn to_lba(&self, heads_per_cylinder: u16, sectors_per_track: u16) -> u32 {
        let c = self.cylinder as u32;
        let h = self.head as u32;
        let s = (self.sector - 1) as u32; // Sectors are 1-indexed

        (c * heads_per_cylinder as u32 + h) * sectors_per_track as u32 + s
    }
}

impl fmt::Display for CHSAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "C:{}/H:{}/S:{}", self.cylinder, self.head, self.sector)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_partition_type_from_byte() {
        assert_eq!(MbrPartitionType::from_byte(0x00), MbrPartitionType::Empty);
        assert_eq!(MbrPartitionType::from_byte(0x0B), MbrPartitionType::Fat32Chs);
        assert_eq!(MbrPartitionType::from_byte(0x83), MbrPartitionType::LinuxNative);
        assert!(matches!(MbrPartitionType::from_byte(0xFF), MbrPartitionType::Unknown(0xFF)));
    }

    #[test]
    fn test_partition_type_to_byte() {
        assert_eq!(MbrPartitionType::Fat32Lba.to_byte(), 0x0C);
        assert_eq!(MbrPartitionType::Unknown(0x42).to_byte(), 0x42);
    }

    #[test]
    fn test_partition_type_name() {
        assert_eq!(MbrPartitionType::Fat32Lba.name(), "FAT32 (LBA)");
        assert_eq!(MbrPartitionType::LinuxNative.name(), "Linux");
    }

    #[test]
    fn test_chs_from_bytes() {
        // Example: C=0, H=1, S=1
        let bytes = [0x01, 0x01, 0x00];
        let chs = CHSAddress::from_bytes(&bytes);
        assert_eq!(chs.cylinder, 0);
        assert_eq!(chs.head, 1);
        assert_eq!(chs.sector, 1);
    }

    #[test]
    fn test_chs_to_bytes() {
        let chs = CHSAddress {
            cylinder: 100,
            head: 5,
            sector: 10,
        };
        let bytes = chs.to_bytes();
        let chs2 = CHSAddress::from_bytes(&bytes);
        assert_eq!(chs, chs2);
    }

    #[test]
    fn test_chs_to_lba() {
        let chs = CHSAddress {
            cylinder: 0,
            head: 1,
            sector: 1,
        };
        // With 16 heads and 63 sectors per track
        let lba = chs.to_lba(16, 63);
        assert_eq!(lba, 63); // (0 * 16 + 1) * 63 + 0 = 63
    }
}
