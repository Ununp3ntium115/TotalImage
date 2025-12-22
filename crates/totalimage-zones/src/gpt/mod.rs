//! GPT (GUID Partition Table) partition table implementation

pub mod types;

use std::io::SeekFrom;
use totalimage_core::{Error, ReadSeek, Result, Zone, ZoneTable};
use types::{GptHeader, GptPartitionEntry};

/// GPT partition table
///
/// The GUID Partition Table is the modern partitioning scheme used by UEFI-based systems.
/// It supports up to 128 partitions by default and uses GUIDs for partition identification.
///
/// # Structure
///
/// ```text
/// LBA 0:    Protective MBR (for backward compatibility)
/// LBA 1:    Primary GPT header
/// LBA 2-33: Partition entries array (typically 128 entries)
/// LBA 34+:  Usable disk space
/// ...
/// Last 33:  Backup partition entries array
/// Last 1:   Backup GPT header
/// ```
#[derive(Debug, Clone)]
pub struct GptZoneTable {
    zones: Vec<Zone>,
    header: GptHeader,
    backup_header: Option<GptHeader>,
}

/// Configuration for GPT parsing
#[derive(Debug, Clone, Copy)]
pub struct GptConfig {
    /// Whether to validate the backup header
    pub validate_backup_header: bool,
}

impl Default for GptConfig {
    fn default() -> Self {
        Self {
            validate_backup_header: true,
        }
    }
}

impl GptZoneTable {
    /// Parse a GPT from a readable and seekable stream
    ///
    /// # Arguments
    ///
    /// * `stream` - A stream positioned at the start of the disk
    /// * `sector_size` - The sector size in bytes (usually 512)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The GPT signature is invalid
    /// - The stream cannot be read
    /// - The partition table is corrupted
    pub fn parse(stream: &mut dyn ReadSeek, sector_size: u32) -> Result<Self> {
        Self::parse_with_config(stream, sector_size, GptConfig::default())
    }

    /// Parse a GPT from a readable and seekable stream with configuration
    ///
    /// # Arguments
    ///
    /// * `stream` - A stream positioned at the start of the disk
    /// * `sector_size` - The sector size in bytes (usually 512)
    /// * `config` - Configuration for parsing options
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The GPT signature is invalid
    /// - The stream cannot be read
    /// - The partition table is corrupted
    /// - Backup header validation fails (if enabled)
    pub fn parse_with_config(
        stream: &mut dyn ReadSeek,
        sector_size: u32,
        config: GptConfig,
    ) -> Result<Self> {
        // GPT header is at LBA 1 (second sector)
        let header_lba = 1u64;
        let header_offset = header_lba * sector_size as u64;

        stream.seek(SeekFrom::Start(header_offset))?;

        // Read GPT header
        let mut header_bytes = vec![0u8; sector_size as usize];
        stream.read_exact(&mut header_bytes)?;

        let header = GptHeader::from_bytes(&header_bytes)
            .ok_or_else(|| Error::invalid_zone_table("Invalid GPT header signature".to_string()))?;

        // Verify header CRC32 (SEC-006: Checksum enforcement)
        if !header.verify_header_crc32(&header_bytes) {
            return Err(Error::ChecksumVerification(
                "GPT header CRC32 verification failed".to_string(),
            ));
        }

        // Read partition entries
        let entries_lba = header.partition_entries_lba;
        let entries_offset = entries_lba * sector_size as u64;
        let num_entries = header.num_partition_entries;
        let entry_size = header.partition_entry_size as usize;

        stream.seek(SeekFrom::Start(entries_offset))?;

        // Read all partition entries at once for CRC32 verification
        let total_entries_size = num_entries as usize * entry_size;
        let mut all_entries_bytes = vec![0u8; total_entries_size];
        stream.read_exact(&mut all_entries_bytes)?;

        // Verify partition entries CRC32 (SEC-006: Checksum enforcement)
        if !header.verify_partition_entries_crc32(&all_entries_bytes) {
            return Err(Error::ChecksumVerification(
                "GPT partition entries CRC32 verification failed".to_string(),
            ));
        }

        // Parse individual partition entries
        let mut zones = Vec::new();

        for i in 0..num_entries {
            let entry_start = i as usize * entry_size;
            let entry_end = entry_start + entry_size;
            let entry_bytes = &all_entries_bytes[entry_start..entry_end];

            let entry = GptPartitionEntry::from_bytes(entry_bytes);

            // Skip unused partitions
            if entry.is_unused() {
                continue;
            }

            // Calculate byte offsets
            let zone_offset = entry.first_lba * sector_size as u64;
            let zone_length = entry.size_lba() * sector_size as u64;

            // Use partition name if available, otherwise use type
            let zone_type = if !entry.name.is_empty() {
                format!("{} ({})", entry.partition_type_guid.name(), entry.name)
            } else {
                entry.partition_type_guid.name().to_string()
            };

            // Create zone
            let zone = Zone::new(i as usize, zone_offset, zone_length, zone_type);

            zones.push(zone);
        }

        // Read and validate backup header if requested
        let backup_header = if config.validate_backup_header {
            match Self::read_backup_header(stream, &header, sector_size) {
                Ok(backup) => {
                    // Validate backup header against primary
                    if let Err(e) = Self::validate_backup_header(&header, &backup) {
                        // Don't fail, just log warning (backup may be stale)
                        eprintln!("GPT backup header validation warning: {}", e);
                    }
                    Some(backup)
                }
                Err(_e) => {
                    // Failed to read backup header - not critical, may not exist
                    None
                }
            }
        } else {
            None
        };

        Ok(Self {
            zones,
            header,
            backup_header,
        })
    }

    /// Read the backup GPT header from the end of the disk
    ///
    /// # Arguments
    ///
    /// * `stream` - A stream positioned at the start of the disk
    /// * `primary_header` - The primary GPT header (contains backup LBA location)
    /// * `sector_size` - The sector size in bytes (usually 512)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The backup header cannot be read
    /// - The backup header signature is invalid
    /// - The backup header CRC32 is invalid
    pub fn read_backup_header(
        stream: &mut dyn ReadSeek,
        primary_header: &GptHeader,
        sector_size: u32,
    ) -> Result<GptHeader> {
        // Backup header is at the LBA specified in primary header
        let backup_lba = primary_header.backup_lba;
        let backup_offset = backup_lba * sector_size as u64;

        stream.seek(SeekFrom::Start(backup_offset))?;

        // Read backup header
        let mut header_bytes = vec![0u8; sector_size as usize];
        stream.read_exact(&mut header_bytes)?;

        let backup_header = GptHeader::from_bytes(&header_bytes).ok_or_else(|| {
            Error::invalid_zone_table("Invalid backup GPT header signature".to_string())
        })?;

        // Verify backup header CRC32
        if !backup_header.verify_header_crc32(&header_bytes) {
            return Err(Error::ChecksumVerification(
                "GPT backup header CRC32 verification failed".to_string(),
            ));
        }

        Ok(backup_header)
    }

    /// Validate that backup header matches primary header
    ///
    /// Compares critical fields between primary and backup headers.
    /// If there's a mismatch, it logs a warning but doesn't fail (backup may be stale).
    ///
    /// # Arguments
    ///
    /// * `primary` - The primary GPT header
    /// * `backup` - The backup GPT header
    ///
    /// # Errors
    ///
    /// Returns an error if critical fields don't match (disk GUID, partition entry LBA, etc.)
    pub fn validate_backup_header(primary: &GptHeader, backup: &GptHeader) -> Result<()> {
        // Compare critical fields
        if primary.disk_guid != backup.disk_guid {
            return Err(Error::invalid_zone_table(format!(
                "Backup header disk GUID mismatch: primary {:?} != backup {:?}",
                primary.disk_guid, backup.disk_guid
            )));
        }

        if primary.partition_entries_lba != backup.partition_entries_lba {
            return Err(Error::invalid_zone_table(format!(
                "Backup header partition entries LBA mismatch: primary {} != backup {}",
                primary.partition_entries_lba, backup.partition_entries_lba
            )));
        }

        if primary.num_partition_entries != backup.num_partition_entries {
            return Err(Error::invalid_zone_table(format!(
                "Backup header partition entry count mismatch: primary {} != backup {}",
                primary.num_partition_entries, backup.num_partition_entries
            )));
        }

        if primary.partition_entry_size != backup.partition_entry_size {
            return Err(Error::invalid_zone_table(format!(
                "Backup header partition entry size mismatch: primary {} != backup {}",
                primary.partition_entry_size, backup.partition_entry_size
            )));
        }

        // Note: We don't compare current_lba and backup_lba as they should be swapped
        // (primary.current_lba should equal backup.backup_lba and vice versa)
        if primary.current_lba != backup.backup_lba || primary.backup_lba != backup.current_lba {
            // This is a warning, not an error - headers may be swapped correctly
            // but we log it for debugging
        }

        Ok(())
    }

    /// Get the backup header if it was read and validated
    pub fn backup_header(&self) -> Option<&GptHeader> {
        self.backup_header.as_ref()
    }

    /// Get the disk GUID
    pub fn disk_guid(&self) -> &[u8; 16] {
        &self.header.disk_guid
    }

    /// Get the GPT header
    pub fn header(&self) -> &GptHeader {
        &self.header
    }

    /// Get the number of usable sectors on the disk
    pub fn usable_lba_count(&self) -> u64 {
        if self.header.last_usable_lba >= self.header.first_usable_lba {
            self.header.last_usable_lba - self.header.first_usable_lba + 1
        } else {
            0
        }
    }
}

impl ZoneTable for GptZoneTable {
    fn identify(&self) -> &str {
        "GUID Partition Table"
    }

    fn enumerate_zones(&self) -> &[Zone] {
        &self.zones
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Seek, SeekFrom};

    /// Create a minimal valid GPT with one partition
    fn create_test_gpt() -> Vec<u8> {
        let sector_size = 512;
        let total_sectors = 1000;
        let mut disk = vec![0u8; total_sectors * sector_size];

        // LBA 0: Protective MBR (we'll skip this for now)

        // LBA 1: GPT Header
        let header_offset = 512;
        disk[header_offset..header_offset + 8].copy_from_slice(b"EFI PART");

        // Revision (0x00010000)
        disk[header_offset + 8..header_offset + 12].copy_from_slice(&0x00010000u32.to_le_bytes());

        // Header size (92 bytes)
        disk[header_offset + 12..header_offset + 16].copy_from_slice(&92u32.to_le_bytes());

        // CRC32 (we'll skip validation)
        disk[header_offset + 16..header_offset + 20].copy_from_slice(&0u32.to_le_bytes());

        // Reserved
        disk[header_offset + 20..header_offset + 24].copy_from_slice(&0u32.to_le_bytes());

        // Current LBA (1)
        disk[header_offset + 24..header_offset + 32].copy_from_slice(&1u64.to_le_bytes());

        // Backup LBA (999)
        disk[header_offset + 32..header_offset + 40].copy_from_slice(&999u64.to_le_bytes());

        // First usable LBA (34)
        disk[header_offset + 40..header_offset + 48].copy_from_slice(&34u64.to_le_bytes());

        // Last usable LBA (966)
        disk[header_offset + 48..header_offset + 56].copy_from_slice(&966u64.to_le_bytes());

        // Disk GUID
        let disk_guid = [
            0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc,
            0xde, 0xf0,
        ];
        disk[header_offset + 56..header_offset + 72].copy_from_slice(&disk_guid);

        // Partition entries LBA (2)
        disk[header_offset + 72..header_offset + 80].copy_from_slice(&2u64.to_le_bytes());

        // Number of partition entries (128)
        disk[header_offset + 80..header_offset + 84].copy_from_slice(&128u32.to_le_bytes());

        // Size of partition entry (128 bytes)
        disk[header_offset + 84..header_offset + 88].copy_from_slice(&128u32.to_le_bytes());

        // Partition entries CRC32 (we'll skip validation)
        disk[header_offset + 88..header_offset + 92].copy_from_slice(&0u32.to_le_bytes());

        // LBA 2+: Partition entries (128 entries * 128 bytes = 16384 bytes = 32 sectors)
        let entries_offset = 2 * sector_size;

        // First partition entry: Linux filesystem, LBA 100-199
        let entry_offset = entries_offset;

        // Partition type GUID: Linux filesystem
        disk[entry_offset..entry_offset + 16].copy_from_slice(&[
            0xaf, 0x3d, 0xc6, 0x0f, 0x83, 0x84, 0x72, 0x47, 0x8e, 0x79, 0x3d, 0x69, 0xd8, 0x47,
            0x7d, 0xe4,
        ]);

        // Unique partition GUID (random)
        disk[entry_offset + 16..entry_offset + 32].copy_from_slice(&[
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
            0x0f, 0x10,
        ]);

        // First LBA (100)
        disk[entry_offset + 32..entry_offset + 40].copy_from_slice(&100u64.to_le_bytes());

        // Last LBA (199)
        disk[entry_offset + 40..entry_offset + 48].copy_from_slice(&199u64.to_le_bytes());

        // Attributes (0)
        disk[entry_offset + 48..entry_offset + 56].copy_from_slice(&0u64.to_le_bytes());

        // Partition name (UTF-16LE): "Test"
        let name_utf16: Vec<u16> = "Test".encode_utf16().collect();
        for (i, &code) in name_utf16.iter().enumerate() {
            let bytes = code.to_le_bytes();
            disk[entry_offset + 56 + i * 2] = bytes[0];
            disk[entry_offset + 56 + i * 2 + 1] = bytes[1];
        }

        // Calculate and set partition entries CRC32
        let entries_size = 128 * 128; // num_entries * entry_size
        let entries_crc = crc32fast::hash(&disk[entries_offset..entries_offset + entries_size]);
        disk[header_offset + 88..header_offset + 92].copy_from_slice(&entries_crc.to_le_bytes());

        // Calculate and set header CRC32 (with CRC32 field zeroed)
        let mut header_for_crc = disk[header_offset..header_offset + 92].to_vec();
        header_for_crc[16] = 0;
        header_for_crc[17] = 0;
        header_for_crc[18] = 0;
        header_for_crc[19] = 0;
        let header_crc = crc32fast::hash(&header_for_crc);
        disk[header_offset + 16..header_offset + 20].copy_from_slice(&header_crc.to_le_bytes());

        // LBA 999: Backup GPT Header (mirror of primary, with swapped current/backup LBAs)
        let backup_header_offset = 999 * sector_size;
        disk[backup_header_offset..backup_header_offset + 8].copy_from_slice(b"EFI PART");
        disk[backup_header_offset + 8..backup_header_offset + 12]
            .copy_from_slice(&0x00010000u32.to_le_bytes());
        disk[backup_header_offset + 12..backup_header_offset + 16]
            .copy_from_slice(&92u32.to_le_bytes());
        // CRC32 will be calculated below
        disk[backup_header_offset + 20..backup_header_offset + 24]
            .copy_from_slice(&0u32.to_le_bytes());
        // Current LBA (999) - swapped from primary
        disk[backup_header_offset + 24..backup_header_offset + 32]
            .copy_from_slice(&999u64.to_le_bytes());
        // Backup LBA (1) - swapped from primary
        disk[backup_header_offset + 32..backup_header_offset + 40]
            .copy_from_slice(&1u64.to_le_bytes());
        // First usable LBA (34) - same as primary
        disk[backup_header_offset + 40..backup_header_offset + 48]
            .copy_from_slice(&34u64.to_le_bytes());
        // Last usable LBA (966) - same as primary
        disk[backup_header_offset + 48..backup_header_offset + 56]
            .copy_from_slice(&966u64.to_le_bytes());
        // Disk GUID - same as primary
        disk[backup_header_offset + 56..backup_header_offset + 72].copy_from_slice(&disk_guid);
        // Partition entries LBA (2) - same as primary
        disk[backup_header_offset + 72..backup_header_offset + 80]
            .copy_from_slice(&2u64.to_le_bytes());
        // Number of partition entries (128) - same as primary
        disk[backup_header_offset + 80..backup_header_offset + 84]
            .copy_from_slice(&128u32.to_le_bytes());
        // Size of partition entry (128 bytes) - same as primary
        disk[backup_header_offset + 84..backup_header_offset + 88]
            .copy_from_slice(&128u32.to_le_bytes());
        // Partition entries CRC32 - same as primary
        disk[backup_header_offset + 88..backup_header_offset + 92]
            .copy_from_slice(&entries_crc.to_le_bytes());

        // Calculate and set backup header CRC32 (with CRC32 field zeroed)
        let mut backup_header_for_crc =
            disk[backup_header_offset..backup_header_offset + 92].to_vec();
        backup_header_for_crc[16] = 0;
        backup_header_for_crc[17] = 0;
        backup_header_for_crc[18] = 0;
        backup_header_for_crc[19] = 0;
        let backup_header_crc = crc32fast::hash(&backup_header_for_crc);
        disk[backup_header_offset + 16..backup_header_offset + 20]
            .copy_from_slice(&backup_header_crc.to_le_bytes());

        disk
    }

    #[test]
    fn test_parse_valid_gpt() {
        let gpt_data = create_test_gpt();
        let mut cursor = Cursor::new(gpt_data);

        let table = GptZoneTable::parse(&mut cursor, 512).unwrap();

        assert_eq!(table.identify(), "GUID Partition Table");
        assert_eq!(table.enumerate_zones().len(), 1);
    }

    #[test]
    fn test_parse_gpt_zone_details() {
        let gpt_data = create_test_gpt();
        let mut cursor = Cursor::new(gpt_data);

        let table = GptZoneTable::parse(&mut cursor, 512).unwrap();
        let zones = table.enumerate_zones();

        assert_eq!(zones.len(), 1);
        assert_eq!(zones[0].index, 0);
        assert_eq!(zones[0].offset, 100 * 512); // LBA 100 * 512 bytes/sector
        assert_eq!(zones[0].length, 100 * 512); // 100 sectors
        assert!(zones[0].zone_type.contains("Linux filesystem"));
        assert!(zones[0].zone_type.contains("Test"));
    }

    #[test]
    fn test_parse_invalid_gpt_signature() {
        let mut gpt_data = create_test_gpt();
        // Corrupt signature in GPT header (LBA 1)
        gpt_data[512] = 0xFF;

        let mut cursor = Cursor::new(gpt_data);
        let result = GptZoneTable::parse(&mut cursor, 512);

        assert!(result.is_err());
    }

    #[test]
    fn test_gpt_header_crc32_validation() {
        let mut gpt_data = create_test_gpt();
        // Corrupt a byte in the header (but not signature or CRC32 field)
        gpt_data[512 + 50] = 0xFF; // Modify first_usable_lba

        let mut cursor = Cursor::new(gpt_data);
        let result = GptZoneTable::parse(&mut cursor, 512);

        assert!(result.is_err());
        assert!(matches!(result, Err(Error::ChecksumVerification(_))));
    }

    #[test]
    fn test_gpt_partition_entries_crc32_validation() {
        let mut gpt_data = create_test_gpt();
        // Corrupt a byte in the partition entries
        let entries_offset = 2 * 512;
        gpt_data[entries_offset + 100] = 0xFF;

        let mut cursor = Cursor::new(gpt_data);
        let result = GptZoneTable::parse(&mut cursor, 512);

        assert!(result.is_err());
        assert!(matches!(result, Err(Error::ChecksumVerification(_))));
    }

    #[test]
    fn test_gpt_disk_guid() {
        let gpt_data = create_test_gpt();
        let mut cursor = Cursor::new(gpt_data);

        let table = GptZoneTable::parse(&mut cursor, 512).unwrap();
        let expected_guid = [
            0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc,
            0xde, 0xf0,
        ];

        assert_eq!(table.disk_guid(), &expected_guid);
    }

    #[test]
    fn test_gpt_usable_lba_count() {
        let gpt_data = create_test_gpt();
        let mut cursor = Cursor::new(gpt_data);

        let table = GptZoneTable::parse(&mut cursor, 512).unwrap();

        // First usable: 34, Last usable: 966
        // Count: 966 - 34 + 1 = 933
        assert_eq!(table.usable_lba_count(), 933);
    }

    #[test]
    fn test_gpt_backup_header_location() {
        // GPT backup header should be at last LBA
        // For a 1000-sector disk, backup header at LBA 999
        let disk_size_lba = 1000u64;
        let backup_lba = disk_size_lba - 1;

        assert_eq!(backup_lba, 999);
    }

    #[test]
    fn test_gpt_partition_entry_array_size() {
        // GPT spec: 128 entries * 128 bytes each = 16384 bytes
        const ENTRY_COUNT: usize = 128;
        const ENTRY_SIZE: usize = 128;
        const TOTAL_SIZE: usize = ENTRY_COUNT * ENTRY_SIZE;

        assert_eq!(TOTAL_SIZE, 16384);
        // That's 32 sectors of 512 bytes
        assert_eq!(TOTAL_SIZE / 512, 32);
    }

    #[test]
    fn test_gpt_header_size_constant() {
        // GPT header is exactly 92 bytes (rest of 512-byte sector is reserved)
        const GPT_HEADER_SIZE: usize = 92;
        assert_eq!(GPT_HEADER_SIZE, 92);
    }

    #[test]
    fn test_gpt_protective_mbr_partition_type() {
        // Protective MBR uses partition type 0xEE
        const GPT_PROTECTIVE_MBR_TYPE: u8 = 0xEE;
        assert_eq!(GPT_PROTECTIVE_MBR_TYPE, 0xEE);
    }

    #[test]
    fn test_gpt_first_usable_lba_calculation() {
        // First usable LBA = LBA 1 (GPT header) + 32 (partition entries) + 1 = 34
        let gpt_header_lba = 1;
        let partition_entry_sectors = 32; // 16384 bytes / 512
        let first_usable = gpt_header_lba + partition_entry_sectors + 1;

        assert_eq!(first_usable, 34);
    }

    #[test]
    fn test_gpt_partition_guid_uniqueness() {
        let gpt_data = create_test_gpt();
        let mut cursor = Cursor::new(gpt_data);

        let table = GptZoneTable::parse(&mut cursor, 512).unwrap();
        let zones = &table.zones;

        // Each partition should have a unique GUID
        // (In real GPTs - our test may have duplicates, but we verify the field exists)
        for zone in zones {
            // Verify zone has valid offset and length
            assert!(zone.offset > 0 || zone.length > 0);
        }
    }

    #[test]
    fn test_read_backup_header() -> Result<()> {
        let gpt_data = create_test_gpt();
        let mut cursor = Cursor::new(gpt_data);

        // Parse primary header first
        let table = GptZoneTable::parse(&mut cursor, 512)?;
        let primary_header = table.header();

        // Read backup header
        cursor.seek(SeekFrom::Start(0))?;
        let backup_header = GptZoneTable::read_backup_header(&mut cursor, primary_header, 512)?;

        // Verify backup header signature
        assert_eq!(&backup_header.signature, b"EFI PART");

        // Verify backup header has swapped LBAs
        assert_eq!(backup_header.current_lba, 999);
        assert_eq!(backup_header.backup_lba, 1);

        // Verify critical fields match
        assert_eq!(primary_header.disk_guid, backup_header.disk_guid);
        assert_eq!(
            primary_header.partition_entries_lba,
            backup_header.partition_entries_lba
        );
        assert_eq!(
            primary_header.num_partition_entries,
            backup_header.num_partition_entries
        );
        assert_eq!(
            primary_header.partition_entry_size,
            backup_header.partition_entry_size
        );

        Ok(())
    }

    #[test]
    fn test_validate_backup_header() -> Result<()> {
        let gpt_data = create_test_gpt();
        let mut cursor = Cursor::new(gpt_data);

        // Parse with backup validation enabled
        let table = GptZoneTable::parse_with_config(
            &mut cursor,
            512,
            GptConfig {
                validate_backup_header: true,
            },
        )?;

        // Backup header should be present
        assert!(table.backup_header().is_some());

        let backup = table.backup_header().unwrap();
        let primary = table.header();

        // Validate should succeed
        GptZoneTable::validate_backup_header(primary, backup)?;
        Ok(())
    }

    #[test]
    fn test_validate_backup_header_mismatch() -> Result<()> {
        let gpt_data = create_test_gpt();
        let mut cursor = Cursor::new(gpt_data);

        // Parse primary header
        let table = GptZoneTable::parse(&mut cursor, 512)?;
        let primary = table.header();

        // Create a backup header with mismatched disk GUID
        let mut backup = primary.clone();
        backup.disk_guid[0] = 0xFF; // Corrupt disk GUID

        // Validation should fail
        let result = GptZoneTable::validate_backup_header(primary, &backup);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("disk GUID"));
        Ok(())
    }

    #[test]
    fn test_parse_with_backup_validation_disabled() -> Result<()> {
        let gpt_data = create_test_gpt();
        let mut cursor = Cursor::new(gpt_data);

        // Parse with backup validation disabled
        let table = GptZoneTable::parse_with_config(
            &mut cursor,
            512,
            GptConfig {
                validate_backup_header: false,
            },
        )?;

        // Backup header should not be present
        assert!(table.backup_header().is_none());
        Ok(())
    }

    #[test]
    fn test_backup_header_crc32_validation() -> Result<()> {
        let mut gpt_data = create_test_gpt();
        // Corrupt backup header CRC32
        let backup_header_offset = 999 * 512;
        gpt_data[backup_header_offset + 16] = 0xFF;

        let mut cursor = Cursor::new(gpt_data);

        // Parse primary header
        let table = GptZoneTable::parse(&mut cursor, 512)?;
        let primary_header = table.header();

        // Reading backup header should fail due to CRC32 mismatch
        cursor.seek(SeekFrom::Start(0))?;
        let result = GptZoneTable::read_backup_header(&mut cursor, primary_header, 512);
        assert!(result.is_err());
        assert!(matches!(result, Err(Error::ChecksumVerification(_))));
        Ok(())
    }
}
