//! Partition table creation for USB drives
//!
//! Provides functionality to create MBR and GPT partition tables for WinPE bootable USB drives.

use crate::error::Result;
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

    /// Create a GPT partition table with a single EFI System Partition
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
        device: &mut W,
        boot_partition_size: u64,
    ) -> Result<(u64, u64)> {
        // GPT structure:
        // - Sector 0: Protective MBR
        // - Sector 1: Primary GPT Header
        // - Sectors 2-33: Partition Entry Array (128 entries × 128 bytes = 16KB)
        // - Data area starts at sector 34 (or later for alignment)
        // - Backup GPT entries and header at end of disk

        // Calculate total disk size (estimate from partition size)
        // Assume partition starts at sector 2048 for alignment
        let start_lba = 2048u64;
        let partition_sectors = boot_partition_size / self.sector_size as u64;
        let total_sectors = start_lba + partition_sectors + 34; // Data + backup GPT

        // 1. Write protective MBR at sector 0
        self.write_protective_mbr(device, total_sectors)?;

        // 2. Write primary GPT header at sector 1
        let partition_entry_lba = 2u64; // Partition entries start at sector 2
        let num_partition_entries = 128u32;
        let partition_entry_size = 128u32;
        self.write_gpt_header(
            device,
            1,                                  // Header LBA
            total_sectors - 1,                  // Backup header LBA (last sector)
            start_lba,                          // First usable LBA
            total_sectors - 34,                 // Last usable LBA
            partition_entry_lba,                // Partition array LBA
            num_partition_entries,
            partition_entry_size,
        )?;

        // 3. Write partition entry array at sectors 2-33
        self.write_gpt_partition_entries(
            device,
            partition_entry_lba,
            start_lba,
            partition_sectors,
        )?;

        // 4. Write backup partition entries and header at end
        let backup_partition_entry_lba = total_sectors - 33;
        self.write_gpt_partition_entries(
            device,
            backup_partition_entry_lba,
            start_lba,
            partition_sectors,
        )?;
        self.write_gpt_header(
            device,
            total_sectors - 1,                  // Backup header LBA (last sector)
            1,                                  // Primary header LBA
            start_lba,
            total_sectors - 34,
            backup_partition_entry_lba,
            num_partition_entries,
            partition_entry_size,
        )?;

        let partition_offset = start_lba * self.sector_size as u64;
        let partition_length = partition_sectors * self.sector_size as u64;

        Ok((partition_offset, partition_length))
    }

    /// Write protective MBR for GPT
    fn write_protective_mbr<W: Write + Seek>(
        &self,
        device: &mut W,
        total_sectors: u64,
    ) -> Result<()> {
        device.seek(SeekFrom::Start(0))?;

        let mut mbr = vec![0u8; 512];

        // Partition entry 1 (protective GPT partition)
        let entry_offset = 446;
        mbr[entry_offset + 4] = 0xEE; // Partition type: GPT protective

        // LBA start: sector 1 (GPT header)
        mbr[entry_offset + 8..entry_offset + 12].copy_from_slice(&1u32.to_le_bytes());

        // LBA size: total sectors - 1 (or 0xFFFFFFFF if too large)
        let size_lba = if total_sectors > u32::MAX as u64 {
            0xFFFFFFFFu32
        } else {
            (total_sectors - 1) as u32
        };
        mbr[entry_offset + 12..entry_offset + 16].copy_from_slice(&size_lba.to_le_bytes());

        // Boot signature
        mbr[510] = 0x55;
        mbr[511] = 0xAA;

        device.write_all(&mbr)?;
        device.flush()?;

        Ok(())
    }

    /// Write GPT header
    #[allow(clippy::too_many_arguments)]
    fn write_gpt_header<W: Write + Seek>(
        &self,
        device: &mut W,
        header_lba: u64,
        alternate_lba: u64,
        first_usable_lba: u64,
        last_usable_lba: u64,
        partition_entry_lba: u64,
        num_partition_entries: u32,
        partition_entry_size: u32,
    ) -> Result<()> {
        device.seek(SeekFrom::Start(header_lba * self.sector_size as u64))?;

        let mut header = vec![0u8; 512];

        // Signature: "EFI PART"
        header[0..8].copy_from_slice(b"EFI PART");

        // Revision: 1.0 (0x00010000)
        header[8..12].copy_from_slice(&0x00010000u32.to_le_bytes());

        // Header size: 92 bytes
        header[12..16].copy_from_slice(&92u32.to_le_bytes());

        // CRC32 (calculated below, zero for now)
        header[16..20].copy_from_slice(&0u32.to_le_bytes());

        // Reserved (must be zero)
        header[20..24].copy_from_slice(&0u32.to_le_bytes());

        // Current LBA
        header[24..32].copy_from_slice(&header_lba.to_le_bytes());

        // Backup LBA
        header[32..40].copy_from_slice(&alternate_lba.to_le_bytes());

        // First usable LBA
        header[40..48].copy_from_slice(&first_usable_lba.to_le_bytes());

        // Last usable LBA
        header[48..56].copy_from_slice(&last_usable_lba.to_le_bytes());

        // Disk GUID (random for testing)
        let disk_guid = [
            0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0,
            0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88,
        ];
        header[56..72].copy_from_slice(&disk_guid);

        // Partition entry LBA
        header[72..80].copy_from_slice(&partition_entry_lba.to_le_bytes());

        // Number of partition entries
        header[80..84].copy_from_slice(&num_partition_entries.to_le_bytes());

        // Size of partition entry (typically 128)
        header[84..88].copy_from_slice(&partition_entry_size.to_le_bytes());

        // CRC32 of partition array (calculated from actual entries)
        // For now, use zero - in real implementation, read entries and calculate
        header[88..92].copy_from_slice(&0u32.to_le_bytes());

        // Calculate CRC32 of header (first 92 bytes)
        let crc = calculate_crc32(&header[0..92]);
        header[16..20].copy_from_slice(&crc.to_le_bytes());

        device.write_all(&header)?;
        device.flush()?;

        Ok(())
    }

    /// Write GPT partition entries
    fn write_gpt_partition_entries<W: Write + Seek>(
        &self,
        device: &mut W,
        entry_lba: u64,
        partition_start_lba: u64,
        partition_size_lba: u64,
    ) -> Result<()> {
        device.seek(SeekFrom::Start(entry_lba * self.sector_size as u64))?;

        // GPT partition entry array: 128 entries × 128 bytes = 16KB (32 sectors)
        let mut entries = vec![0u8; 128 * 128];

        // Entry 0: EFI System Partition
        let entry_offset = 0;

        // Partition type GUID: EFI System Partition
        // C12A7328-F81F-11D2-BA4B-00A0C93EC93B
        let efi_system_guid = [
            0x28, 0x73, 0x2A, 0xC1, 0x1F, 0xF8, 0xD2, 0x11,
            0xBA, 0x4B, 0x00, 0xA0, 0xC9, 0x3E, 0xC9, 0x3B,
        ];
        entries[entry_offset..entry_offset + 16].copy_from_slice(&efi_system_guid);

        // Unique partition GUID (random for testing)
        let partition_guid = [
            0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x11, 0x22,
            0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA,
        ];
        entries[entry_offset + 16..entry_offset + 32].copy_from_slice(&partition_guid);

        // First LBA
        entries[entry_offset + 32..entry_offset + 40]
            .copy_from_slice(&partition_start_lba.to_le_bytes());

        // Last LBA
        let last_lba = partition_start_lba + partition_size_lba - 1;
        entries[entry_offset + 40..entry_offset + 48].copy_from_slice(&last_lba.to_le_bytes());

        // Attribute flags (bit 0 = required partition)
        entries[entry_offset + 48..entry_offset + 56].copy_from_slice(&1u64.to_le_bytes());

        // Partition name (UTF-16LE, up to 36 characters)
        let name = "EFI System";
        let name_utf16: Vec<u16> = name.encode_utf16().collect();
        for (i, &ch) in name_utf16.iter().enumerate().take(36) {
            let offset = entry_offset + 56 + (i * 2);
            entries[offset..offset + 2].copy_from_slice(&ch.to_le_bytes());
        }

        device.write_all(&entries)?;
        device.flush()?;

        Ok(())
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
    fn test_gpt_creation() {
        let mut device = Cursor::new(vec![0u8; 10 * 1024 * 1024]); // 10MB test device
        let builder = PartitionTableBuilder::new(PartitionTableType::Gpt, 512);
        let partition_size = 5 * 1024 * 1024; // 5MB partition

        let result = builder.create_gpt(&mut device, partition_size);
        assert!(result.is_ok());

        let (offset, length) = result.unwrap();
        assert_eq!(offset, 2048 * 512); // Start at sector 2048
        assert!(length > 0);

        // Verify protective MBR signature
        device.set_position(510);
        let mut sig = [0u8; 2];
        std::io::Read::read_exact(&mut device, &mut sig).unwrap();
        assert_eq!(&sig, &[0x55, 0xAA]);

        // Verify GPT signature at sector 1
        device.set_position(512);
        let mut gpt_sig = [0u8; 8];
        std::io::Read::read_exact(&mut device, &mut gpt_sig).unwrap();
        assert_eq!(&gpt_sig, b"EFI PART");
    }
}

/// Calculate CRC32 checksum (IEEE polynomial)
fn calculate_crc32(data: &[u8]) -> u32 {
    const CRC32_POLYNOMIAL: u32 = 0xEDB88320;

    let mut crc = 0xFFFFFFFFu32;

    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ CRC32_POLYNOMIAL;
            } else {
                crc >>= 1;
            }
        }
    }

    !crc
}
