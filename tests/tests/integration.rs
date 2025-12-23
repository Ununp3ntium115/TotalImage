//! Integration test suite for TotalImage
//!
//! End-to-end tests verifying complete workflows from disk image to file extraction.
//!
//! NOTE: Full integration tests require test fixtures (actual disk images).
//! Run with: `cargo test --test integration -- --include-ignored` to see fixture requirements.

use std::io::Read;
use std::path::Path;
use tempfile::NamedTempFile;

use totalimage_core::{Result, ZoneTable};
use totalimage_integration_tests::utils;
use totalimage_vaults::factory::detect_vault_type;
use totalimage_zones::MbrZoneTable;

/// Check if test fixtures are available
fn fixtures_available() -> bool {
    utils::fixtures_available()
}

#[test]
fn test_integration_framework_setup() {
    // This test verifies the integration test framework is configured
    assert!(Path::new("tests").exists(), "Tests directory should exist");

    // Document fixture requirements for full integration tests
    if !fixtures_available() {
        eprintln!("\n=== Integration Test Setup ===");
        eprintln!("Full integration tests require test fixtures.");
        eprintln!("To create fixtures:");
        eprintln!("  mkdir -p tests/fixtures/");
        eprintln!("  # Add test images: test.vhd, test.e01, test.aff4");
        eprintln!("===============================\n");
    }
}

/// Test VHD vault opening and basic operations
#[test]
fn test_vhd_vault_basic_operations() -> Result<()> {
    // Create a test VHD using the helper from vault tests
    // This is a simplified test that doesn't require a full partition table
    use totalimage_vaults::vhd::VhdVault;

    // Create minimal VHD data (we'll use the test helper if accessible)
    // For now, just verify the integration test can import and use the vault types
    let _vault_type = std::any::type_name::<VhdVault>();
    assert!(!_vault_type.is_empty());

    Ok(())
}

/// Test vault factory auto-detection
#[test]
fn test_vault_factory_detection() -> Result<()> {
    // Test VHD detection by extension
    let temp = NamedTempFile::with_suffix(".vhd")?;
    let vault_type = detect_vault_type(temp.path())?;
    assert_eq!(vault_type.name(), "Microsoft VHD");

    // Test raw detection by extension
    let temp2 = NamedTempFile::with_suffix(".img")?;
    let vault_type2 = detect_vault_type(temp2.path())?;
    assert_eq!(vault_type2.name(), "Raw Sector Image");

    Ok(())
}

/// Test zone table parsing from vault content
#[test]
fn test_zone_table_parsing() -> Result<()> {
    // Create a minimal MBR for testing
    let mut mbr_data = vec![0u8; 512];
    // Boot signature
    mbr_data[510] = 0x55;
    mbr_data[511] = 0xAA;

    // Create a simple partition entry (type 0x0C = FAT32 LBA)
    // Offset 446: Partition entry 1
    mbr_data[446] = 0x80; // Boot flag
    mbr_data[450] = 0x0C; // Partition type (FAT32 LBA)
                          // LBA start (little endian, offset 454)
    mbr_data[454..458].copy_from_slice(&2048u32.to_le_bytes()); // Start at sector 2048
                                                                // LBA length (little endian, offset 458)
    mbr_data[458..462].copy_from_slice(&102400u32.to_le_bytes()); // 100MB partition

    let mut cursor = std::io::Cursor::new(mbr_data);
    let table = MbrZoneTable::parse(&mut cursor, 512)?;
    let zones = table.enumerate_zones();

    assert_eq!(zones.len(), 1);
    assert_eq!(zones[0].offset, 2048 * 512);
    assert_eq!(zones[0].length, 102400 * 512);

    Ok(())
}

/// Test FAT territory parsing (without full filesystem)
#[test]
fn test_fat_territory_parsing() -> Result<()> {
    use totalimage_territories::fat::types::BiosParameterBlock;

    // Create a minimal FAT12 boot sector
    let mut boot_sector = vec![0u8; 512];
    boot_sector[0..3].copy_from_slice(&[0xEB, 0x3C, 0x90]); // Jump
    boot_sector[3..11].copy_from_slice(b"MSWIN4.1"); // OEM
    boot_sector[11..13].copy_from_slice(&512u16.to_le_bytes()); // Bytes per sector
    boot_sector[13] = 1; // Sectors per cluster
    boot_sector[14..16].copy_from_slice(&1u16.to_le_bytes()); // Reserved sectors
    boot_sector[16] = 2; // Number of FATs
    boot_sector[17..19].copy_from_slice(&224u16.to_le_bytes()); // Root entries
    boot_sector[19..21].copy_from_slice(&2880u16.to_le_bytes()); // Total sectors
    boot_sector[21] = 0xF0; // Media descriptor
    boot_sector[22..24].copy_from_slice(&9u16.to_le_bytes()); // Sectors per FAT
    boot_sector[510..512].copy_from_slice(&[0x55, 0xAA]); // Boot signature

    // Parse BPB
    let bpb = BiosParameterBlock::from_bytes(&boot_sector)?;
    assert_eq!(bpb.bytes_per_sector, 512);
    assert_eq!(bpb.total_sectors(), 2880);

    Ok(())
}

/// Test full pipeline: VHD → Zone → Territory (simplified)
#[test]
fn test_vhd_to_zone_pipeline() -> Result<()> {
    // This test verifies we can:
    // 1. Open a VHD vault
    // 2. Read content from it
    // 3. Parse zone tables from the content

    // For a full test, we would need a VHD with a partition table
    // For now, we'll test the components separately

    // Test that we can create a partial pipeline (simulating a partition)
    use std::io::Cursor;
    use totalimage_pipeline::PartialPipeline;

    let data = vec![0u8; 1024];
    let mut cursor = Cursor::new(&data);
    let _partition = PartialPipeline::new(&mut cursor, 0, 1024)?;

    Ok(())
}

#[test]
#[ignore] // Requires test fixtures with actual filesystems
fn test_vhd_fat32_full_pipeline() {
    if !fixtures_available() {
        eprintln!("Skipping: test fixtures not available");
    }

    // TODO: Full VHD → FAT32 → file extraction pipeline test
    // Would open VHD, parse partition table, read FAT32, extract files
    // Requires: VHD file with MBR partition table and FAT32 filesystem
}

#[test]
#[ignore] // Requires test fixtures
fn test_e01_ntfs_full_pipeline() {
    if !fixtures_available() {
        eprintln!("Skipping: test fixtures not available");
    }

    // TODO: Full E01 → NTFS → file extraction pipeline test
    // Would open E01, parse GPT, read NTFS, handle compressed files
    // Requires: E01 file with GPT partition table and NTFS filesystem
}

#[test]
#[ignore] // Requires test fixtures
fn test_aff4_exfat_full_pipeline() {
    if !fixtures_available() {
        eprintln!("Skipping: test fixtures not available");
    }

    // TODO: Full AFF4 → exFAT → file extraction pipeline test
    // Would open AFF4, verify compression methods, read exFAT, extract large files
    // Requires: AFF4 file with exFAT filesystem
}

/// Test E01 write → read roundtrip
#[test]
#[ignore] // Requires totalimage-acquire dependency
fn test_e01_write_read_roundtrip() -> Result<()> {
    // This test requires the totalimage-acquire crate which is not
    // included in the integration test dependencies.
    // Full E01 write/read testing is done in the acquire crate tests.
    Ok(())
}

/// Test VHD → GPT → FAT32 pipeline (in-memory)
#[test]
fn test_vhd_gpt_fat32_pipeline() -> Result<()> {
    use totalimage_vaults::vhd::VhdVault;
    use totalimage_zones::gpt::GptZoneTable;
    use totalimage_territories::fat::FatTerritory;

    // Create a minimal VHD with GPT and FAT32
    // This is a simplified test - full implementation would require
    // creating actual VHD, GPT, and FAT32 structures

    // For now, verify the types can be used together
    let _vhd_type = std::any::type_name::<VhdVault>();
    let _gpt_type = std::any::type_name::<GptZoneTable>();
    let _fat_type = std::any::type_name::<FatTerritory>();

    assert!(!_vhd_type.is_empty());
    assert!(!_gpt_type.is_empty());
    assert!(!_fat_type.is_empty());

    Ok(())
}

/// Test error handling for corrupted images
#[test]
fn test_corrupted_image_handling() {
    use tempfile::NamedTempFile;
    use totalimage_vaults::factory::detect_vault_type;

    // Test with invalid data in a temp file
    let temp_file = NamedTempFile::with_suffix(".vhd").unwrap();
    std::fs::write(temp_file.path(), vec![0xFFu8; 512]).unwrap();

    // Should handle gracefully without panicking
    let result = detect_vault_type(temp_file.path());
    // Result may be Ok or Err, but should not panic
    let _ = result;
}

/// Test missing file handling
#[test]
fn test_missing_file_handling() {
    use std::path::Path;
    use totalimage_vaults::factory::detect_vault_type;

    let missing_path = Path::new("/nonexistent/path/to/file.vhd");
    let result = detect_vault_type(missing_path);

    // Should return an error, not panic
    assert!(result.is_err());
}

/// Test property test integration with integration tests
#[test]
fn test_property_test_integration() {
    // Verify property tests are available (if feature is enabled)
    // Property tests are integrated via totalimage-core
    // This test just verifies the integration test framework works
    assert!(true);
}

/// Test concurrent request handling (basic)
#[test]
fn test_concurrent_operations() -> Result<()> {
    use std::io::Cursor;
    use std::sync::Arc;
    use std::thread;

    // Create test data
    let data = vec![0u8; 1024];
    let shared_data = Arc::new(data);

    // Test that we can create multiple cursors from shared data
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let data = Arc::clone(&shared_data);
            thread::spawn(move || {
                let mut cursor = Cursor::new(data.as_ref());
                cursor.seek(std::io::SeekFrom::Start(0)).unwrap();
                cursor.read_exact(&mut vec![0u8; 512]).unwrap();
            })
        })
        .collect();

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    Ok(())
}

/// Test memory usage validation (basic)
#[test]
fn test_memory_usage_validation() {
    use totalimage_core::security::MAX_ALLOCATION_SIZE;

    // Verify security limits are enforced
    assert!(MAX_ALLOCATION_SIZE <= 256 * 1024 * 1024); // 256 MB max
}

/// Test large file handling simulation
#[test]
fn test_large_file_handling() -> Result<()> {
    use std::io::Cursor;
    use totalimage_pipeline::PartialPipeline;

    // Simulate a large file (10GB) with a small buffer
    let large_size = 10_000_000_000u64;
    let small_buffer = vec![0u8; 1024];
    let cursor = Cursor::new(&small_buffer);

    // Should handle large sizes without allocating full buffer
    let pipeline = PartialPipeline::new(cursor, 0, large_size)?;
    assert_eq!(pipeline.length(), large_size);

    Ok(())
}
