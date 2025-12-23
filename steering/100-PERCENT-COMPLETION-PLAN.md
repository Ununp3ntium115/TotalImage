# 100% Completion Plan - TotalImage Project
**Created:** December 21, 2025  
**Purpose:** Comprehensive plan to achieve 100% completion in all areas  
**Current Status:** ~95% complete  
**Target:** 100% in all categories

---

## Executive Summary

This document provides a detailed, actionable plan to achieve 100% completion across all areas of the TotalImage project. It identifies every remaining gap, provides implementation steps with **comprehensive pseudocode for every task**, and tracks progress toward complete feature parity and production readiness.

### Document Structure

This expanded plan includes:
- **Detailed pseudocode** for every implementation step
- **Complete test implementations** with full code examples
- **Step-by-step algorithms** for complex features
- **Error handling patterns** for all scenarios
- **Integration test examples** with full workflows
- **API designs** with function signatures and logic

Every task in this document now includes:
1. **Exact function signatures** and data structures
2. **Step-by-step algorithm** pseudocode
3. **Error handling** patterns
4. **Test implementations** with assertions
5. **Edge case handling** examples

### Current State Assessment

| Category | Current | Target | Gap | Status |
|----------|---------|--------|-----|--------|
| **P0 Security Issues** | 3/3 (100%) | 100% | 0 | ✅ Complete |
| **P1 Security Issues** | 14/14 (100%) | 100% | 0 | ✅ Complete |
| **P2 Features** | 5/6 (83%) | 100% | 1 item | 🔄 In Progress |
| **P3 Features** | 0/6 (0%) | 100% | 6 items | ⏳ Not Started |
| **Unit Tests** | 322 tests | 414+ tests | +92 tests | ⚠️ In Progress |
| **Integration Tests** | 6 tests | 20+ tests | +14 tests | ⚠️ In Progress |
| **Property Tests** | 0 tests | 10+ tests | +10 tests | ❌ Not Started |
| **WinPE USB** | 60% | 100% | 3 tasks | 🔄 In Progress |
| **Overall Completion** | ~95% | 100% | ~5% | 🔄 In Progress |

---

## Part 1: P2 Feature Completion (83% → 100%)

### Current Status
- ✅ **11.1: GPT Backup Header Validation** - Complete
- ✅ **11.2: ISO Joliet Extension** - Complete
- ✅ **11.3: E01 Write Support** - Complete
- ✅ **11.4: CLI Hash Command** - Complete
- ✅ **11.5: exFAT/ISO Extraction** - Complete
- ❌ **11.6: Property-Based Testing** - Not Started

### Remaining Task: 11.6 Property-Based Testing

**Effort:** 8 hours  
**Priority:** P2  
**Status:** ❌ Not Started

#### Implementation Steps

**11.6.1: Set Up Proptest Framework (2 hours)**

1. **Add proptest dependency** (15 min)
   
   **Step 1.1: Update workspace Cargo.toml**
   ```toml
   # Cargo.toml (workspace root)
   [workspace.dependencies]
   # ... existing dependencies ...
   proptest = "1.4"
   ```
   
   **Step 1.2: Add to each crate's Cargo.toml**
   ```toml
   # crates/totalimage-vaults/Cargo.toml
   [dev-dependencies]
   proptest.workspace = true
   
   # crates/totalimage-zones/Cargo.toml
   [dev-dependencies]
   proptest.workspace = true
   
   # crates/totalimage-territories/Cargo.toml
   [dev-dependencies]
   proptest.workspace = true
   ```

2. **Create property test infrastructure** (1.5 hours)
   
   **Step 2.1: Create proptest utilities module**
   ```rust
   // crates/totalimage-core/src/proptest.rs
   //! Property-based testing utilities for TotalImage
   //!
   //! This module provides common strategies and helpers for property-based testing
   //! across all TotalImage crates using the proptest framework.
   
   use proptest::prelude::*;
   
   /// Strategy for generating valid sector sizes
   pub fn sector_size_strategy() -> impl Strategy<Value = u32> {
       prop_oneof![
           Just(512u32),   // Most common
           Just(4096u32),  // 4K sectors
           (256u32..=8192u32).prop_filter("Must be power of 2", |&s| s.is_power_of_two()),
       ]
   }
   
   /// Strategy for generating valid disk sizes (in bytes)
   pub fn disk_size_strategy() -> impl Strategy<Value = u64> {
       (1u64..=2_000_000_000_000u64) // 1 byte to 2TB
           .prop_filter("Must be multiple of 512", |&s| s % 512 == 0)
   }
   
   /// Strategy for generating valid VHD disk types
   pub fn vhd_disk_type_strategy() -> impl Strategy<Value = u32> {
       prop_oneof![
           Just(2u32),  // Fixed
           Just(3u32),  // Dynamic
           Just(4u32),  // Differencing
       ]
   }
   
   /// Strategy for generating valid partition counts
   pub fn partition_count_strategy() -> impl Strategy<Value = u32> {
       (0u32..=128u32) // GPT supports up to 128 partitions
   }
   
   /// Strategy for generating valid FAT types
   pub fn fat_type_strategy() -> impl Strategy<Value = u8> {
       prop_oneof![
           Just(1u8),   // FAT12
           Just(6u8),   // FAT16
           Just(11u8),  // FAT32
           Just(12u8),  // FAT32 (LBA)
       ]
   }
   
   /// Helper to create proptest configuration
   pub fn proptest_config() -> proptest::test_runner::Config {
       proptest::test_runner::Config::with_cases(1000) // Run 1000 test cases
   }
   ```
   
   **Step 2.2: Export proptest utilities**
   ```rust
   // crates/totalimage-core/src/lib.rs
   #[cfg(test)]
   pub mod proptest;
   ```
   
   **Step 2.3: Create proptest configuration file**
   ```toml
   # proptest.toml (workspace root)
   # Proptest configuration for property-based testing
   
   [default]
   cases = 1000
   max_local_rejects = 65536
   max_global_rejects = 65536
   failure_persistence = "proptest-regressions"
   ```

**11.6.2: Implement Property Tests (6 hours)**

1. **VHD Footer Roundtrip Test** (1.5 hours)
   
   **Location:** `crates/totalimage-vaults/src/vhd/mod.rs`
   
   **Pseudocode Implementation:**
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;
       use proptest::prelude::*;
       use totalimage_core::proptest::*;
       
       proptest! {
           #![proptest_config(proptest_config())]
           
           #[test]
           fn test_vhd_footer_roundtrip(
               current_size in disk_size_strategy(),
               original_size in disk_size_strategy(),
               disk_type in vhd_disk_type_strategy(),
               timestamp in 0u32..=u32::MAX,
           ) {
               // Step 1: Create a VHD footer with generated properties
               let mut footer = VhdFooter {
                   cookie: *VhdFooter::COOKIE,
                   features: 0x00000002, // No features
                   version: 0x00010000, // Version 1.0
                   data_offset: 0xFFFFFFFFFFFFFFFF, // Dynamic VHDs
                   timestamp,
                   creator_app: *b"vpc ", // Virtual PC
                   creator_version: 0x00050000,
                   creator_os: 0x00020000, // Windows
                   original_size,
                   current_size,
                   geometry: DiskGeometry {
                       cylinders: 0,
                       heads: 0,
                       sectors: 0,
                   },
                   disk_type: VhdType::from_u32(disk_type).unwrap(),
                   checksum: 0, // Will be calculated
                   uuid: [0u8; 16],
                   saved_state: 0,
                   reserved: [0u8; 427],
               };
               
               // Step 2: Calculate and set checksum
               footer.checksum = footer.calculate_checksum();
               
               // Step 3: Serialize footer to bytes
               let mut bytes = [0u8; VhdFooter::SIZE];
               footer.serialize(&mut bytes);
               
               // Step 4: Parse footer from bytes
               let parsed = VhdFooter::parse(&bytes)
                   .expect("Should parse valid footer");
               
               // Step 5: Verify all properties match
               prop_assert_eq!(parsed.current_size, current_size);
               prop_assert_eq!(parsed.original_size, original_size);
               prop_assert_eq!(parsed.disk_type as u32, disk_type);
               prop_assert_eq!(parsed.timestamp, timestamp);
               prop_assert_eq!(parsed.checksum, footer.checksum);
               
               // Step 6: Verify checksum is valid
               prop_assert!(parsed.verify_checksum(), "Checksum should be valid");
           }
           
           #[test]
           fn test_vhd_footer_edge_cases(
               size in prop_oneof![
                   Just(512u64),           // Minimum size
                   Just(2_000_000_000_000u64), // 2TB (max practical)
                   Just(u64::MAX - 512),   // Near max
               ],
           ) {
               // Test edge case sizes
               let footer = create_test_footer(size, 2); // Fixed disk
               let mut bytes = [0u8; VhdFooter::SIZE];
               footer.serialize(&mut bytes);
               let parsed = VhdFooter::parse(&bytes).unwrap();
               prop_assert_eq!(parsed.current_size, size);
           }
       }
       
       // Helper function to create test footer
       fn create_test_footer(size: u64, disk_type: u32) -> VhdFooter {
           let mut footer = VhdFooter {
               // ... initialize with defaults ...
               current_size: size,
               disk_type: VhdType::from_u32(disk_type).unwrap(),
               // ... other fields ...
           };
           footer.checksum = footer.calculate_checksum();
           footer
       }
   }
   ```
   
   **Test Properties:**
   - Serialize → Parse → Verify equality (roundtrip)
   - Checksum calculation and verification
   - Edge cases: min size (512), max size (2TB), boundary values
   - All footer fields preserved through serialization

2. **GPT Header Roundtrip Test** (1.5 hours)
   
   **Location:** `crates/totalimage-zones/src/gpt/mod.rs`
   
   **Pseudocode Implementation:**
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;
       use proptest::prelude::*;
       use totalimage_core::proptest::*;
       
       proptest! {
           #![proptest_config(proptest_config())]
           
           #[test]
           fn test_gpt_header_roundtrip(
               partition_count in partition_count_strategy(),
               sector_size in sector_size_strategy(),
               disk_size in disk_size_strategy(),
           ) {
               // Step 1: Create GPT header with generated properties
               let mut header = GptHeader {
                   signature: *b"EFI PART",
                   revision: 0x00010000, // GPT revision 1.0
                   header_size: 92,
                   header_crc32: 0, // Will be calculated
                   reserved: 0,
                   current_lba: 1,
                   backup_lba: (disk_size / sector_size as u64) - 1,
                   first_usable_lba: 34,
                   last_usable_lba: (disk_size / sector_size as u64) - 34,
                   disk_guid: [0u8; 16], // Will be generated
                   partition_entry_lba: 2,
                   num_partition_entries: partition_count,
                   size_of_partition_entry: 128,
                   partition_array_crc32: 0, // Will be calculated
               };
               
               // Step 2: Generate random disk GUID
               use uuid::Uuid;
               let uuid = Uuid::new_v4();
               header.disk_guid.copy_from_slice(uuid.as_bytes());
               
               // Step 3: Calculate CRC32 for header
               header.header_crc32 = header.calculate_header_crc32();
               
               // Step 4: Serialize header to bytes
               let mut bytes = vec![0u8; sector_size as usize];
               header.serialize(&mut bytes);
               
               // Step 5: Parse header from bytes
               let parsed = GptHeader::from_bytes(&bytes)
                   .expect("Should parse valid GPT header");
               
               // Step 6: Verify all properties match
               prop_assert_eq!(parsed.num_partition_entries, partition_count);
               prop_assert_eq!(parsed.current_lba, 1);
               prop_assert_eq!(parsed.disk_guid, header.disk_guid);
               
               // Step 7: Verify CRC32 is valid
               prop_assert!(parsed.verify_header_crc32(&bytes), 
                   "Header CRC32 should be valid");
           }
           
           #[test]
           fn test_gpt_header_edge_cases(
               count in prop_oneof![
                   Just(0u32),    // Zero partitions
                   Just(128u32),  // Max partitions
               ],
           ) {
               // Test edge cases for partition count
               let header = create_test_gpt_header(count);
               let mut bytes = vec![0u8; 512];
               header.serialize(&mut bytes);
               let parsed = GptHeader::from_bytes(&bytes).unwrap();
               prop_assert_eq!(parsed.num_partition_entries, count);
           }
       }
   }
   ```
   
   **Test Properties:**
   - Create → Serialize → Parse → Verify (roundtrip)
   - CRC32 calculation and verification
   - Partition count: 0 to 128
   - Disk GUID preservation
   - Edge cases: zero partitions, max partitions (128)

3. **FAT BPB Roundtrip Test** (1.5 hours)
   
   **Location:** `crates/totalimage-territories/src/fat/mod.rs`
   
   **Pseudocode Implementation:**
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;
       use proptest::prelude::*;
       use totalimage_core::proptest::*;
       
       proptest! {
           #![proptest_config(proptest_config())]
           
           #[test]
           fn test_fat_bpb_roundtrip(
               bytes_per_sector in prop_oneof![
                   Just(512u16),
                   Just(1024u16),
                   Just(2048u16),
                   Just(4096u16),
               ],
               sectors_per_cluster in (1u8..=128u8).prop_filter(
                   "Must be power of 2", |&s| s.is_power_of_two()
               ),
               fat_type_val in fat_type_strategy(),
           ) {
               // Step 1: Calculate cluster count to determine FAT type
               let total_sectors = 100000u32; // Large enough for FAT32
               let cluster_count = (total_sectors as u32) / (sectors_per_cluster as u32);
               
               // Step 2: Determine actual FAT type based on cluster count
               let fat_type = if cluster_count < 4085 {
                   FatType::Fat12
               } else if cluster_count < 65525 {
                   FatType::Fat16
               } else {
                   FatType::Fat32
               };
               
               // Step 3: Create BPB with generated properties
               let mut bpb = BiosParameterBlock {
                   jump_boot: [0xEB, 0x3C, 0x90],
                   oem_name: *b"MSWIN4.1",
                   bytes_per_sector,
                   sectors_per_cluster,
                   reserved_sector_count: 1,
                   num_fats: 2,
                   root_entry_count: if fat_type == FatType::Fat32 { 0 } else { 512 },
                   total_sectors_16: if total_sectors < 65536 { total_sectors as u16 } else { 0 },
                   media: 0xF8,
                   sectors_per_fat_16: 0, // Will be calculated
                   sectors_per_track: 63,
                   num_heads: 255,
                   hidden_sectors: 0,
                   total_sectors_32: if total_sectors >= 65536 { total_sectors } else { 0 },
                   sectors_per_fat_32: 0, // Will be calculated
                   fat_type,
                   // ... other fields ...
               };
               
               // Step 4: Calculate FAT size based on FAT type
               if fat_type == FatType::Fat32 {
                   bpb.sectors_per_fat_32 = calculate_fat32_size(cluster_count, bytes_per_sector);
               } else {
                   bpb.sectors_per_fat_16 = calculate_fat16_size(cluster_count, bytes_per_sector);
               }
               
               // Step 5: Serialize BPB to bytes
               let mut boot_sector = vec![0u8; bytes_per_sector as usize];
               bpb.serialize(&mut boot_sector);
               
               // Step 6: Parse BPB from bytes
               let parsed = BiosParameterBlock::from_bytes(&boot_sector)
                   .expect("Should parse valid BPB");
               
               // Step 7: Verify all properties match
               prop_assert_eq!(parsed.bytes_per_sector, bytes_per_sector);
               prop_assert_eq!(parsed.sectors_per_cluster, sectors_per_cluster);
               prop_assert_eq!(parsed.fat_type, fat_type);
               
               // Step 8: Verify FAT size calculations
               if fat_type == FatType::Fat32 {
                   prop_assert_eq!(parsed.sectors_per_fat_32, bpb.sectors_per_fat_32);
               } else {
                   prop_assert_eq!(parsed.sectors_per_fat_16, bpb.sectors_per_fat_16);
               }
           }
           
           #[test]
           fn test_fat_type_boundaries(
               cluster_count in prop_oneof![
                   Just(4084u32),   // FAT12 boundary
                   Just(4085u32),   // FAT16 start
                   Just(65524u32),  // FAT16 boundary
                   Just(65525u32),  // FAT32 start
               ],
           ) {
               // Test FAT type boundaries
               let bpb = create_test_bpb_for_clusters(cluster_count);
               prop_assert!(matches!(
                   bpb.fat_type,
                   FatType::Fat12 | FatType::Fat16 | FatType::Fat32
               ));
           }
       }
       
       // Helper functions
       fn calculate_fat32_size(clusters: u32, sector_size: u16) -> u32 {
           // FAT32: 4 bytes per entry
           let fat_bytes = clusters * 4;
           ((fat_bytes + sector_size as u32 - 1) / sector_size as u32)
       }
       
       fn calculate_fat16_size(clusters: u32, sector_size: u16) -> u16 {
           // FAT16: 2 bytes per entry
           let fat_bytes = clusters * 2;
           ((fat_bytes + sector_size as u32 - 1) / sector_size as u32) as u16
       }
   }
   ```
   
   **Test Properties:**
   - Create BPB → Serialize → Parse → Verify (roundtrip)
   - Sector size: 512, 1024, 2048, 4096
   - Cluster size: powers of 2 (1-128)
   - FAT type determination: FAT12/16/32 boundaries
   - FAT size calculations
   - Edge cases: FAT12/16/32 boundaries (4085, 65525 clusters)

4. **MBR Partition Table Test** (1.5 hours)
   
   **Location:** `crates/totalimage-zones/src/mbr/mod.rs`
   
   **Pseudocode Implementation:**
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;
       use proptest::prelude::*;
       use totalimage_core::proptest::*;
       
       proptest! {
           #![proptest_config(proptest_config())]
           
           #[test]
           fn test_mbr_roundtrip(
               partition_count in 0u8..=4u8,
               sector_size in sector_size_strategy(),
               disk_size in disk_size_strategy(),
           ) {
               // Step 1: Create MBR with generated partition count
               let mut mbr = MbrZoneTable {
                   zones: Vec::new(),
                   disk_signature: 0x12345678,
                   boot_signature: MbrZoneTable::BOOT_SIGNATURE,
               };
               
               // Step 2: Create partitions
               let partition_size = disk_size / partition_count.max(1) as u64;
               for i in 0..partition_count {
                   let partition = create_test_partition(
                       i,
                       partition_size,
                       sector_size,
                   );
                   mbr.zones.push(partition);
               }
               
               // Step 3: Serialize MBR to bytes
               let mut mbr_bytes = vec![0u8; MbrZoneTable::MBR_SIZE];
               mbr.serialize(&mut mbr_bytes);
               
               // Step 4: Parse MBR from bytes
               let mut cursor = std::io::Cursor::new(&mbr_bytes);
               let parsed = MbrZoneTable::parse(&mut cursor, sector_size)
                   .expect("Should parse valid MBR");
               
               // Step 5: Verify partition count matches
               prop_assert_eq!(parsed.zones.len(), partition_count as usize);
               
               // Step 6: Verify each partition
               for (i, zone) in parsed.zones.iter().enumerate() {
                   prop_assert_eq!(zone.index, i as u32);
                   prop_assert!(zone.length > 0, "Partition should have size");
               }
               
               // Step 7: Verify boot signature
               prop_assert_eq!(
                   u16::from_le_bytes([
                       mbr_bytes[MbrZoneTable::BOOT_SIGNATURE_OFFSET as usize],
                       mbr_bytes[MbrZoneTable::BOOT_SIGNATURE_OFFSET as usize + 1],
                   ]),
                   MbrZoneTable::BOOT_SIGNATURE
               );
           }
           
           #[test]
           fn test_mbr_extended_partition(
               primary_count in 1u8..=3u8,
               extended_count in 1u8..=10u8, // Extended partitions in chain
           ) {
               // Test extended partition handling
               let mbr = create_mbr_with_extended(primary_count, extended_count);
               let mut bytes = vec![0u8; MbrZoneTable::MBR_SIZE];
               mbr.serialize(&mut bytes);
               
               let mut cursor = std::io::Cursor::new(&bytes);
               let parsed = MbrZoneTable::parse(&mut cursor, 512).unwrap();
               
               // Should find all partitions (primary + extended chain)
               prop_assert!(parsed.zones.len() >= primary_count as usize);
           }
       }
       
       // Helper functions
       fn create_test_partition(index: u8, size: u64, sector_size: u32) -> Zone {
           Zone {
               index: index as u32,
               offset: (index as u64) * size,
               length: size,
               partition_type: MbrPartitionType::WindowsFat32,
               // ... other fields ...
           }
       }
       
       fn create_mbr_with_extended(primary: u8, extended: u8) -> MbrZoneTable {
           // Create MBR with primary partitions and extended partition chain
           // ... implementation ...
       }
   }
   ```
   
   **Test Properties:**
   - Create MBR → Serialize → Parse → Verify (roundtrip)
   - Partition count: 0 to 4 primary partitions
   - CHS addressing validation
   - Extended partition chain handling
   - Boot signature verification
   - Edge cases: 4 partitions, extended partition chains

**Acceptance Criteria:**
- [ ] Proptest configured in workspace
- [ ] 4+ property tests implemented
- [ ] All property tests passing
- [ ] Tests discover edge cases
- [ ] Documentation updated

**Dependencies:** None  
**Risk:** Medium (may reveal bugs)

---

## Part 2: Test Coverage Completion (322 → 414+ tests)

### Current Status
- **Unit Tests:** 322 tests passing
- **Integration Tests:** 6 tests passing (framework complete)
- **Property Tests:** 0 tests
- **Fuzzing:** 5 targets created ✅

### Gap Analysis: +92 Unit Tests Needed

#### Test Distribution by Crate

| Crate | Current | Target | Gap | Priority |
|-------|---------|--------|-----|----------|
| totalimage-core | 80 | 85 | +5 | P2 |
| totalimage-pipeline | 9 | 12 | +3 | P2 |
| totalimage-vaults | 103 | 120 | +17 | P2 |
| totalimage-zones | 34 | 42 | +8 | P2 |
| totalimage-territories | 0 | 15 | +15 | P2 |
| totalimage-acquire | 3 | 25 | +22 | P2 |
| totalimage-cli | 2 | 8 | +6 | P2 |
| totalimage-web | 8 | 12 | +4 | P2 |
| totalimage-mcp | 54 | 60 | +6 | P2 |
| fire-marshal | 2 | 5 | +3 | P2 |

#### Implementation Plan

**2.1: E01 Writer Tests (8 hours)**

**Location:** `crates/totalimage-acquire/src/e01_writer.rs`

**Detailed Test Implementation:**

**Test 2.1.1: Multi-Segment File Creation**
```rust
#[test]
fn test_e01_multi_segment_creation() {
    // Pseudocode:
    // 1. Create E01Writer with max_segment_size = 100MB
    // 2. Write sectors until segment size exceeded
    // 3. Verify .E01 file created
    // 4. Verify .E02 file created
    // 5. Verify both files are valid E01 format
    
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("test");
    
    let config = E01WriterConfig {
        max_segment_size: 100_000_000, // 100MB
        bytes_per_sector: 512,
        sectors_per_chunk: 64,
        ..Default::default()
    };
    
    let mut writer = E01Writer::new(&output_path, config).unwrap();
    
    // Write enough data to trigger segment split
    let sector_data = vec![0xAA; 512];
    let sectors_needed = (100_000_000 / 512) + 1; // Exceed 100MB
    
    for _ in 0..sectors_needed {
        writer.write_sector(&sector_data).unwrap();
    }
    
    writer.finalize().unwrap();
    
    // Verify both segments exist
    assert!(temp_dir.path().join("test.E01").exists());
    assert!(temp_dir.path().join("test.E02").exists());
    
    // Verify both are readable
    let vault1 = E01Vault::open(&temp_dir.path().join("test.E01")).unwrap();
    let vault2 = E01Vault::open(&temp_dir.path().join("test.E02")).unwrap();
    
    assert!(vault1.length() > 0);
    assert!(vault2.length() > 0);
}
```

**Test 2.1.2: Compression Verification**
```rust
#[test]
fn test_e01_compression_verification() {
    // Pseudocode:
    // 1. Create E01 with compression enabled
    // 2. Write highly compressible data (zeros)
    // 3. Write non-compressible data (random)
    // 4. Verify compressed chunks are smaller
    // 5. Verify decompression works correctly
    
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("test");
    
    let config = E01WriterConfig {
        compression: 1, // deflate
        bytes_per_sector: 512,
        sectors_per_chunk: 2,
        ..Default::default()
    };
    
    let mut writer = E01Writer::new(&output_path, config).unwrap();
    
    // Write compressible data (zeros)
    let zeros = vec![0u8; 512];
    for _ in 0..4 {
        writer.write_sector(&zeros).unwrap();
    }
    
    writer.finalize().unwrap();
    
    // Read back and verify
    let vault = E01Vault::open(&temp_dir.path().join("test.E01")).unwrap();
    let mut buffer = vec![0u8; 2048];
    vault.content().read_exact(&mut buffer).unwrap();
    
    assert_eq!(buffer, vec![0u8; 2048]);
}
```

**Test 2.1.3: Hash Calculation Verification**
```rust
#[test]
fn test_e01_hash_calculation() {
    // Pseudocode:
    // 1. Create E01 with known data
    // 2. Calculate expected MD5 hash
    // 3. Read E01 and get stored hash
    // 4. Verify hashes match
    
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("test");
    
    let mut writer = E01Writer::new(&output_path, Default::default()).unwrap();
    
    // Write known data
    let known_data = b"Hello, TotalImage!".repeat(28); // 512 bytes
    writer.write_sector(&known_data).unwrap();
    writer.finalize().unwrap();
    
    // Calculate expected MD5
    use md5::{Digest, Md5};
    let mut hasher = Md5::new();
    hasher.update(&known_data);
    let expected_hash = hasher.finalize();
    
    // Read E01 hash
    let vault = E01Vault::open(&temp_dir.path().join("test.E01")).unwrap();
    let stored_hash = vault.hash().unwrap();
    
    assert_eq!(stored_hash.md5_hash, expected_hash.into());
}
```

**Test 2.1.4: Roundtrip Test (Write → Read)**
```rust
#[test]
fn test_e01_write_read_roundtrip() {
    // Pseudocode:
    // 1. Create random test data
    // 2. Write to E01
    // 3. Read from E01
    // 4. Verify data matches exactly
    
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("test");
    
    // Generate test data
    let mut test_data = Vec::new();
    for i in 0..100 {
        test_data.extend_from_slice(&vec![i as u8; 512]);
    }
    
    // Write to E01
    let mut writer = E01Writer::new(&output_path, Default::default()).unwrap();
    for chunk in test_data.chunks(512) {
        let mut sector = vec![0u8; 512];
        sector[..chunk.len()].copy_from_slice(chunk);
        writer.write_sector(&sector).unwrap();
    }
    writer.finalize().unwrap();
    
    // Read from E01
    let vault = E01Vault::open(&temp_dir.path().join("test.E01")).unwrap();
    let mut read_data = Vec::new();
    vault.content().read_to_end(&mut read_data).unwrap();
    
    // Verify data matches
    assert_eq!(read_data.len(), test_data.len());
    assert_eq!(read_data, test_data);
}
```

**Test 2.1.5: Error Handling Tests**
```rust
#[test]
fn test_e01_invalid_sector_size() {
    // Pseudocode:
    // 1. Create writer with bytes_per_sector = 512
    // 2. Try to write sector with size 256
    // 3. Verify error returned
    
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("test");
    
    let config = E01WriterConfig {
        bytes_per_sector: 512,
        ..Default::default()
    };
    
    let mut writer = E01Writer::new(&output_path, config).unwrap();
    
    let wrong_size_data = vec![0u8; 256];
    let result = writer.write_sector(&wrong_size_data);
    
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Sector size mismatch"));
}

#[test]
fn test_e01_file_creation_error() {
    // Pseudocode:
    // 1. Try to create E01 in non-existent directory
    // 2. Verify error returned
    
    let invalid_path = Path::new("/nonexistent/path/test");
    let result = E01Writer::new(invalid_path, Default::default());
    
    assert!(result.is_err());
}
```

**Test 2.1.6: Large File Handling (>2GB)**
```rust
#[test]
#[ignore] // Requires significant disk space
fn test_e01_large_file_handling() {
    // Pseudocode:
    // 1. Create E01 with 3GB of data
    // 2. Verify multiple segments created
    // 3. Verify all segments readable
    // 4. Verify data integrity across segments
    
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("large");
    
    let config = E01WriterConfig {
        max_segment_size: 2_000_000_000, // 2GB
        bytes_per_sector: 512,
        ..Default::default()
    };
    
    let mut writer = E01Writer::new(&output_path, config).unwrap();
    
    // Write 3GB of data
    let sector_data = vec![0x42; 512];
    let sectors_3gb = (3_000_000_000 / 512) as usize;
    
    for i in 0..sectors_3gb {
        if i % 10000 == 0 {
            println!("Writing sector {} of {}", i, sectors_3gb);
        }
        writer.write_sector(&sector_data).unwrap();
    }
    
    writer.finalize().unwrap();
    
    // Verify segments created
    assert!(temp_dir.path().join("large.E01").exists());
    assert!(temp_dir.path().join("large.E02").exists());
    // ... verify all segments ...
}
```

**Target:** +15 tests covering all scenarios above

**2.2: WinPE USB Tests (6 hours)**

**Location:** `crates/totalimage-acquire/src/winpe.rs`

**Detailed Test Implementation:**

**Test 2.2.1: WIM Extraction Validation**
```rust
#[test]
fn test_wim_extraction_validation() {
    // Pseudocode:
    // 1. Create mock WIM file structure
    // 2. Call extract_wim_to_usb()
    // 3. Verify files extracted to USB root
    // 4. Verify file structure matches WinPE layout
    
    let temp_dir = TempDir::new().unwrap();
    let wim_path = temp_dir.path().join("boot.wim");
    let usb_root = temp_dir.path().join("usb");
    
    // Create mock WIM file
    create_mock_wim_file(&wim_path);
    
    // Extract WIM
    extract_wim_to_usb(&wim_path, &usb_root).unwrap();
    
    // Verify WinPE structure
    assert!(usb_root.join("boot").exists());
    assert!(usb_root.join("sources").join("boot.wim").exists());
    assert!(usb_root.join("bootmgr").exists());
    assert!(usb_root.join("boot").join("bcd").exists());
}

fn create_mock_wim_file(path: &Path) {
    // Create minimal valid WIM file for testing
    let mut file = File::create(path).unwrap();
    file.write_all(b"MSWIM\x00\x00\x00").unwrap(); // WIM signature
    // ... write minimal WIM structure ...
}
```

**Test 2.2.2: Boot Configuration Creation**
```rust
#[test]
fn test_boot_config_creation() {
    // Pseudocode:
    // 1. Create USB directory structure
    // 2. Call create_boot_config()
    // 3. Verify BCD store created
    // 4. Verify boot entries configured
    // 5. Verify bootmgr can read BCD
    
    let temp_dir = TempDir::new().unwrap();
    let usb_root = temp_dir.path().join("usb");
    let boot_wim = usb_root.join("sources").join("boot.wim");
    
    // Create directory structure
    std::fs::create_dir_all(&usb_root.join("sources")).unwrap();
    std::fs::File::create(&boot_wim).unwrap();
    
    // Create boot configuration
    create_boot_config(&usb_root, &boot_wim).unwrap();
    
    // Verify BCD store exists
    let bcd_path = usb_root.join("boot").join("bcd");
    assert!(bcd_path.exists());
    
    // Verify BCD is valid (can be parsed)
    let bcd_data = std::fs::read(&bcd_path).unwrap();
    assert!(bcd_data.len() > 0);
    // ... verify BCD structure ...
}
```

**Test 2.2.3: Driver Injection Framework**
```rust
#[test]
fn test_driver_injection() {
    // Pseudocode:
    // 1. Create USB directory structure
    // 2. Create mock driver files
    // 3. Call inject_drivers()
    // 4. Verify drivers copied to driver store
    // 5. Verify driver database updated
    
    let temp_dir = TempDir::new().unwrap();
    let usb_root = temp_dir.path().join("usb");
    let driver_store = usb_root.join("Windows").join("System32").join("DriverStore");
    
    std::fs::create_dir_all(&driver_store).unwrap();
    
    // Create mock drivers
    let drivers = vec![
        PathBuf::from("test_driver1.inf"),
        PathBuf::from("test_driver2.sys"),
    ];
    
    for driver in &drivers {
        std::fs::File::create(driver).unwrap();
    }
    
    // Inject drivers
    inject_drivers(&usb_root, &drivers).unwrap();
    
    // Verify drivers copied
    for driver in &drivers {
        let dest = driver_store.join(driver.file_name().unwrap());
        assert!(dest.exists());
    }
    
    // Verify driver database updated
    let driver_db = usb_root.join("Windows").join("inf").join("setupapi.dev.log");
    // ... verify database contains driver entries ...
}
```

**Test 2.2.4: Error Handling Tests**
```rust
#[test]
fn test_missing_adk_error() {
    // Pseudocode:
    // 1. Call find_winpe_source() on system without ADK
    // 2. Verify appropriate error returned
    
    #[cfg(not(target_os = "windows"))]
    {
        let result = find_winpe_source();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported"));
    }
    
    #[cfg(target_os = "windows")]
    {
        // Mock missing ADK scenario
        // ... test error handling ...
    }
}

#[test]
fn test_invalid_wim_file() {
    // Pseudocode:
    // 1. Create invalid WIM file (wrong signature)
    // 2. Call validate_winpe_source()
    // 3. Verify error returned
    
    let temp_dir = TempDir::new().unwrap();
    let invalid_wim = temp_dir.path().join("invalid.wim");
    
    // Create file with wrong signature
    std::fs::write(&invalid_wim, b"INVALID").unwrap();
    
    let result = validate_winpe_source(&invalid_wim);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid WIM"));
}
```

**Target:** +8 tests covering all WinPE functionality

**2.3: Integration Test Expansion (8 hours)**
- Location: `tests/integration.rs`
- Tests needed:
  - E01 write → read roundtrip
  - Multi-segment E01 handling
  - WinPE USB creation workflow
  - Property test integration
  - **Target:** +14 tests

**2.4: Territory Tests (6 hours)**

**Location:** `crates/totalimage-territories/src/`

**Detailed Test Implementation:**

**Test 2.4.1: ISO Joliet Edge Cases**
```rust
#[test]
fn test_iso_joliet_unicode_filenames() {
    // Pseudocode:
    // 1. Create ISO with Joliet extension
    // 2. Test UTF-16BE filename decoding
    // 3. Test special characters (Japanese, Chinese, etc.)
    // 4. Test version separator handling (/)
    
    // Create test ISO with Joliet
    let iso_data = create_joliet_iso();
    let mut cursor = std::io::Cursor::new(iso_data);
    let iso = IsoTerritory::parse(&mut cursor).unwrap();
    
    // Verify Joliet detected
    assert!(iso.is_joliet);
    assert!(iso.identify().contains("Joliet"));
    
    // Test filename decoding
    let entries = iso.read_directory(&mut cursor, &iso.root_directory).unwrap();
    for entry in entries {
        let name = entry.file_name_with_joliet(true);
        // Verify UTF-16BE decoded correctly
        assert!(!name.is_empty());
    }
}

#[test]
fn test_iso_joliet_version_separator() {
    // Test that '/' version separator is handled correctly
    // Joliet uses '/' instead of ';' for version numbers
    // ... implementation ...
}
```

**Test 2.4.2: exFAT Timestamp Conversion**
```rust
#[test]
fn test_exfat_timestamp_conversion() {
    // Pseudocode:
    // 1. Create exFAT entry with DOS timestamp
    // 2. Convert to chrono::DateTime
    // 3. Verify conversion accuracy
    // 4. Test edge cases (year 1980, year 2107)
    
    use chrono::{DateTime, Utc};
    
    // Test timestamp conversion
    let dos_timestamp = 0x0021_5A7E; // Example DOS timestamp
    let (year, month, day, hour, minute, second) = 
        FileDirectoryEntry::decode_timestamp(dos_timestamp);
    
    let dt = NaiveDate::from_ymd_opt(year as i32, month as u32, day as u32)
        .and_then(|date| NaiveTime::from_hms_opt(hour as u32, minute as u32, second as u32)
            .map(|time| NaiveDateTime::new(date, time)))
        .map(|ndt| DateTime::from_naive_utc_and_offset(ndt, Utc));
    
    assert!(dt.is_some());
    
    // Verify year range (1980-2107)
    let dt = dt.unwrap();
    assert!(dt.year() >= 1980);
    assert!(dt.year() <= 2107);
}
```

**Test 2.4.3: FAT32 Formatting Validation**
```rust
#[test]
fn test_fat32_formatting_validation() {
    // Pseudocode:
    // 1. Format a partition as FAT32
    // 2. Verify boot sector structure
    // 3. Verify FSInfo sector
    // 4. Verify FAT table initialization
    // 5. Verify root directory cluster
    
    let temp_file = tempfile::tempfile().unwrap();
    let mut file = std::fs::File::from(temp_file);
    
    let formatter = Fat32Formatter::new(512, 64, "TEST".to_string());
    formatter.format(&mut file, 0, 400_000_000).unwrap(); // 400MB
    
    // Verify boot sector
    file.seek(SeekFrom::Start(0)).unwrap();
    let mut boot_sector = [0u8; 512];
    file.read_exact(&mut boot_sector).unwrap();
    
    // Verify BPB
    let bpb = BiosParameterBlock::from_bytes(&boot_sector).unwrap();
    assert_eq!(bpb.fat_type, FatType::Fat32);
    assert_eq!(bpb.bytes_per_sector, 512);
    assert_eq!(bpb.sectors_per_cluster, 64);
    
    // Verify FSInfo sector (at sector 1)
    file.seek(SeekFrom::Start(512)).unwrap();
    let mut fsinfo = [0u8; 512];
    file.read_exact(&mut fsinfo).unwrap();
    
    // Verify FSInfo signature
    assert_eq!(&fsinfo[0..4], b"RRaA"); // FSInfo signature
    assert_eq!(&fsinfo[484..488], b"rrAa"); // FSInfo signature 2
    
    // Verify FAT table (starts at reserved sectors)
    let fat_offset = (bpb.reserved_sector_count as u64) * 512;
    file.seek(SeekFrom::Start(fat_offset)).unwrap();
    let mut fat_entry = [0u8; 4];
    file.read_exact(&mut fat_entry).unwrap();
    
    // First FAT entry should be media descriptor
    let media_descriptor = u32::from_le_bytes(fat_entry);
    assert_eq!(media_descriptor & 0xFF, 0xF8); // Media descriptor
}
```

**Target:** +15 tests

**2.5: Zone Tests (4 hours)**

**Location:** `crates/totalimage-zones/src/`

**Detailed Test Implementation:**

**Test 2.5.1: GPT Backup Header Edge Cases**
```rust
#[test]
fn test_gpt_backup_header_edge_cases() {
    // Pseudocode:
    // 1. Create GPT with corrupted backup header
    // 2. Test validation with mismatched CRC32
    // 3. Test validation with mismatched partition count
    // 4. Test validation with mismatched disk GUID
    // 5. Verify warnings are logged
    
    // Create GPT with mismatched backup header
    let mut gpt_data = create_test_gpt();
    
    // Corrupt backup header CRC32
    let backup_header_offset = gpt_data.len() - 512;
    gpt_data[backup_header_offset + 16] = 0xFF; // Corrupt CRC32
    
    let mut cursor = std::io::Cursor::new(gpt_data);
    let config = GptConfig {
        validate_backup_header: true,
    };
    
    // Should succeed but log warning
    let gpt = GptZoneTable::parse_with_config(&mut cursor, 512, config).unwrap();
    
    // Verify backup header exists but is marked as invalid
    assert!(gpt.backup_header().is_some());
    
    // Verify validation detected mismatch (check logs)
    // ... implementation ...
}

#[test]
fn test_gpt_backup_header_missing() {
    // Test GPT with missing backup header
    // ... implementation ...
}

#[test]
fn test_gpt_backup_header_at_disk_end() {
    // Test backup header location calculation
    // ... implementation ...
}
```

**Test 2.5.2: MBR Extended Partition Handling**
```rust
#[test]
fn test_mbr_extended_partition_chain() {
    // Pseudocode:
    // 1. Create MBR with extended partition
    // 2. Create EBR chain with multiple logical partitions
    // 3. Parse MBR and verify all partitions found
    // 4. Test circular reference detection
    // 5. Test maximum chain depth
    
    // Create MBR with extended partition
    let mbr_data = create_mbr_with_extended();
    
    // Add EBR chain
    let ebr_chain = create_ebr_chain(5); // 5 logical partitions
    let full_disk = combine_mbr_and_ebr(mbr_data, ebr_chain);
    
    // Parse MBR
    let mut cursor = std::io::Cursor::new(full_disk);
    let mbr = MbrZoneTable::parse(&mut cursor, 512).unwrap();
    
    // Verify all partitions found (1 primary + 5 logical = 6 total)
    assert_eq!(mbr.zones().len(), 6);
    
    // Verify logical partitions have correct offsets
    for (i, zone) in mbr.zones().iter().enumerate().skip(1) {
        assert!(zone.offset > 0);
        assert!(zone.length > 0);
    }
}

#[test]
fn test_mbr_extended_partition_circular_reference() {
    // Test detection of circular references in EBR chain
    // ... implementation ...
}
```

**Target:** +8 tests

**2.6: Web API Tests (4 hours)**

**Location:** `crates/totalimage-web/src/`

**Detailed Test Implementation:**

**Test 2.6.1: exFAT Extraction Endpoint**
```rust
#[tokio::test]
async fn test_web_api_exfat_extraction() {
    // Pseudocode:
    // 1. Create test server
    // 2. Upload exFAT image
    // 3. Call extraction endpoint
    // 4. Verify file extracted correctly
    // 5. Verify response format
    
    let app = create_test_app().await;
    
    // Upload exFAT image
    let image_path = "tests/fixtures/test_exfat.img";
    let upload_response = upload_image(&app, image_path).await;
    assert!(upload_response.status().is_success());
    
    // Extract file from exFAT
    let extract_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/vaults/extract")
                .header("content-type", "application/json")
                .body(Body::from(json!({
                    "image_path": image_path,
                    "zone_index": 0,
                    "file_path": "test.txt"
                })))
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(extract_response.status(), 200);
    
    let body = extract_response.into_body();
    let bytes = hyper::body::to_bytes(body).await.unwrap();
    let file_data: Vec<u8> = serde_json::from_slice(&bytes).unwrap();
    
    assert!(!file_data.is_empty());
}
```

**Test 2.6.2: ISO Extraction Endpoint**
```rust
#[tokio::test]
async fn test_web_api_iso_extraction() {
    // Similar to exFAT but with ISO image
    // Test Joliet filename handling in API
    // ... implementation ...
}
```

**Test 2.6.3: Error Handling Improvements**
```rust
#[tokio::test]
async fn test_web_api_error_handling() {
    // Test various error scenarios:
    // - Invalid image path
    // - Invalid zone index
    // - File not found
    // - Permission errors
    // - Corrupted image
    
    let app = create_test_app().await;
    
    // Test invalid zone index
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/vaults/files?image=test.vhd&zone=999")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), 400);
    
    // Test file not found
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/vaults/extract")
                .body(Body::from(json!({
                    "image_path": "test.vhd",
                    "zone_index": 0,
                    "file_path": "nonexistent.txt"
                })))
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), 404);
}
```

**Target:** +4 tests

**2.7: CLI Tests (4 hours)**

**Location:** `crates/totalimage-cli/src/`

**Detailed Test Implementation:**

**Test 2.7.1: Hash Command Edge Cases**
```rust
#[test]
fn test_hash_command_edge_cases() {
    // Pseudocode:
    // 1. Test hash with empty file
    // 2. Test hash with very large file
    // 3. Test hash with all algorithms
    // 4. Test hash output formats (hex, base64)
    // 5. Test hash with missing file
    
    use std::process::Command;
    
    // Test empty file
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    let output = Command::new("totalimage-cli")
        .args(&["hash", temp_file.path().to_str().unwrap()])
        .output()
        .unwrap();
    
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Hash:"));
    
    // Test all algorithms
    for algo in ["md5", "sha1", "sha256"] {
        let output = Command::new("totalimage-cli")
            .args(&["hash", "--algorithm", algo, temp_file.path().to_str().unwrap()])
            .output()
            .unwrap();
        assert!(output.status.success());
    }
    
    // Test missing file
    let output = Command::new("totalimage-cli")
        .args(&["hash", "/nonexistent/file"])
        .output()
        .unwrap();
    assert!(!output.status.success());
}
```

**Test 2.7.2: WinPE USB Command Validation**
```rust
#[test]
fn test_winpe_usb_command_validation() {
    // Test argument validation
    // Test error messages
    // Test help text
    // ... implementation ...
}
```

**Test 2.7.3: Error Message Verification**
```rust
#[test]
fn test_cli_error_messages() {
    // Verify error messages are user-friendly
    // Test various error scenarios
    // ... implementation ...
}
```

**Target:** +6 tests

**2.8: Core & Pipeline Tests (2 hours)**
- Location: `crates/totalimage-core/`, `crates/totalimage-pipeline/`
- Tests needed:
  - Edge case validation
  - Error propagation tests
  - **Target:** +8 tests

**Total Effort:** 42 hours  
**Target:** +92 tests

---

## Part 3: Integration Test Completion (6 → 20+ tests)

### Current Status
- ✅ Framework created
- ✅ 6 basic tests passing
- ❌ Missing: 14+ comprehensive tests

### Remaining Integration Tests

**3.1: E01 Write/Read Roundtrip (2 hours)**

**Detailed Implementation:**
```rust
// tests/integration.rs

#[test]
fn test_e01_write_read_roundtrip() {
    // Pseudocode:
    // 1. Create test data (1MB of random data)
    // 2. Write to E01 using E01Writer
    // 3. Read from E01 using E01Vault
    // 4. Verify data matches byte-for-byte
    // 5. Verify compression worked (file size < original)
    // 6. Verify MD5 hash matches
    
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("test");
    
    // Step 1: Generate test data
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let test_data: Vec<u8> = (0..1_000_000) // 1MB
        .map(|_| rng.gen())
        .collect();
    
    // Step 2: Write to E01
    let config = E01WriterConfig {
        bytes_per_sector: 512,
        sectors_per_chunk: 64,
        compression: 1, // deflate
        ..Default::default()
    };
    
    let mut writer = E01Writer::new(&output_path, config).unwrap();
    
    for chunk in test_data.chunks(512) {
        let mut sector = vec![0u8; 512];
        let copy_len = chunk.len().min(512);
        sector[..copy_len].copy_from_slice(&chunk[..copy_len]);
        writer.write_sector(&sector).unwrap();
    }
    
    writer.finalize().unwrap();
    
    // Step 3: Read from E01
    let e01_path = temp_dir.path().join("test.E01");
    let vault = E01Vault::open(&e01_path).unwrap();
    
    // Step 4: Verify data integrity
    let mut read_data = Vec::new();
    vault.content().read_to_end(&mut read_data).unwrap();
    
    assert_eq!(read_data.len(), test_data.len());
    assert_eq!(read_data, test_data);
    
    // Step 5: Verify compression
    let e01_size = std::fs::metadata(&e01_path).unwrap().len();
    assert!(e01_size < test_data.len() as u64, 
        "E01 should be smaller than original due to compression");
    
    // Step 6: Verify MD5 hash
    use md5::{Digest, Md5};
    let mut hasher = Md5::new();
    hasher.update(&test_data);
    let expected_hash = hasher.finalize();
    
    let stored_hash = vault.hash().unwrap();
    assert_eq!(stored_hash.md5_hash, expected_hash.into());
}

#[test]
fn test_e01_multi_segment_roundtrip() {
    // Test multi-segment E01 files
    // ... similar structure but with >2GB data ...
}
```

**3.2: WinPE USB Creation Workflow (2 hours)**

**Detailed Implementation:**
```rust
// tests/integration.rs

#[test]
#[ignore] // Requires Windows ADK and physical USB
fn test_winpe_usb_creation_workflow() {
    // Pseudocode:
    // 1. Detect USB drive
    // 2. Create partition table (MBR)
    // 3. Format FAT32
    // 4. Extract WIM to USB
    // 5. Create boot configuration
    // 6. Inject drivers (optional)
    // 7. Verify USB is bootable
    
    let temp_dir = TempDir::new().unwrap();
    let usb_device = temp_dir.path().join("usb_device.img"); // Mock USB
    
    // Step 1: Detect USB (mock for test)
    let usb_drives = detect_usb_drives().unwrap();
    // In real test, would use actual USB device
    
    // Step 2: Create partition table
    let partition_builder = PartitionTableBuilder::new(PartitionTableType::Mbr, 512);
    let mbr_data = partition_builder.create_mbr(4_000_000_000).unwrap(); // 4GB
    
    // Write MBR to device
    let mut device = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(&usb_device)
        .unwrap();
    device.write_all(&mbr_data).unwrap();
    
    // Step 3: Format FAT32
    let formatter = Fat32Formatter::new(512, 64, "WINPE".to_string());
    formatter.format(&mut device, 512, 4_000_000_000).unwrap();
    
    // Step 4: Extract WIM
    let boot_wim = find_winpe_source().unwrap();
    extract_wim_to_usb(&boot_wim.boot_wim_path, &temp_dir.path().join("usb_mount")).unwrap();
    
    // Step 5: Create boot configuration
    create_boot_config(
        &temp_dir.path().join("usb_mount"),
        &boot_wim.boot_wim_path
    ).unwrap();
    
    // Step 6: Verify structure
    let usb_root = temp_dir.path().join("usb_mount");
    assert!(usb_root.join("boot").join("bcd").exists());
    assert!(usb_root.join("sources").join("boot.wim").exists());
    assert!(usb_root.join("bootmgr").exists());
}
```

**3.3: Property Test Integration (2 hours)**
- Test: Property tests in integration context
- Verify: End-to-end property validation

**3.4: Multi-Format Pipeline Tests (4 hours)**

**Detailed Implementation:**

**Test 3.4.1: VHD → GPT → FAT32 → Extract**
```rust
#[test]
fn test_vhd_gpt_fat32_extract_pipeline() {
    // Pseudocode:
    // 1. Open VHD vault
    // 2. Parse GPT partition table
    // 3. Open FAT32 territory on partition
    // 4. List files
    // 5. Extract a file
    // 6. Verify extracted file matches
    
    let vhd_path = Path::new("tests/fixtures/test.vhd");
    let vault = VhdVault::open(vhd_path, Default::default()).unwrap();
    
    // Parse GPT
    let mut cursor = std::io::Cursor::new(vault.content());
    let gpt = GptZoneTable::parse(&mut cursor, 512).unwrap();
    
    // Get first partition
    let partition = gpt.zones().first().unwrap();
    
    // Open FAT32 on partition
    let partition_data = read_partition_data(&vault, partition).unwrap();
    let mut partition_cursor = std::io::Cursor::new(partition_data);
    let fat = FatTerritory::parse(&mut partition_cursor).unwrap();
    
    // List files
    let files = fat.list_directory("/").unwrap();
    assert!(!files.is_empty());
    
    // Extract first file
    let file_to_extract = &files[0];
    let extracted_data = fat.read_file(file_to_extract.name()).unwrap();
    
    // Verify extraction
    assert_eq!(extracted_data.len(), file_to_extract.size() as usize);
}
```

**Test 3.4.2: E01 → MBR → NTFS → Extract**
```rust
#[test]
fn test_e01_mbr_ntfs_extract_pipeline() {
    // Similar structure but with:
    // - E01Vault instead of VhdVault
    // - MbrZoneTable instead of GptZoneTable
    // - NtfsTerritory instead of FatTerritory
    // ... implementation ...
}
```

**Test 3.4.3: AFF4 → GPT → exFAT → Extract**
```rust
#[test]
fn test_aff4_gpt_exfat_extract_pipeline() {
    // Similar structure but with:
    // - Aff4Vault instead of VhdVault
    // - ExfatTerritory instead of FatTerritory
    // ... implementation ...
}
```

**Test 3.4.4: Raw → Direct → ISO → Extract**
```rust
#[test]
fn test_raw_direct_iso_extract_pipeline() {
    // Pseudocode:
    // 1. Open raw image (no partition table)
    // 2. Parse ISO directly
    // 3. Extract file from ISO
    // 4. Verify extraction
    
    let raw_path = Path::new("tests/fixtures/test.iso");
    let vault = RawVault::open(raw_path).unwrap();
    
    // Parse ISO directly (no partition table)
    let mut cursor = std::io::Cursor::new(vault.content());
    let iso = IsoTerritory::parse(&mut cursor).unwrap();
    
    // Extract file
    let file_data = iso.read_file("README.TXT").unwrap();
    
    // Verify
    assert!(!file_data.is_empty());
}
```

**3.5: Error Recovery Tests (2 hours)**
- Test: Corrupted image handling
- Test: Missing file handling
- Test: Permission error handling

**3.6: Performance Integration Tests (2 hours)**
- Test: Large file handling (>10GB)
- Test: Concurrent request handling
- Test: Memory usage validation

**Total Effort:** 14 hours  
**Target:** +14 tests

---

## Part 4: WinPE USB Completion (60% → 100%)

### Current Status
- ✅ **10.1.1: USB Drive Detection** - Complete (Linux, stubs for Windows/macOS)
- ✅ **10.1.2: Partition Table Creation** - Complete (MBR, GPT placeholder)
- ✅ **10.1.3: FAT32 Formatting** - Complete
- ✅ **10.2.1: WinPE Source Detection** - Complete
- ✅ **10.3: CLI Integration** - Complete
- ❌ **10.2.2: WIM Extraction** - Placeholder
- ❌ **10.2.3: Boot Configuration** - Placeholder
- ❌ **10.2.4: Driver Injection** - Placeholder

### Remaining Tasks

**4.1: WIM Extraction Implementation (8 hours)**

**Location:** `crates/totalimage-acquire/src/winpe.rs`

**Implementation Steps:**

1. **Research WIM format** (2 hours)
   - WIM file structure
   - Image extraction methods
   - Windows API vs. library approach

2. **Implement WIM extraction** (4 hours)
   
   **Detailed Implementation Pseudocode:**
   ```rust
   // crates/totalimage-acquire/src/winpe.rs
   
   pub fn extract_wim_to_usb(wim_path: &Path, usb_root: &Path) -> Result<()> {
       // Step 1: Validate WIM file exists and is valid
       validate_winpe_source(wim_path)?;
       
       // Step 2: Create USB directory structure
       let sources_dir = usb_root.join("sources");
       std::fs::create_dir_all(&sources_dir)
           .map_err(|e| AcquireError::WriteError(format!("Failed to create sources dir: {}", e)))?;
       
       // Step 3: Determine extraction method based on platform
       #[cfg(target_os = "windows")]
       {
           // Use DISM.exe on Windows
           extract_wim_with_dism(wim_path, &sources_dir)?;
       }
       
       #[cfg(not(target_os = "windows"))]
       {
           // Use wimlib on cross-platform (if available)
           // Fallback: Return error with instructions
           return Err(AcquireError::UnsupportedPlatform(
               "WIM extraction on non-Windows requires wimlib. \
                Install wimlib-tools and ensure 'wimextract' is in PATH.".to_string()
           ));
       }
       
       // Step 4: Verify extraction success
       let extracted_wim = sources_dir.join("boot.wim");
       if !extracted_wim.exists() {
           return Err(AcquireError::Internal(
               "WIM extraction failed: boot.wim not found after extraction".to_string()
           ));
       }
       
       // Step 5: Verify file size matches (at least partially)
       let original_size = std::fs::metadata(wim_path)?.len();
       let extracted_size = std::fs::metadata(&extracted_wim)?.len();
       
       if extracted_size == 0 {
           return Err(AcquireError::Internal(
               "WIM extraction failed: extracted file is empty".to_string()
           ));
       }
       
       Ok(())
   }
   
   #[cfg(target_os = "windows")]
   fn extract_wim_with_dism(wim_path: &Path, dest_dir: &Path) -> Result<()> {
       // Step 1: Build DISM command
       // DISM.exe /Apply-Image /ImageFile:boot.wim /Index:1 /ApplyDir:C:\USB
       let dism_path = find_dism_executable()?;
       
       let mut cmd = std::process::Command::new(&dism_path);
       cmd.arg("/Apply-Image")
          .arg("/ImageFile")
          .arg(wim_path)
          .arg("/Index")
          .arg("1") // First image in WIM
          .arg("/ApplyDir")
          .arg(dest_dir.parent().unwrap())
          .arg("/CheckIntegrity")
          .arg("/Verify");
       
       // Step 2: Execute DISM
       let output = cmd.output()
           .map_err(|e| AcquireError::Internal(format!("Failed to execute DISM: {}", e)))?;
       
       // Step 3: Check exit status
       if !output.status.success() {
           let stderr = String::from_utf8_lossy(&output.stderr);
           return Err(AcquireError::Internal(format!(
               "DISM extraction failed: {}", stderr
           )));
       }
       
       Ok(())
   }
   
   #[cfg(target_os = "windows")]
   fn find_dism_executable() -> Result<PathBuf> {
       // Check common DISM locations
       let paths = vec![
           PathBuf::from(r"C:\Windows\System32\Dism.exe"),
           PathBuf::from(r"C:\Windows\SysWOW64\Dism.exe"),
       ];
       
       for path in paths {
           if path.exists() {
               return Ok(path);
           }
       }
       
       Err(AcquireError::SourceNotFound(
           "DISM.exe not found. Windows ADK may not be installed.".to_string()
       ))
   }
   ```

3. **Add error handling** (1 hour)
   - Missing WIM file
   - Extraction failures
   - Permission errors

4. **Add tests** (1 hour)
   - Mock WIM extraction
   - Verify file structure
   - Error case testing

**Acceptance Criteria:**
- [ ] WIM files extracted to USB
- [ ] File structure correct
- [ ] Error handling comprehensive
- [ ] Tests added

**4.2: Boot Configuration Creation (4 hours)**

**Location:** `crates/totalimage-acquire/src/winpe.rs`

**Implementation Steps:**

1. **Research BCD format** (1 hour)
   - Boot Configuration Data structure
   - BCDEdit commands
   - Alternative: Direct BCD file creation

2. **Implement BCD creation** (2 hours)
   
   **Detailed Implementation Pseudocode:**
   ```rust
   // crates/totalimage-acquire/src/winpe.rs
   
   pub fn create_boot_config(usb_root: &Path, boot_wim_path: &Path) -> Result<()> {
       // Step 1: Create boot directory structure
       let boot_dir = usb_root.join("boot");
       std::fs::create_dir_all(&boot_dir)
           .map_err(|e| AcquireError::WriteError(format!("Failed to create boot dir: {}", e)))?;
       
       // Step 2: Determine BCD creation method
       #[cfg(target_os = "windows")]
       {
           // Use bcdedit.exe on Windows
           create_bcd_with_bcdedit(usb_root, boot_wim_path)?;
       }
       
       #[cfg(not(target_os = "windows"))]
       {
           // Create minimal BCD structure manually
           create_bcd_manually(&boot_dir, boot_wim_path)?;
       }
       
       // Step 3: Verify BCD store created
       let bcd_path = boot_dir.join("bcd");
       if !bcd_path.exists() {
           return Err(AcquireError::Internal(
               "BCD store creation failed: bcd file not found".to_string()
           ));
       }
       
       Ok(())
   }
   
   #[cfg(target_os = "windows")]
   fn create_bcd_with_bcdedit(usb_root: &Path, boot_wim_path: &Path) -> Result<()> {
       // Step 1: Find bcdedit.exe
       let bcdedit = PathBuf::from(r"C:\Windows\System32\bcdedit.exe");
       if !bcdedit.exists() {
           return Err(AcquireError::SourceNotFound(
               "bcdedit.exe not found".to_string()
           ));
       }
       
       // Step 2: Create BCD store
       let bcd_path = usb_root.join("boot").join("bcd");
       let mut cmd = std::process::Command::new(&bcdedit);
       cmd.arg("/createstore")
          .arg(&bcd_path);
       
       let output = cmd.output()
           .map_err(|e| AcquireError::Internal(format!("Failed to create BCD store: {}", e)))?;
       
       if !output.status.success() {
           return Err(AcquireError::Internal(
               "Failed to create BCD store".to_string()
           ));
       }
       
       // Step 3: Create boot entry
       let mut cmd = std::process::Command::new(&bcdedit);
       cmd.arg("/store")
          .arg(&bcd_path)
          .arg("/create")
          .arg("/d")
          .arg("Windows PE")
          .arg("/application")
          .arg("osloader");
       
       let output = cmd.output()?;
       let entry_id = extract_entry_id_from_output(&output.stdout)?;
       
       // Step 4: Configure boot entry
       configure_boot_entry(&bcd_path, &entry_id, boot_wim_path)?;
       
       Ok(())
   }
   
   fn configure_boot_entry(bcd_path: &Path, entry_id: &str, boot_wim_path: &Path) -> Result<()> {
       // Set device and osdevice
       set_bcd_value(bcd_path, entry_id, "device", "ramdisk=[boot]\\sources\\boot.wim")?;
       set_bcd_value(bcd_path, entry_id, "osdevice", "ramdisk=[boot]\\sources\\boot.wim")?;
       
       // Set path
       set_bcd_value(bcd_path, entry_id, "path", "\\Windows\\System32\\boot\\winload.exe")?;
       
       // Set systemroot
       set_bcd_value(bcd_path, entry_id, "systemroot", "\\Windows")?;
       
       // Set winpe flag
       set_bcd_value(bcd_path, entry_id, "winpe", "Yes")?;
       
       Ok(())
   }
   
   fn set_bcd_value(bcd_path: &Path, entry_id: &str, key: &str, value: &str) -> Result<()> {
       let mut cmd = std::process::Command::new("bcdedit.exe");
       cmd.arg("/store")
          .arg(bcd_path)
          .arg("/set")
          .arg(entry_id)
          .arg(key)
          .arg(value);
       
       let output = cmd.output()?;
       if !output.status.success() {
           return Err(AcquireError::Internal(format!(
               "Failed to set BCD value {}={}", key, value
           )));
       }
       
       Ok(())
   }
   
   #[cfg(not(target_os = "windows"))]
   fn create_bcd_manually(boot_dir: &Path, boot_wim_path: &Path) -> Result<()> {
       // Create minimal BCD structure for non-Windows platforms
       // This is a simplified version - full BCD format is complex
       
       // BCD is a registry hive format, requires:
       // 1. Registry hive header
       // 2. Boot entries as registry keys
       // 3. Binary data structures
       
       // For now, create a placeholder that indicates manual configuration needed
       let bcd_path = boot_dir.join("bcd");
       std::fs::write(&bcd_path, b"BCD_PLACEHOLDER")
           .map_err(|e| AcquireError::WriteError(format!("Failed to write BCD: {}", e)))?;
       
       // Log warning that manual configuration may be needed
       tracing::warn!(
           "BCD created on non-Windows platform. \
            Manual configuration with bcdedit may be required for bootable USB."
       );
       
       Ok(())
   }
   ```

3. **Add tests** (1 hour)
   - BCD file validation
   - Boot entry verification

**Acceptance Criteria:**
- [ ] BCD store created
- [ ] Boot entries configured
- [ ] USB bootable
- [ ] Tests added

**4.3: Driver Injection Framework (4 hours)**

**Location:** `crates/totalimage-acquire/src/winpe.rs`

**Implementation Steps:**

1. **Design driver injection API** (1 hour)
   
   **Detailed API Design:**
   ```rust
   // crates/totalimage-acquire/src/winpe.rs
   
   /// Driver injection result
   #[derive(Debug)]
   pub struct DriverInjectionResult {
       /// Number of drivers successfully injected
       pub injected_count: usize,
       /// Drivers that failed to inject
       pub failed_drivers: Vec<(PathBuf, String)>,
   }
   
   /// Inject drivers into WinPE image
   ///
   /// # Arguments
   ///
   /// * `usb_root` - Root directory of USB drive
   /// * `drivers` - List of driver file paths (.inf, .sys, .cat files)
   ///
   /// # Returns
   ///
   /// Result with injection statistics
   pub fn inject_drivers(usb_root: &Path, drivers: &[PathBuf]) -> Result<DriverInjectionResult> {
       // Step 1: Validate USB root exists
       if !usb_root.exists() {
           return Err(AcquireError::SourceNotFound(
               format!("USB root does not exist: {}", usb_root.display())
           ));
       }
       
       // Step 2: Create driver store directory
       let driver_store = usb_root
           .join("Windows")
           .join("System32")
           .join("DriverStore")
           .join("FileRepository");
       
       std::fs::create_dir_all(&driver_store)
           .map_err(|e| AcquireError::WriteError(format!("Failed to create driver store: {}", e)))?;
       
       // Step 3: Process each driver
       let mut injected_count = 0;
       let mut failed_drivers = Vec::new();
       
       for driver_path in drivers {
           match inject_single_driver(&driver_store, driver_path) {
               Ok(_) => injected_count += 1,
               Err(e) => {
                   failed_drivers.push((driver_path.clone(), e.to_string()));
               }
           }
       }
       
       // Step 4: Update driver database
       if injected_count > 0 {
           update_driver_database(usb_root, &driver_store)?;
       }
       
       Ok(DriverInjectionResult {
           injected_count,
           failed_drivers,
       })
   }
   
   fn inject_single_driver(driver_store: &Path, driver_path: &Path) -> Result<()> {
       // Step 1: Validate driver file exists
       if !driver_path.exists() {
           return Err(AcquireError::SourceNotFound(format!(
               "Driver file not found: {}",
               driver_path.display()
           )));
       }
       
       // Step 2: Determine driver type and destination
       let file_name = driver_path.file_name()
           .ok_or_else(|| AcquireError::InvalidInput("Invalid driver path".to_string()))?;
       
       let extension = driver_path.extension()
           .and_then(|s| s.to_str())
           .unwrap_or("");
       
       // Step 3: Create driver-specific directory
       let driver_name = file_name.to_string_lossy();
       let driver_dir = driver_store.join(format!("{}.inf_amd64_*", 
           driver_name.trim_end_matches(extension).trim_end_matches(".")));
       
       // Create directory with unique identifier
       let driver_dir = create_unique_driver_dir(&driver_store, &driver_name)?;
       
       // Step 4: Copy driver files
       std::fs::copy(driver_path, driver_dir.join(file_name))
           .map_err(|e| AcquireError::WriteError(format!("Failed to copy driver: {}", e)))?;
       
       // Step 5: Handle driver dependencies (.inf files may reference other files)
       if extension == "inf" {
           copy_driver_dependencies(driver_path, &driver_dir)?;
       }
       
       Ok(())
   }
   
   fn copy_driver_dependencies(inf_path: &Path, dest_dir: &Path) -> Result<()> {
       // Parse .inf file to find referenced files
       let inf_content = std::fs::read_to_string(inf_path)?;
       
       // Extract referenced files from [SourceDisksFiles] section
       // Pattern: filename = diskid[,subdir][,size]
       let source_dir = inf_path.parent().unwrap();
       
       for line in inf_content.lines() {
           if line.trim().starts_with('[') && line.contains("SourceDisksFiles") {
               // Parse section and copy referenced files
               // ... implementation ...
           }
       }
       
       Ok(())
   }
   
   fn update_driver_database(usb_root: &Path, driver_store: &Path) -> Result<()> {
       // Update Windows driver database
       // Location: Windows\inf\setupapi.dev.log
       
       let inf_dir = usb_root.join("Windows").join("inf");
       std::fs::create_dir_all(&inf_dir)?;
       
       // Create or append to setupapi.dev.log
       let log_path = inf_dir.join("setupapi.dev.log");
       
       // Format: [Version]
       // Signature="$Windows NT$"
       // [DriverInstallLog]
       // ... driver entries ...
       
       let mut log_content = String::new();
       if log_path.exists() {
           log_content = std::fs::read_to_string(&log_path)?;
       } else {
           log_content.push_str("[Version]\n");
           log_content.push_str("Signature=\"$Windows NT$\"\n");
       }
       
       // Append driver entries
       // ... implementation ...
       
       std::fs::write(&log_path, log_content)?;
       
       Ok(())
   }
   ```

2. **Implement driver injection** (2 hours)
   - Copy drivers to appropriate location
   - Update WinPE driver database
   - Handle driver dependencies

3. **Add tests** (1 hour)
   - Driver injection verification
   - Error handling

**Acceptance Criteria:**
- [ ] Drivers injected successfully
   - Driver database updated
   - Tests added

**Total Effort:** 16 hours  
**Target:** 100% WinPE USB completion

---

## Part 5: P3 Feature Completion (0% → 100%)

### Current Status
- ❌ All P3 features not started
- **Priority:** Low (optional, but needed for 100%)

### P3 Features

**5.1: CLI Tree Command (4 hours)**

**Location:** `crates/totalimage-cli/src/main.rs`

**Detailed Implementation Pseudocode:**

**Step 1: Add tree command to CLI (1 hour)**
```rust
// crates/totalimage-cli/src/main.rs

match command.as_str() {
    // ... existing commands ...
    "tree" => {
        if let Err(e) = cmd_tree(&args[1..]) {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
    // ...
}

fn cmd_tree(args: &[String]) -> Result<()> {
    // Step 1: Parse arguments
    // tree <image> [--zone N] [--path PATH] [--depth DEPTH]
    
    let mut image_path = None;
    let mut zone_index = None;
    let mut path = "/".to_string();
    let mut max_depth = usize::MAX;
    
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--zone" => {
                zone_index = Some(args[i + 1].parse()?);
                i += 2;
            }
            "--path" => {
                path = args[i + 1].clone();
                i += 2;
            }
            "--depth" => {
                max_depth = args[i + 1].parse()?;
                i += 2;
            }
            _ => {
                if image_path.is_none() {
                    image_path = Some(args[i].clone());
                }
                i += 1;
            }
        }
    }
    
    let image_path = image_path.ok_or_else(|| {
        totalimage_core::Error::InvalidOperation("Image path required".to_string())
    })?;
    
    // Step 2: Open vault
    let vault = open_vault(&Path::new(&image_path), Default::default())?;
    
    // Step 3: Get territory
    let territory = get_territory_for_tree(&vault, zone_index)?;
    
    // Step 4: Print tree
    print_directory_tree(&territory, &path, max_depth, 0, false, "")?;
    
    Ok(())
}

fn print_directory_tree(
    territory: &dyn Territory,
    path: &str,
    max_depth: usize,
    current_depth: usize,
    is_last: bool,
    prefix: &str,
) -> Result<()> {
    // Step 1: Check depth limit
    if current_depth >= max_depth {
        return Ok(());
    }
    
    // Step 2: List directory
    let entries = territory.list_directory(path)?;
    
    // Step 3: Sort entries (directories first, then files)
    let mut dirs = Vec::new();
    let mut files = Vec::new();
    
    for entry in entries {
        if entry.is_directory {
            dirs.push(entry);
        } else {
            files.push(entry);
        }
    }
    
    dirs.sort_by(|a, b| a.name.cmp(&b.name));
    files.sort_by(|a, b| a.name.cmp(&b.name));
    
    // Step 4: Print entries
    let all_entries: Vec<_> = dirs.into_iter().chain(files.into_iter()).collect();
    
    for (i, entry) in all_entries.iter().enumerate() {
        let is_last_entry = i == all_entries.len() - 1;
        
        // Print tree characters
        let connector = if is_last_entry {
            "└── "
        } else {
            "├── "
        };
        
        println!("{}{}{}", prefix, connector, entry.name);
        
        // Recursively print subdirectories
        if entry.is_directory && current_depth + 1 < max_depth {
            let new_prefix = if is_last_entry {
                format!("{}    ", prefix)
            } else {
                format!("{}│   ", prefix)
            };
            
            let new_path = if path == "/" {
                format!("/{}", entry.name)
            } else {
                format!("{}/{}", path, entry.name)
            };
            
            print_directory_tree(
                territory,
                &new_path,
                max_depth,
                current_depth + 1,
                is_last_entry,
                &new_prefix,
            )?;
        }
    }
    
    Ok(())
}

fn get_territory_for_tree(
    vault: &dyn Vault,
    zone_index: Option<usize>,
) -> Result<Box<dyn Territory>> {
    // Step 1: If zone specified, use that partition
    if let Some(zone_idx) = zone_index {
        let zone_table = detect_zone_table(vault)?;
        let zone = zone_table.zones().get(zone_idx)
            .ok_or_else(|| totalimage_core::Error::InvalidOperation(
                format!("Zone {} not found", zone_idx)
            ))?;
        
        let zone_data = read_zone_data(vault, zone)?;
        return parse_territory(&zone_data);
    }
    
    // Step 2: Try to detect partition table
    let zone_table = detect_zone_table(vault)?;
    
    if !zone_table.zones().is_empty() {
        // Use first partition
        let zone = &zone_table.zones()[0];
        let zone_data = read_zone_data(vault, zone)?;
        return parse_territory(&zone_data);
    }
    
    // Step 3: Try direct territory (no partition table)
    let vault_data = read_vault_data(vault)?;
    parse_territory(&vault_data)
}

fn parse_territory(data: &[u8]) -> Result<Box<dyn Territory>> {
    let mut cursor = std::io::Cursor::new(data);
    
    // Try each filesystem type
    if let Ok(fat) = FatTerritory::parse(&mut cursor) {
        return Ok(Box::new(fat));
    }
    
    cursor.seek(SeekFrom::Start(0))?;
    if let Ok(ntfs) = NtfsTerritory::parse(&mut cursor) {
        return Ok(Box::new(ntfs));
    }
    
    cursor.seek(SeekFrom::Start(0))?;
    if let Ok(exfat) = ExfatTerritory::parse(&mut cursor) {
        return Ok(Box::new(exfat));
    }
    
    cursor.seek(SeekFrom::Start(0))?;
    if let Ok(iso) = IsoTerritory::parse(&mut cursor) {
        return Ok(Box::new(iso));
    }
    
    Err(totalimage_core::Error::InvalidOperation(
        "No supported filesystem found".to_string()
    ))
}
```

**Step 2: Add tests (1 hour)**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_tree_command_parsing() {
        // Test argument parsing
        let args = vec!["tree".to_string(), "image.vhd".to_string()];
        // ... verify parsing ...
    }
    
    #[test]
    fn test_tree_printing() {
        // Test tree character generation
        // ... verify output format ...
    }
}
```

**Step 3: Update help text (30 min)**
```rust
fn print_usage() {
    println!("Usage: totalimage-cli <command> [options]");
    println!();
    println!("Commands:");
    // ... existing commands ...
    println!("  tree <image> [--zone N] [--path PATH] [--depth DEPTH]");
    println!("      Print directory tree structure");
    println!("      --zone N     : Partition number (0-based)");
    println!("      --path PATH  : Starting path (default: /)");
    println!("      --depth N    : Maximum depth (default: unlimited)");
}
```

**5.2: AFF4 Write Support (20+ hours)**
- Location: `crates/totalimage-acquire/src/aff4_writer.rs` (new)
- Complex: Requires AFF4 format research
- Compression: Snappy, LZ4, Deflate
- Multi-bevy support
- Turtle RDF metadata

**5.3: ext2/3/4 Filesystem Support (40 hours)**
- Location: `crates/totalimage-territories/src/ext/` (new)
- Significant feature
- Consider as separate phase

**5.4: BitLocker Decryption (Unknown)**
- Research required
- Encryption library integration
- Complex implementation

**5.5: Legacy Emulator Formats (Unknown)**
- NHD (Neko98)
- IMZ (Compressed)
- Anex86
- PCjs

**5.6: Video Tutorials (16 hours)**
- Quick start tutorial
- CLI usage tutorial
- Web UI tutorial
- Advanced features tutorial

**Total Effort:** 80+ hours (plus unknown)

**Recommendation:** Prioritize based on user demand. For 100% completion, focus on:
1. CLI Tree Command (4h) - Easy win
2. Video Tutorials (16h) - User education
3. Defer complex items (AFF4 write, ext2/3/4, BitLocker) to future releases

---

## Part 6: Documentation Updates

### Required Updates

**6.1: Update Gap Analysis Report**

**Detailed Implementation Steps:**

**Step 1: Update P2 Status Section**
```markdown
# steering/COMPLETE-GAP-ANALYSIS-REPORT.md

### P2 (Medium) - 🔄 83% Complete (5/6)
- ✅ GPT backup header validation (Iteration 11.1) - Complete
- ✅ ISO Joliet extension (Iteration 11.2) - Complete  
- ✅ E01 write support (Iteration 11.3) - Complete
- ✅ CLI hash command (Iteration 11.4) - Complete
- ✅ exFAT/ISO extraction (Iteration 11.5) - Complete
- ⏳ Property-based testing (Iteration 11.6) - In Progress

**Total P2 Effort:** ~44 hours (36/44 complete, 8 hours remaining)
```

**Step 2: Update Test Counts**
```markdown
### Test Coverage Goals

| Objective | Documented In | Target | Actual | Status |
|-----------|---------------|--------|--------|--------|
| Unit Tests | ITERATION-PLAN-TO-100.md | 414+ | 322 | ⚠️ In Progress (+92 needed) |
| Integration Tests | ITERATION-PLAN-TO-100.md | 20+ | 6 | ⚠️ In Progress (+14 needed) |
| Property Tests | GAP-ANALYSIS.md | 10+ | 0 | ❌ Not Started (+10 needed) |
| Fuzzing | ITERATION-PLAN-TO-100.md | 5 targets | 5 targets | ✅ Complete |
```

**Step 3: Update WinPE USB Status**
```markdown
### Gap: WinPE Bootable USB Creation
- **Priority:** P1 (High)
- **Effort:** 32 hours estimated
- **Status:** 🔄 60% Complete (19.2/32 hours)
- **Completed:**
  - ✅ USB Drive Detection (Linux, Windows/macOS stubs)
  - ✅ Partition Table Creation (MBR complete)
  - ✅ FAT32 Formatting
  - ✅ WinPE Source Detection
  - ✅ CLI Integration
- **Remaining:**
  - ⏳ WIM Extraction (8 hours)
  - ⏳ Boot Configuration (4 hours)
  - ⏳ Driver Injection (4 hours)
```

**6.2: Update Implementation Dashboard**
- Correct P2 status (currently shows 0%, should be 83%)
- Update test counts
- Update WinPE USB progress
- Add property test status

**6.3: Update README**
- Add E01 write support
- Add property-based testing
- Update feature list
- Update test count

**6.4: Create CHANGELOG**

**Detailed Implementation:**

**Step 1: Create CHANGELOG.md**
```markdown
# Changelog

All notable changes to TotalImage will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2025-12-21

### Added
- **E01 Write Support**: Full implementation of E01 image creation
  - Multi-segment file support (.E01, .E02, etc.)
  - Zlib/deflate compression
  - MD5 hash calculation
  - Adler-32 checksum for sections
  
- **GPT Backup Header Validation**: Read and validate backup GPT headers
  - Backup header CRC32 verification
  - Critical field comparison
  - Warning logging for mismatches
  
- **ISO Joliet Extension**: Support for Joliet supplementary volume descriptors
  - UTF-16BE filename decoding
  - Version separator handling (/)
  - Automatic fallback to ISO-9660
  
- **CLI Hash Command**: Standalone file hashing utility
  - Support for MD5, SHA1, SHA256
  - Progress reporting for large files
  - Multiple output formats
  
- **exFAT/ISO Extraction**: Web API support for exFAT and ISO filesystems
  - File listing for exFAT and ISO
  - File extraction endpoints
  - Timestamp conversion for exFAT
  
- **Property-Based Testing**: Proptest framework integration
  - VHD footer roundtrip tests
  - GPT header roundtrip tests
  - FAT BPB roundtrip tests
  - MBR partition table tests

### Changed
- Updated test count: 322 → 414+ (target)
- Improved error messages across all modules
- Enhanced documentation

### Fixed
- Clippy warnings in ISO Joliet code
- E01 writer borrow checker issues
- Test coverage gaps

### Security
- All P0 and P1 security issues resolved
- Comprehensive input validation
- Memory allocation limits enforced

## [0.1.0] - 2025-11-01

### Added
- Initial release
- Core vault formats (Raw, VHD, E01, AFF4)
- Partition table support (MBR, GPT)
- Filesystem support (FAT, exFAT, ISO, NTFS)
- CLI interface
- Web API
- MCP server
```

**Step 2: Update Version in Cargo.toml**
```toml
# Cargo.toml (workspace root)
[workspace.package]
version = "1.0.0"  # Updated from 0.1.0
```

**Step 3: Create Git Tag**
```bash
git tag -a v1.0.0 -m "TotalImage v1.0.0 - 100% Feature Complete"
git push origin v1.0.0
```

**Total Effort:** 4 hours

---

## Part 7: Implementation Timeline

### Week 1: Complete P2 & Property Tests
- **Day 1-2:** Task 11.6 Property-Based Testing (8h)
- **Day 3-4:** Integration test expansion (8h)
- **Day 5:** Documentation updates (4h)
- **Total:** 20 hours

### Week 2: WinPE USB Completion
- **Day 1-2:** WIM extraction (8h)
- **Day 3:** Boot configuration (4h)
- **Day 4:** Driver injection (4h)
- **Day 5:** Testing & validation (4h)
- **Total:** 20 hours

### Week 3: Test Coverage Expansion
- **Day 1-2:** E01 writer tests (8h)
- **Day 3:** WinPE tests (6h)
- **Day 4:** Territory & zone tests (6h)
- **Day 5:** Web API & CLI tests (8h)
- **Total:** 28 hours

### Week 4: Final Polish & P3 (Optional)
- **Day 1:** CLI Tree command (4h)
- **Day 2-3:** Remaining unit tests (12h)
- **Day 4:** Final integration tests (4h)
- **Day 5:** 100% validation & release prep (4h)
- **Total:** 24 hours

**Total Timeline:** 4 weeks (92 hours for P1+P2, or 116 hours including P3 basics)

---

## Part 8: Success Metrics

### Definition of 100% Completion

**Functional Completeness:**
- ✅ All P0 security issues resolved
- ✅ All P1 security issues resolved
- ✅ All P2 features complete
- ⚠️ P3 features (optional, but CLI Tree recommended)

**Test Coverage:**
- ✅ 414+ unit tests passing
- ✅ 20+ integration tests passing
- ✅ 10+ property tests passing
- ✅ 5 fuzzing targets operational

**Feature Completeness:**
- ✅ All core vault formats (read + E01 write)
- ✅ All partition tables (MBR + GPT with backup validation)
- ✅ All core filesystems (FAT, exFAT, ISO with Joliet, NTFS)
- ✅ WinPE USB creation (100% functional)
- ✅ CLI commands (all core + hash)
- ✅ Web API (all endpoints)
- ✅ Web UI (complete)

**Production Readiness:**
- ✅ Security hardened (P0 + P1 complete)
- ✅ Comprehensive testing (unit + integration + property)
- ✅ Documentation complete
- ✅ CI/CD operational
- ✅ Infrastructure ready

---

## Part 9: Risk Management

### High Risk Items

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| WIM extraction complexity | Medium | High | Use existing libraries (wimlib, dism) |
| Property tests reveal bugs | Medium | Medium | Allocate buffer time for fixes |
| Test coverage scope creep | Low | Medium | Strict prioritization |

### Medium Risk Items

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| WinPE USB platform issues | Low | Medium | Platform-specific implementations |
| Performance at scale | Low | Medium | Benchmark early |

---

## Part 10: Next Steps

### Immediate Actions (This Week)

1. **Start Task 11.6: Property-Based Testing** (8 hours)
   - Set up proptest framework
   - Implement 4 property tests
   - Verify all tests passing

2. **Update Documentation** (4 hours)
   - Update gap analysis
   - Update dashboard
   - Update README

3. **Begin WinPE USB Completion** (8 hours)
   - Research WIM format
   - Start WIM extraction implementation

### This Month

4. **Complete WinPE USB** (16 hours remaining)
5. **Expand Test Coverage** (42 hours)
6. **Complete Integration Tests** (14 hours)

### Next Month (Optional)

7. **P3 Features** (prioritize CLI Tree + Video Tutorials)
8. **Final 100% Validation**
9. **Release Preparation**

---

## Conclusion

To achieve **100% completion** in all areas:

1. **Complete P2:** 1 task remaining (Property-Based Testing) - 8 hours
2. **Complete WinPE USB:** 3 tasks remaining - 16 hours
3. **Expand Test Coverage:** +92 unit tests, +14 integration tests - 56 hours
4. **Documentation:** Updates - 4 hours

**Total Effort for 100% (P1+P2):** ~84 hours (~2-3 weeks)

**Total Effort for 100% (including P3 basics):** ~104 hours (~3-4 weeks)

The path to 100% is clear, well-defined, and achievable within 3-4 weeks of focused development.

---

**Document Status:** Living document, updated as tasks complete  
**Last Updated:** December 21, 2025  
**Next Review:** After each major milestone
