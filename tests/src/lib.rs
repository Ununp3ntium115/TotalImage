//! Integration test library for TotalImage
//!
//! This crate provides shared utilities and helpers for integration tests.

/// Test utilities and helpers
pub mod utils {
    use std::path::Path;

    /// Check if test fixtures are available
    pub fn fixtures_available() -> bool {
        Path::new("tests/fixtures").exists()
    }
}

/// Synthetic disk image generators for testing
pub mod generators {
    use totalimage_core::Result;

    /// Create a FAT12 floppy disk image (1.44 MB)
    ///
    /// Returns a complete bootable FAT12 image with:
    /// - Boot sector with BPB
    /// - 2 FAT copies
    /// - Root directory
    /// - Data area
    pub fn create_fat12_floppy() -> Result<Vec<u8>> {
        const SECTOR_SIZE: usize = 512;
        const TOTAL_SECTORS: usize = 2880; // 1.44 MB
        const SECTORS_PER_FAT: usize = 9;
        const RESERVED_SECTORS: usize = 1;
        const NUM_FATS: usize = 2;
        const ROOT_ENTRIES: usize = 224;

        let mut disk = vec![0u8; TOTAL_SECTORS * SECTOR_SIZE];

        // Boot sector (sector 0)
        let boot_sector = &mut disk[0..SECTOR_SIZE];
        boot_sector[0..3].copy_from_slice(&[0xEB, 0x3C, 0x90]); // JMP + NOP
        boot_sector[3..11].copy_from_slice(b"MSWIN4.1"); // OEM name
        boot_sector[11..13].copy_from_slice(&(SECTOR_SIZE as u16).to_le_bytes()); // Bytes per sector
        boot_sector[13] = 1; // Sectors per cluster
        boot_sector[14..16].copy_from_slice(&(RESERVED_SECTORS as u16).to_le_bytes());
        boot_sector[16] = NUM_FATS as u8;
        boot_sector[17..19].copy_from_slice(&(ROOT_ENTRIES as u16).to_le_bytes());
        boot_sector[19..21].copy_from_slice(&(TOTAL_SECTORS as u16).to_le_bytes());
        boot_sector[21] = 0xF0; // Media descriptor (removable disk)
        boot_sector[22..24].copy_from_slice(&(SECTORS_PER_FAT as u16).to_le_bytes());
        boot_sector[24..26].copy_from_slice(&18u16.to_le_bytes()); // Sectors per track
        boot_sector[26..28].copy_from_slice(&2u16.to_le_bytes()); // Number of heads
        boot_sector[510..512].copy_from_slice(&[0x55, 0xAA]); // Boot signature

        // FAT1 (starts at sector 1)
        let fat1_start = RESERVED_SECTORS * SECTOR_SIZE;
        let fat1_end = fat1_start + SECTORS_PER_FAT * SECTOR_SIZE;
        disk[fat1_start] = 0xF0; // Media descriptor
        disk[fat1_start + 1] = 0xFF;
        disk[fat1_start + 2] = 0xFF;

        // FAT2 (copy of FAT1)
        let fat2_start = fat1_end;
        let fat1_copy: Vec<u8> = disk[fat1_start..fat1_end].to_vec();
        disk[fat2_start..fat2_start + SECTORS_PER_FAT * SECTOR_SIZE]
            .copy_from_slice(&fat1_copy);

        // Root directory (initialized to zeros, which is valid)
        // Starts after FAT2
        let _root_start = fat2_start + SECTORS_PER_FAT * SECTOR_SIZE;

        // Data area (already zeroed)

        Ok(disk)
    }

    /// Create a VHD fixed disk footer
    ///
    /// Returns a 512-byte VHD footer with proper checksums
    pub fn create_vhd_footer(disk_size: u64, disk_type: u32) -> Vec<u8> {
        let mut footer = vec![0u8; 512];

        // Cookie: "conectix"
        footer[0..8].copy_from_slice(b"conectix");

        // Features (0x00000002 = reserved)
        footer[8..12].copy_from_slice(&0x00000002u32.to_be_bytes());

        // File format version (1.0)
        footer[12..16].copy_from_slice(&0x00010000u32.to_be_bytes());

        // Data offset (0xFFFFFFFFFFFFFFFF for fixed disk)
        footer[16..24].copy_from_slice(&0xFFFFFFFFFFFFFFFFu64.to_be_bytes());

        // Timestamp (seconds since 2000-01-01 00:00:00 UTC)
        footer[24..28].copy_from_slice(&0x00000000u32.to_be_bytes());

        // Creator application: "wi2k"
        footer[28..32].copy_from_slice(b"wi2k");

        // Creator version (Windows 2000)
        footer[32..36].copy_from_slice(&0x00050000u32.to_be_bytes());

        // Creator host OS (Windows)
        footer[36..40].copy_from_slice(b"Wi2k");

        // Original size
        footer[40..48].copy_from_slice(&disk_size.to_be_bytes());

        // Current size
        footer[48..56].copy_from_slice(&disk_size.to_be_bytes());

        // Disk geometry (CHS) - calculate based on size
        let total_sectors = disk_size / 512;
        let (cylinders, heads, sectors_per_track) = calculate_chs(total_sectors);
        footer[56..58].copy_from_slice(&cylinders.to_be_bytes());
        footer[58] = heads;
        footer[59] = sectors_per_track;

        // Disk type
        footer[60..64].copy_from_slice(&disk_type.to_be_bytes());

        // Checksum (calculated below)
        footer[64..68].copy_from_slice(&0u32.to_be_bytes()); // Placeholder

        // UUID (random for testing)
        footer[68..84].copy_from_slice(&[
            0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89, 0xAB,
            0xCD, 0xEF,
        ]);

        // Saved state (0 = not in saved state)
        footer[84] = 0;

        // Calculate checksum (ones' complement of sum of all bytes except checksum field)
        let mut checksum: u32 = 0;
        for i in 0..512 {
            if i < 64 || i >= 68 {
                checksum = checksum.wrapping_add(footer[i] as u32);
            }
        }
        let checksum = !checksum;
        footer[64..68].copy_from_slice(&checksum.to_be_bytes());

        footer
    }

    /// Calculate CHS geometry for a given sector count
    fn calculate_chs(total_sectors: u64) -> (u16, u8, u8) {
        let sectors_per_track = 63;
        let heads = 16;

        let total = total_sectors;
        let cylinder_times_heads = total / sectors_per_track as u64;
        let cylinders = cylinder_times_heads / heads as u64;

        let cylinders = cylinders.min(65535) as u16;
        (cylinders, heads as u8, sectors_per_track as u8)
    }

    /// Create an MBR with a single FAT32 partition
    ///
    /// Returns a 512-byte MBR with one bootable FAT32 LBA partition
    pub fn create_mbr_with_fat32_partition(partition_sectors: u32) -> Vec<u8> {
        let mut mbr = vec![0u8; 512];

        // Boot code area (optional, we'll leave it zeroed)

        // Partition entry 1 (offset 446)
        let entry_offset = 446;
        mbr[entry_offset] = 0x80; // Boot flag (bootable)
        mbr[entry_offset + 1] = 0x00; // CHS start head
        mbr[entry_offset + 2] = 0x02; // CHS start sector/cylinder
        mbr[entry_offset + 3] = 0x00;
        mbr[entry_offset + 4] = 0x0C; // Partition type: FAT32 LBA
        mbr[entry_offset + 5] = 0xFE; // CHS end head
        mbr[entry_offset + 6] = 0xFF; // CHS end sector/cylinder
        mbr[entry_offset + 7] = 0xFF;
        mbr[entry_offset + 8..entry_offset + 12].copy_from_slice(&2048u32.to_le_bytes()); // LBA start
        mbr[entry_offset + 12..entry_offset + 16]
            .copy_from_slice(&partition_sectors.to_le_bytes()); // LBA length

        // Boot signature
        mbr[510..512].copy_from_slice(&[0x55, 0xAA]);

        mbr
    }

    /// Create a complete fixed VHD with FAT12 filesystem
    ///
    /// Returns a VHD image containing:
    /// - FAT12 floppy image (1.44 MB)
    /// - VHD footer
    pub fn create_fixed_vhd_with_fat12() -> Result<Vec<u8>> {
        let fat12_image = create_fat12_floppy()?;
        let disk_size = fat12_image.len() as u64;

        // Create VHD footer
        let footer = create_vhd_footer(disk_size, 2); // Type 2 = Fixed disk

        // Combine data + footer
        let mut vhd = fat12_image;
        vhd.extend_from_slice(&footer);

        Ok(vhd)
    }

    /// Create a VHD with MBR and FAT32 partition
    ///
    /// Returns a VHD image containing:
    /// - MBR at sector 0
    /// - FAT32 partition starting at sector 2048
    /// - VHD footer
    pub fn create_vhd_with_mbr_fat32(total_mb: u64) -> Result<Vec<u8>> {
        const SECTOR_SIZE: usize = 512;
        let total_sectors = (total_mb * 1024 * 1024) / SECTOR_SIZE as u64;
        let partition_start_sector = 2048u64;
        let partition_sectors = (total_sectors - partition_start_sector) as u32;

        let mut disk = vec![0u8; (total_sectors * SECTOR_SIZE as u64) as usize];

        // Write MBR at sector 0
        let mbr = create_mbr_with_fat32_partition(partition_sectors);
        disk[0..SECTOR_SIZE].copy_from_slice(&mbr);

        // Write minimal FAT32 boot sector at partition start
        let partition_offset = (partition_start_sector * SECTOR_SIZE as u64) as usize;
        let fat32_boot = create_fat32_boot_sector(partition_sectors);
        disk[partition_offset..partition_offset + SECTOR_SIZE].copy_from_slice(&fat32_boot);

        // Create and append VHD footer
        let footer = create_vhd_footer(total_sectors * SECTOR_SIZE as u64, 2);
        disk.extend_from_slice(&footer);

        Ok(disk)
    }

    /// Create a minimal FAT32 boot sector
    fn create_fat32_boot_sector(total_sectors: u32) -> Vec<u8> {
        let mut boot = vec![0u8; 512];

        boot[0..3].copy_from_slice(&[0xEB, 0x58, 0x90]); // JMP + NOP
        boot[3..11].copy_from_slice(b"MSWIN4.1"); // OEM
        boot[11..13].copy_from_slice(&512u16.to_le_bytes()); // Bytes per sector
        boot[13] = 8; // Sectors per cluster (4KB clusters)
        boot[14..16].copy_from_slice(&32u16.to_le_bytes()); // Reserved sectors
        boot[16] = 2; // Number of FATs
        boot[17..19].copy_from_slice(&0u16.to_le_bytes()); // Root entries (0 for FAT32)
        boot[19..21].copy_from_slice(&0u16.to_le_bytes()); // Total sectors (use 32-bit field)
        boot[21] = 0xF8; // Media descriptor (hard disk)
        boot[22..24].copy_from_slice(&0u16.to_le_bytes()); // Sectors per FAT (use 32-bit field)
        boot[24..26].copy_from_slice(&63u16.to_le_bytes()); // Sectors per track
        boot[26..28].copy_from_slice(&255u16.to_le_bytes()); // Number of heads
        boot[28..32].copy_from_slice(&0u32.to_le_bytes()); // Hidden sectors
        boot[32..36].copy_from_slice(&total_sectors.to_le_bytes()); // Total sectors (32-bit)

        // FAT32 extended BPB
        let sectors_per_fat = ((total_sectors / 8) / 128).max(1); // Rough estimate
        boot[36..40].copy_from_slice(&sectors_per_fat.to_le_bytes()); // Sectors per FAT (32-bit)
        boot[40..42].copy_from_slice(&0u16.to_le_bytes()); // Flags
        boot[42..44].copy_from_slice(&0u16.to_le_bytes()); // Version
        boot[44..48].copy_from_slice(&2u32.to_le_bytes()); // Root cluster
        boot[48..50].copy_from_slice(&1u16.to_le_bytes()); // FSInfo sector
        boot[50..52].copy_from_slice(&6u16.to_le_bytes()); // Backup boot sector

        boot[510..512].copy_from_slice(&[0x55, 0xAA]); // Boot signature

        boot
    }
}
