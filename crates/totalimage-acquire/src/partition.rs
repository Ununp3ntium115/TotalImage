//! Partition table creation for USB drives
//!
//! Provides functionality to create MBR and GPT partition tables for WinPE bootable USB drives.

use crate::error::{AcquireError, Result};
use std::io::{Seek, SeekFrom, Write};

/// Type of partition table to create
#[derive(Debug, Clone, Copy)]
pub enum PartitionTableType {
    /// Master Boot Record (MBR) - for BIOS systems
    Mbr,
    /// GUID Partition Table (GPT) - for UEFI systems
    Gpt,
}

/// Builder for creating partition tables
pub struct PartitionTableBuilder {
    table_type: PartitionTableType,
    sector_size: u32,
}

impl PartitionTableBuilder {
    /// Create a new partition table builder
    ///
    /// # Arguments
    ///
    /// * `table_type` - Type of partition table (MBR or GPT)
    /// * `sector_size` - Sector size in bytes (typically 512)
    pub fn new(table_type: PartitionTableType, sector_size: u32) -> Self {
        Self {
            table_type,
            sector_size,
        }
    }

    /// Create an MBR partition table with a single bootable partition
    ///
    /// # Arguments
    ///
    /// * `device` - Device to write the MBR to
    /// * `boot_partition_size` - Size of the boot partition in bytes
    ///
    /// # Returns
    ///
    /// The offset and length of the created partition
    pub fn create_mbr<W: Write + Seek>(
        &self,
        device: &mut W,
        boot_partition_size: u64,
    ) -> Result<(u64, u64)> {
        // MBR is always at sector 0
        device.seek(SeekFrom::Start(0))?;

        // Create MBR structure (512 bytes)
        let mut mbr = vec![0u8; 512];

        // Boot signature at offset 0x1FE (510)
        mbr[510] = 0x55;
        mbr[511] = 0xAA;

        // Partition entry 1 starts at offset 0x1BE (446)
        let partition_offset = 446;

        // Boot flag (0x80 = active/bootable)
        mbr[partition_offset] = 0x80;

        // Partition type: 0x0C = FAT32 LBA
        mbr[partition_offset + 4] = 0x0C;

        // Calculate LBA values
        // Start at sector 2048 (typical alignment)
        let start_lba = 2048u32;
        let size_sectors = (boot_partition_size / self.sector_size as u64) as u32;

        // Write LBA start (little endian, offset 454)
        mbr[partition_offset + 8..partition_offset + 12].copy_from_slice(&start_lba.to_le_bytes());

        // Write LBA size (little endian, offset 458)
        mbr[partition_offset + 12..partition_offset + 16]
            .copy_from_slice(&size_sectors.to_le_bytes());

        // Write MBR to device
        device.write_all(&mbr)?;
        device.flush()?;

        let partition_offset_bytes = start_lba as u64 * self.sector_size as u64;
        let partition_length_bytes = size_sectors as u64 * self.sector_size as u64;

        Ok((partition_offset_bytes, partition_length_bytes))
    }

    /// Create a GPT partition table with a single bootable partition
    ///
    /// # Arguments
    ///
    /// * `device` - Device to write the GPT to
    /// * `boot_partition_size` - Size of the boot partition in bytes
    ///
    /// # Returns
    ///
    /// The offset and length of the created partition
    pub fn create_gpt<W: Write + Seek>(
        &self,
        _device: &mut W,
        _boot_partition_size: u64,
    ) -> Result<(u64, u64)> {
        // GPT implementation is more complex and requires:
        // 1. Protective MBR at sector 0
        // 2. GPT header at sector 1
        // 3. Partition entry array starting at sector 2
        // 4. Backup GPT header and entries at end of disk

        // For now, return an error indicating this needs full implementation
        // TODO: Implement full GPT creation
        Err(AcquireError::UnsupportedPlatform(
            "GPT partition table creation not yet implemented".to_string(),
        ))
    }

    /// Create partition table based on the configured type
    pub fn create<W: Write + Seek>(
        &self,
        device: &mut W,
        boot_partition_size: u64,
    ) -> Result<(u64, u64)> {
        match self.table_type {
            PartitionTableType::Mbr => self.create_mbr(device, boot_partition_size),
            PartitionTableType::Gpt => self.create_gpt(device, boot_partition_size),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use totalimage_core::ZoneTable;
    use totalimage_zones::MbrZoneTable;

    #[test]
    fn test_mbr_creation() {
        let mut device = Cursor::new(vec![0u8; 1024 * 1024]); // 1MB test device
        let builder = PartitionTableBuilder::new(PartitionTableType::Mbr, 512);
        let partition_size = 100 * 1024 * 1024; // 100MB

        let result = builder.create_mbr(&mut device, partition_size);
        assert!(result.is_ok());

        let (offset, length) = result.unwrap();
        assert_eq!(offset, 2048 * 512); // Start at sector 2048
        assert!(length > 0);

        // Verify MBR can be parsed by existing parser
        device.set_position(0);
        let mbr = MbrZoneTable::parse(&mut device, 512);
        assert!(mbr.is_ok());

        let table = mbr.unwrap();
        let zones = table.enumerate_zones();
        assert_eq!(zones.len(), 1);
        assert_eq!(zones[0].offset, offset);
    }

    #[test]
    fn test_gpt_creation_placeholder() {
        let mut device = Cursor::new(vec![0u8; 1024 * 1024]);
        let builder = PartitionTableBuilder::new(PartitionTableType::Gpt, 512);
        let partition_size = 100 * 1024 * 1024;

        let result = builder.create_gpt(&mut device, partition_size);
        // Should return error until implemented
        assert!(result.is_err());
    }
}
