//! MBR (Master Boot Record) partition table implementation

pub mod types;

use std::io::SeekFrom;
use totalimage_core::{Error, ReadSeek, Result, Zone, ZoneTable};
use types::{CHSAddress, MbrPartitionType};

/// MBR partition table
///
/// The Master Boot Record is the traditional partitioning scheme used by BIOS-based systems.
/// It supports up to 4 primary partitions, or 3 primary partitions and 1 extended partition.
///
/// # Structure
///
/// ```text
/// Offset  Size  Field
/// ------  ----  -----
/// 0x000   446   Bootstrap code
/// 0x1BE   16    Partition entry 1
/// 0x1CE   16    Partition entry 2
/// 0x1DE   16    Partition entry 3
/// 0x1EE   16    Partition entry 4
/// 0x1FE   2     Boot signature (0xAA55)
/// ```
#[derive(Debug, Clone)]
pub struct MbrZoneTable {
    zones: Vec<Zone>,
    disk_signature: u32,
    boot_signature: u16,
}

impl MbrZoneTable {
    /// The boot signature that must be present at offset 0x1FE
    pub const BOOT_SIGNATURE: u16 = 0xAA55;

    /// Size of the MBR in bytes (always 512)
    pub const MBR_SIZE: usize = 512;

    /// Offset of the first partition entry
    pub const PARTITION_TABLE_OFFSET: u64 = 0x1BE;

    /// Offset of the disk signature
    pub const DISK_SIGNATURE_OFFSET: u64 = 0x1B8;

    /// Offset of the boot signature
    pub const BOOT_SIGNATURE_OFFSET: u64 = 0x1FE;

    /// Size of each partition entry
    pub const PARTITION_ENTRY_SIZE: usize = 16;

    /// Number of partition entries in MBR
    pub const NUM_PARTITIONS: usize = 4;

    /// Parse an MBR from a readable and seekable stream
    ///
    /// # Arguments
    ///
    /// * `stream` - A stream positioned at the start of the disk
    /// * `sector_size` - The sector size in bytes (usually 512)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The boot signature is invalid
    /// - The stream cannot be read
    /// - The partition table is corrupted
    pub fn parse(stream: &mut dyn ReadSeek, sector_size: u32) -> Result<Self> {
        // Read entire MBR sector
        stream.seek(SeekFrom::Start(0))?;
        let mut mbr = [0u8; Self::MBR_SIZE];
        stream.read_exact(&mut mbr)?;

        // Verify boot signature
        let boot_signature = u16::from_le_bytes([
            mbr[Self::BOOT_SIGNATURE_OFFSET as usize],
            mbr[Self::BOOT_SIGNATURE_OFFSET as usize + 1],
        ]);

        if boot_signature != Self::BOOT_SIGNATURE {
            return Err(Error::invalid_zone_table(format!(
                "Invalid MBR boot signature: expected 0x{:04X}, got 0x{:04X}",
                Self::BOOT_SIGNATURE,
                boot_signature
            )));
        }

        // Read disk signature
        let disk_signature = u32::from_le_bytes([
            mbr[Self::DISK_SIGNATURE_OFFSET as usize],
            mbr[Self::DISK_SIGNATURE_OFFSET as usize + 1],
            mbr[Self::DISK_SIGNATURE_OFFSET as usize + 2],
            mbr[Self::DISK_SIGNATURE_OFFSET as usize + 3],
        ]);

        // Parse partition entries
        let mut zones = Vec::new();

        for i in 0..Self::NUM_PARTITIONS {
            let offset = Self::PARTITION_TABLE_OFFSET as usize + (i * Self::PARTITION_ENTRY_SIZE);
            let entry = &mbr[offset..offset + Self::PARTITION_ENTRY_SIZE];

            // Parse partition entry fields
            let _status = entry[0];
            let _chs_start = CHSAddress::from_bytes(&entry[1..4]);
            let partition_type = MbrPartitionType::from_byte(entry[4]);
            let _chs_end = CHSAddress::from_bytes(&entry[5..8]);
            let lba_start = u32::from_le_bytes([entry[8], entry[9], entry[10], entry[11]]);
            let lba_length = u32::from_le_bytes([entry[12], entry[13], entry[14], entry[15]]);

            // Skip empty partitions
            if partition_type == MbrPartitionType::Empty || lba_length == 0 {
                continue;
            }

            // Calculate byte offsets
            let zone_offset = lba_start as u64 * sector_size as u64;
            let zone_length = lba_length as u64 * sector_size as u64;

            // Create zone
            let zone = Zone::new(i, zone_offset, zone_length, partition_type.name().to_string());

            zones.push(zone);
        }

        Ok(Self {
            zones,
            disk_signature,
            boot_signature,
        })
    }

    /// Get the disk signature
    pub fn disk_signature(&self) -> u32 {
        self.disk_signature
    }

    /// Get the boot signature (should always be 0xAA55)
    pub fn boot_signature(&self) -> u16 {
        self.boot_signature
    }

    /// Check if this MBR contains a GPT protective partition
    ///
    /// A GPT protective partition indicates that this is actually a GPT disk
    /// with a protective MBR for backwards compatibility.
    pub fn is_gpt_protective(&self) -> bool {
        self.zones.iter().any(|z| z.zone_type == "GPT Protective")
    }
}

impl ZoneTable for MbrZoneTable {
    fn identify(&self) -> &str {
        "Master Boot Record"
    }

    fn enumerate_zones(&self) -> &[Zone] {
        &self.zones
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// Create a minimal valid MBR with one partition
    fn create_test_mbr() -> Vec<u8> {
        let mut mbr = vec![0u8; 512];

        // Set disk signature
        mbr[0x1B8] = 0x12;
        mbr[0x1B9] = 0x34;
        mbr[0x1BA] = 0x56;
        mbr[0x1BB] = 0x78;

        // Partition entry 1: FAT32 LBA, 2048 sectors starting at LBA 2048
        let entry_offset = 0x1BE;
        mbr[entry_offset] = 0x80; // Bootable
        mbr[entry_offset + 1] = 0x00; // CHS start (head)
        mbr[entry_offset + 2] = 0x02; // CHS start (sector/cyl)
        mbr[entry_offset + 3] = 0x00; // CHS start (cyl low)
        mbr[entry_offset + 4] = 0x0C; // Type: FAT32 LBA
        mbr[entry_offset + 5] = 0x00; // CHS end (head)
        mbr[entry_offset + 6] = 0x00; // CHS end (sector/cyl)
        mbr[entry_offset + 7] = 0x00; // CHS end (cyl low)

        // LBA start: 2048
        mbr[entry_offset + 8] = 0x00;
        mbr[entry_offset + 9] = 0x08;
        mbr[entry_offset + 10] = 0x00;
        mbr[entry_offset + 11] = 0x00;

        // LBA length: 2048
        mbr[entry_offset + 12] = 0x00;
        mbr[entry_offset + 13] = 0x08;
        mbr[entry_offset + 14] = 0x00;
        mbr[entry_offset + 15] = 0x00;

        // Boot signature
        mbr[0x1FE] = 0x55;
        mbr[0x1FF] = 0xAA;

        mbr
    }

    #[test]
    fn test_parse_valid_mbr() {
        let mbr_data = create_test_mbr();
        let mut cursor = Cursor::new(mbr_data);

        let table = MbrZoneTable::parse(&mut cursor, 512).unwrap();

        assert_eq!(table.identify(), "Master Boot Record");
        assert_eq!(table.boot_signature(), 0xAA55);
        assert_eq!(table.disk_signature(), 0x78563412);
        assert_eq!(table.enumerate_zones().len(), 1);
    }

    #[test]
    fn test_parse_mbr_zone_details() {
        let mbr_data = create_test_mbr();
        let mut cursor = Cursor::new(mbr_data);

        let table = MbrZoneTable::parse(&mut cursor, 512).unwrap();
        let zones = table.enumerate_zones();

        assert_eq!(zones.len(), 1);
        assert_eq!(zones[0].index, 0);
        assert_eq!(zones[0].offset, 2048 * 512); // LBA 2048 * 512 bytes/sector
        assert_eq!(zones[0].length, 2048 * 512);
        assert_eq!(zones[0].zone_type, "FAT32 (LBA)");
    }

    #[test]
    fn test_parse_invalid_boot_signature() {
        let mut mbr_data = create_test_mbr();
        mbr_data[0x1FE] = 0x00; // Invalid signature

        let mut cursor = Cursor::new(mbr_data);
        let result = MbrZoneTable::parse(&mut cursor, 512);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid MBR boot signature"));
    }

    #[test]
    fn test_parse_empty_mbr() {
        let mut mbr = vec![0u8; 512];
        // Only set boot signature, no partitions
        mbr[0x1FE] = 0x55;
        mbr[0x1FF] = 0xAA;

        let mut cursor = Cursor::new(mbr);
        let table = MbrZoneTable::parse(&mut cursor, 512).unwrap();

        assert_eq!(table.enumerate_zones().len(), 0);
    }

    #[test]
    fn test_gpt_protective_detection() {
        let mut mbr = vec![0u8; 512];

        // Set GPT protective partition type
        let entry_offset = 0x1BE;
        mbr[entry_offset + 4] = 0xEE; // GPT protective

        // Set LBA values
        mbr[entry_offset + 8] = 0x01;
        mbr[entry_offset + 12] = 0x00;
        mbr[entry_offset + 13] = 0x08;

        // Boot signature
        mbr[0x1FE] = 0x55;
        mbr[0x1FF] = 0xAA;

        let mut cursor = Cursor::new(mbr);
        let table = MbrZoneTable::parse(&mut cursor, 512).unwrap();

        assert!(table.is_gpt_protective());
    }
}
