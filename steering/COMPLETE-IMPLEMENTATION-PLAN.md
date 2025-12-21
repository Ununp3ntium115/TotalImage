# Complete Implementation Plan - TotalImage Project
**Created:** December 21, 2025 at 12:34:52 MST  
**Based On:** COMPLETE-GAP-ANALYSIS-REPORT.md  
**Purpose:** Detailed, executable plan to achieve 100% completion  
**Current Status:** ~95% complete, 322 tests passing  
**Target:** 100% feature complete, production-ready

---

## Executive Summary

This document provides a comprehensive, iteration-by-iteration implementation plan to complete all remaining work identified in the gap analysis. The plan is organized into 5 major iterations, each with detailed tasks, acceptance criteria, dependencies, and time estimates.

### Current State (December 21, 2025)

- ✅ **P0 Issues:** All resolved (100%)
- ⚠️ **P1 Issues:** 79% complete (3-4 issues remaining, ~54 hours)
- ⚠️ **P2 Issues:** 0% complete (~44 hours)
- ⚠️ **P3 Issues:** 0% complete (60+ hours)
- ✅ **Test Coverage:** 322 tests (exceeded 260+ target)
- ✅ **Documentation:** 8,000+ lines complete
- ✅ **Infrastructure:** Production-ready

### Remaining Work Summary

| Priority | Effort | Items | Status |
|----------|--------|-------|--------|
| P1 (High) | ~54 hours | 3 major items | ⚠️ In Progress |
| P2 (Medium) | ~44 hours | 6 items | ❌ Not Started |
| P3 (Low) | 60+ hours | 6+ items | ❌ Not Started |
| **Total** | **~158 hours** | **15+ items** | **~5% remaining** |

### Implementation Strategy

1. **Iteration 9:** Complete P1 security issues and integration tests (Week 1)
2. **Iteration 10:** Implement WinPE bootable USB (Weeks 2-3)
3. **Iteration 11:** Complete P2 features (Weeks 4-5)
4. **Iteration 12:** Add P3 enhancements (Weeks 6-7)
5. **Iteration 13:** Final polish and 100% validation (Week 8)

**Total Timeline:** 8 weeks (~158 hours for P1+P2, or ~218+ hours for all priorities)

---

## Iteration 9: Security Hardening & Integration Testing
**Duration:** 1 week (22 hours)  
**Priority:** P1 (High)  
**Goal:** Complete remaining security issues and establish comprehensive integration test coverage

---

### Task 9.1: Fix Remaining P1 Security Issues
**Duration:** 6 hours  
**Priority:** P1  
**Owner:** Security Team / Development Team

#### 9.1.1: Complete SEC-004 (Unsafe mmap Validation)
**Effort:** 2 hours  
**Location:** `crates/totalimage-pipeline/src/mmap.rs`

**Current State:**
```rust
// mmap.rs:38,45
let mmap = unsafe { Mmap::map(&file)? };
```

**Implementation Steps:**

1. **Add file type validation** (30 min)
   ```rust
   use std::os::unix::fs::MetadataExt;
   
   pub fn open(path: &Path) -> io::Result<Self> {
       let file = File::open(path)?;
       let metadata = file.metadata()?;
       
       // Validate file is regular file (not device, pipe, etc.)
       #[cfg(unix)]
       if !metadata.is_file() {
           return Err(io::Error::new(
               io::ErrorKind::InvalidInput,
               "Only regular files can be memory-mapped"
           ));
       }
   ```

2. **Add file size validation** (30 min)
   ```rust
       // Check file size is reasonable
       const MAX_MMAP_SIZE: u64 = 16 * 1024 * 1024 * 1024; // 16 GB
       
       if metadata.len() > MAX_MMAP_SIZE {
           return Err(io::Error::new(
               io::ErrorKind::InvalidInput,
               format!("File too large for memory mapping (max {} GB)", MAX_MMAP_SIZE / (1024*1024*1024))
           ));
       }
   ```

3. **Add documentation** (30 min)
   ```rust
   /// Memory-mapped file I/O for efficient random access.
   ///
   /// # Safety
   ///
   /// This uses `unsafe` for memory mapping via `memmap2`. The following
   /// safety guarantees are provided:
   /// - File is validated as a regular file (not device/pipe)
   /// - File size is limited to 16 GB to prevent excessive memory usage
   /// - File is opened read-only to prevent concurrent modification
   /// - File descriptor is kept open for the lifetime of the mapping
   ///
   /// # Limitations
   ///
   /// - Files larger than 16 GB will use streaming I/O instead
   /// - Concurrent file modification may cause SIGBUS
   /// - Not suitable for network-mounted filesystems (use streaming)
   ```

4. **Add signal handling documentation** (30 min)
   - Document SIGBUS handling requirements
   - Add notes about TOCTOU risks
   - Recommend read-only file access

**Acceptance Criteria:**
- [ ] File type validation prevents mapping devices/pipes
- [ ] File size limit enforced (16 GB)
- [ ] Comprehensive safety documentation added
- [ ] Unit tests for validation logic
- [ ] No regressions in existing tests

**Dependencies:** None  
**Risk:** Low (validation only, no behavior change)

---

#### 9.1.2: Fix SEC-008 (Cache Size Overflow)
**Effort:** 2 hours  
**Location:** `crates/totalimage-web/src/cache.rs:296-300`

**Current State:**
```rust
// cache.rs:296-300
let mut total_bytes = 0u64;
total_bytes += vault_table.len()? * 1024;  // ← Can overflow
total_bytes += zone_table.len()? * 2048;
total_bytes += dir_table.len()? * 512;
```

**Implementation Steps:**

1. **Replace with checked arithmetic** (1 hour)
   ```rust
   let mut total_bytes = 0u64;
   
   // Use saturating arithmetic to prevent overflow
   total_bytes = total_bytes.saturating_add(
       vault_table.len()?.saturating_mul(1024)
   );
   total_bytes = total_bytes.saturating_add(
       zone_table.len()?.saturating_mul(2048)
   );
   total_bytes = total_bytes.saturating_add(
       dir_table.len()?.saturating_mul(512)
   );
   
   // Log warning if saturation occurred
   if total_bytes == u64::MAX {
       tracing::warn!("Cache size calculation saturated - cache may be larger than estimated");
   }
   ```

2. **Add unit test** (30 min)
   ```rust
   #[test]
   fn test_cache_size_overflow_protection() {
       // Create scenario that would overflow
       // Verify saturating arithmetic prevents panic
   }
   ```

3. **Update documentation** (30 min)
   - Document saturation behavior
   - Explain why saturating vs. checked arithmetic

**Acceptance Criteria:**
- [ ] No integer overflow possible in cache size calculation
- [ ] Saturation behavior documented
- [ ] Unit test validates overflow protection
- [ ] No performance regression

**Dependencies:** None  
**Risk:** Low (defensive programming)

---

#### 9.1.3: Complete SEC-011 (Timeout Handling)
**Effort:** 2 hours  
**Location:** `crates/totalimage-territories/src/fat/mod.rs`, `crates/totalimage-vaults/src/vhd/mod.rs`

**Current State:**
- FAT cluster chain traversal has max_clusters limit (good)
- VHD dynamic pipeline has no timeout/iteration limits

**Implementation Steps:**

1. **Add iteration limits to VHD block reading** (1 hour)
   ```rust
   // vhd/mod.rs:228-261
   const MAX_BLOCK_ITERATIONS: usize = 100_000; // Prevent infinite loops
   
   impl VhdDynamicPipeline {
       fn read(&mut self, offset: u64, length: usize) -> Result<Vec<u8>> {
           let mut iterations = 0;
           // ... existing code ...
           
           while /* condition */ {
               iterations += 1;
               if iterations > MAX_BLOCK_ITERATIONS {
                   return Err(Error::invalid_vault(
                       format!("Block reading exceeded maximum iterations ({}), possible circular reference", MAX_BLOCK_ITERATIONS)
                   ));
               }
               // ... rest of loop ...
           }
       }
   }
   ```

2. **Add timeout to directory parsing** (30 min)
   - Add max_depth limit to recursive directory operations
   - Document timeout behavior

3. **Add tests** (30 min)
   ```rust
   #[test]
   fn test_vhd_block_iteration_limit() {
       // Create VHD that would cause infinite loop
       // Verify error is returned at limit
   }
   ```

**Acceptance Criteria:**
- [ ] VHD block reading has iteration limit
- [ ] Directory parsing has depth limit
- [ ] Tests validate timeout behavior
- [ ] Error messages are descriptive

**Dependencies:** None  
**Risk:** Low (defensive limits)

---

### Task 9.2: Complete Integration Test Suite
**Duration:** 16 hours  
**Priority:** P1  
**Owner:** QA Team / Development Team

#### 9.2.1: VHD → FAT32 → Extract Pipeline Test
**Effort:** 4 hours  
**Location:** `tests/integration/test_vhd_fat32_pipeline.rs`

**Implementation Steps:**

1. **Create test fixture** (1 hour)
   ```rust
   // Create test VHD with FAT32 partition
   // Use existing test infrastructure or create new fixture
   fn create_test_vhd_fat32() -> PathBuf {
       // Generate or use existing test image
   }
   ```

2. **Implement full pipeline test** (2 hours)
   ```rust
   #[tokio::test]
   async fn test_extract_file_from_vhd_fat32() {
       // 1. Open VHD vault
       let vault_path = create_test_vhd_fat32();
       let mut vault = open_vault(&vault_path, VaultConfig::default())?;
       
       // 2. Parse partition table
       let zones = parse_zone_table(vault.content())?;
       assert!(zones.len() > 0);
       
       // 3. Open FAT32 filesystem
       let partition = PartialPipeline::new(
           vault.content(),
           zones[0].offset,
           zones[0].length
       );
       let mut territory = FatTerritory::parse(partition)?;
       
       // 4. List files
       let root = territory.headquarters()?;
       let files = root.list_occupants()?;
       assert!(files.len() > 0);
       
       // 5. Extract file
       let test_file = "AUTOEXEC.BAT";
       let data = territory.extract_file(test_file)?;
       assert!(data.len() > 0);
       
       // 6. Verify file contents
       // ... validation ...
   }
   ```

3. **Add edge case tests** (1 hour)
   - Empty partition
   - Corrupted FAT
   - Large files (>100MB)
   - Deep directory structures

**Acceptance Criteria:**
- [ ] Full pipeline test passes
- [ ] Edge cases covered
- [ ] Test runs in CI/CD
- [ ] Test fixture is reusable

**Dependencies:** Test fixtures  
**Risk:** Medium (may reveal integration bugs)

---

#### 9.2.2: E01 → NTFS → Extract Pipeline Test
**Effort:** 4 hours  
**Location:** `tests/integration/test_e01_ntfs_pipeline.rs`

**Implementation Steps:**

1. **Create E01 test fixture** (1.5 hours)
   - Generate or obtain test E01 image with NTFS partition
   - Document fixture creation process

2. **Implement pipeline test** (2 hours)
   ```rust
   #[tokio::test]
   async fn test_extract_file_from_e01_ntfs() {
       // Similar structure to VHD test
       // Test E01 decompression + NTFS parsing + extraction
   }
   ```

3. **Add NTFS-specific tests** (30 min)
   - Alternate Data Streams
   - Sparse files
   - Compressed files (LZNT1)

**Acceptance Criteria:**
- [ ] E01 → NTFS pipeline test passes
- [ ] NTFS-specific features tested
- [ ] Test runs in CI/CD

**Dependencies:** E01 test fixture  
**Risk:** Medium

---

#### 9.2.3: AFF4 → exFAT → Extract Pipeline Test
**Effort:** 4 hours  
**Location:** `tests/integration/test_aff4_exfat_pipeline.rs`

**Implementation Steps:**

1. **Create AFF4 test fixture** (1.5 hours)
   - Generate AFF4 image with exFAT partition
   - Test all compression methods (Snappy, LZ4, Deflate)

2. **Implement pipeline test** (2 hours)
   ```rust
   #[tokio::test]
   async fn test_extract_file_from_aff4_exfat() {
       // Test AFF4 decompression + exFAT parsing + extraction
   }
   ```

3. **Add compression method tests** (30 min)
   - Test each compression method separately
   - Verify LRU cache behavior

**Acceptance Criteria:**
- [ ] AFF4 → exFAT pipeline test passes
- [ ] All compression methods tested
- [ ] Cache behavior validated

**Dependencies:** AFF4 test fixture  
**Risk:** Medium

---

#### 9.2.4: MCP Server End-to-End Test
**Effort:** 2 hours  
**Location:** `tests/integration/test_mcp_e2e.rs`

**Implementation Steps:**

1. **Set up MCP server test harness** (1 hour)
   ```rust
   async fn start_mcp_server() -> McpServerHandle {
       // Start MCP server in test mode
       // Return handle for cleanup
   }
   ```

2. **Test all 5 tools** (1 hour)
   ```rust
   #[tokio::test]
   async fn test_mcp_tools_e2e() {
       let server = start_mcp_server().await;
       
       // Test analyze_disk_image
       // Test list_partitions
       // Test list_files
       // Test extract_file
       // Test validate_integrity
   }
   ```

**Acceptance Criteria:**
- [ ] All MCP tools tested end-to-end
- [ ] Authentication tested
- [ ] Error handling validated

**Dependencies:** MCP server test infrastructure  
**Risk:** Low

---

#### 9.2.5: Fire Marshal → TotalImage Integration Test
**Effort:** 2 hours  
**Location:** `tests/integration/test_fire_marshal_integration.rs`

**Implementation Steps:**

1. **Set up Fire Marshal test environment** (1 hour)
   - Start Fire Marshal server
   - Register TotalImage MCP server
   - Set up test fixtures

2. **Test tool orchestration** (1 hour)
   ```rust
   #[tokio::test]
   async fn test_fire_marshal_orchestration() {
       // Test tool registration
       // Test tool execution via Fire Marshal
       // Test result caching
       // Test error propagation
   }
   ```

**Acceptance Criteria:**
- [ ] Fire Marshal integration tested
- [ ] Tool registration works
- [ ] Caching validated

**Dependencies:** Fire Marshal test infrastructure  
**Risk:** Low

---

### Iteration 9 Deliverables

- [ ] All remaining P1 security issues fixed (SEC-004, SEC-008, SEC-011)
- [ ] Complete integration test suite (5 test files)
- [ ] All integration tests passing in CI/CD
- [ ] Test coverage report updated
- [ ] Security audit checklist updated

**Exit Criteria:**
- All P1 security issues resolved
- Integration test suite operational
- CI/CD running all integration tests
- No regressions in existing tests

**Estimated Completion:** End of Week 1

---

## Iteration 10: WinPE Bootable USB Implementation
**Duration:** 2 weeks (32 hours)  
**Priority:** P1 (High)  
**Goal:** Implement WinPE bootable USB creation for FTK Imager parity

---

### Task 10.1: USB Drive Detection & Partitioning
**Duration:** 8 hours  
**Priority:** P1

#### 10.1.1: USB Drive Detection
**Effort:** 3 hours  
**Location:** `crates/totalimage-acquire/src/usb.rs` (new file)

**Implementation Steps:**

1. **Create USB detection module** (1 hour)
   ```rust
   // crates/totalimage-acquire/src/usb.rs
   use std::path::PathBuf;
   
   pub struct UsbDrive {
       pub device_path: PathBuf,
       pub size_bytes: u64,
       pub vendor: String,
       pub model: String,
       pub is_removable: bool,
   }
   
   pub fn detect_usb_drives() -> Result<Vec<UsbDrive>> {
       #[cfg(target_os = "windows")]
       {
           // Use Windows WMI or SetupAPI
           detect_usb_windows()
       }
       
       #[cfg(target_os = "linux")]
       {
           // Parse /sys/block and /dev/disk/by-id
           detect_usb_linux()
       }
       
       #[cfg(target_os = "macos")]
       {
           // Use diskutil or IOKit
           detect_usb_macos()
       }
   }
   ```

2. **Implement platform-specific detection** (1.5 hours)
   - Windows: WMI query or SetupAPI enumeration
   - Linux: Parse /sys/block, check removable flag
   - macOS: Use diskutil list or IOKit

3. **Add safety checks** (30 min)
   - Warn if drive is not removable
   - Require confirmation for drives > 1TB
   - List all detected drives for user selection

**Acceptance Criteria:**
- [ ] USB drives detected on Windows, Linux, macOS
- [ ] Removable drives identified correctly
- [ ] Drive information displayed (size, vendor, model)
- [ ] Safety warnings for non-removable drives
- [ ] Unit tests for detection logic

**Dependencies:** Platform-specific APIs  
**Risk:** Medium (platform differences)

---

#### 10.1.2: Partition Table Creation
**Effort:** 3 hours  
**Location:** `crates/totalimage-acquire/src/partition.rs` (new file)

**Implementation Steps:**

1. **Create partition table builder** (1.5 hours)
   ```rust
   pub enum PartitionTableType {
       Mbr,
       Gpt,
   }
   
   pub struct PartitionTableBuilder {
       table_type: PartitionTableType,
       sector_size: u32,
   }
   
   impl PartitionTableBuilder {
       pub fn create_mbr(&self, boot_partition_size: u64) -> Result<Vec<u8>> {
           // Create MBR with single bootable partition
           // Set boot flag (0x80)
           // Set partition type (0x0C for FAT32)
       }
       
       pub fn create_gpt(&self, boot_partition_size: u64) -> Result<Vec<u8>> {
           // Create GPT with protective MBR
           // Create GPT header
           // Create partition entry for boot partition
           // Set partition type GUID (EBD0A0A2-B9E5-4433-87C0-68B6B72699C7 for FAT32)
       }
   }
   ```

2. **Implement MBR creation** (1 hour)
   - Write boot signature (0xAA55)
   - Create partition entry
   - Set CHS values

3. **Implement GPT creation** (30 min)
   - Create protective MBR
   - Write GPT header with CRC32
   - Create partition entry array

**Acceptance Criteria:**
- [ ] MBR partition table created correctly
- [ ] GPT partition table created correctly
- [ ] Boot flags set appropriately
- [ ] Partition tables validated by existing parsers
- [ ] Unit tests for both table types

**Dependencies:** totalimage-zones crate (for validation)  
**Risk:** Low (well-defined formats)

---

#### 10.1.3: FAT32 Formatting
**Effort:** 2 hours  
**Location:** `crates/totalimage-acquire/src/format.rs` (new file)

**Implementation Steps:**

1. **Create FAT32 formatter** (1.5 hours)
   ```rust
   pub struct Fat32Formatter {
       sector_size: u32,
       sectors_per_cluster: u8,
   }
   
   impl Fat32Formatter {
       pub fn format(&self, device: &mut File, partition_offset: u64, size: u64) -> Result<()> {
           // 1. Write boot sector (BPB)
           // 2. Write FSInfo sector
           // 3. Initialize FAT tables (all zeros except reserved entries)
           // 4. Initialize root directory (empty)
           // 5. Set volume label
       }
   }
   ```

2. **Implement BPB writing** (30 min)
   - Calculate cluster count
   - Set FAT size
   - Write boot signature

**Acceptance Criteria:**
- [ ] FAT32 filesystem created correctly
   - [ ] Boot sector valid
   - [ ] FAT tables initialized
   - [ ] Root directory empty
   - [ ] Volume label set
- [ ] Formatted partition readable by existing FAT parser
- [ ] Unit tests validate formatting

**Dependencies:** totalimage-territories crate (for validation)  
**Risk:** Medium (FAT32 formatting is complex)

---

### Task 10.2: WinPE Deployment
**Duration:** 16 hours  
**Priority:** P1

#### 10.2.1: WinPE Source Detection
**Effort:** 3 hours  
**Location:** `crates/totalimage-acquire/src/winpe.rs` (new file)

**Implementation Steps:**

1. **Detect Windows ADK installation** (1.5 hours)
   ```rust
   pub fn find_winpe_source() -> Result<PathBuf> {
       // Check common ADK installation paths:
       // Windows: C:\Program Files (x86)\Windows Kits\10\Assessment and Deployment Kit\Windows Preinstallation Environment\
       // Or: C:\Program Files\Windows Kits\10\Assessment and Deployment Kit\Windows Preinstallation Environment\
       
       // Look for boot.wim in:
       // {ADK_PATH}\amd64\Media\sources\boot.wim
       // {ADK_PATH}\x86\Media\sources\boot.wim
   }
   ```

2. **Validate WinPE source** (1 hour)
   - Check boot.wim exists
   - Verify WIM file format
   - Check architecture (amd64 vs x86)

3. **Add user override** (30 min)
   - Allow manual path specification
   - Validate user-provided path

**Acceptance Criteria:**
- [ ] ADK installation detected automatically
- [ ] boot.wim file located
- [ ] Architecture detected (amd64/x86)
- [ ] Manual path override supported
- [ ] Clear error messages if not found

**Dependencies:** Windows ADK (external)  
**Risk:** Medium (requires ADK installation)

---

#### 10.2.2: WIM File Extraction
**Effort:** 5 hours  
**Location:** `crates/totalimage-acquire/src/winpe.rs`

**Implementation Steps:**

1. **Research WIM format** (1 hour)
   - WIM (Windows Imaging Format) specification
   - Use existing library or implement parser
   - Consider `wimlib` bindings or native implementation

2. **Extract boot.wim to USB** (3 hours)
   ```rust
   pub fn extract_wim_to_usb(wim_path: &Path, usb_root: &Path) -> Result<()> {
       // 1. Parse WIM file
       // 2. Extract files to USB root
       // 3. Preserve directory structure
       // 4. Set file attributes
   }
   ```

3. **Handle WIM compression** (1 hour)
   - WIM files may be LZX-compressed
   - Decompress during extraction

**Acceptance Criteria:**
- [ ] boot.wim extracted to USB drive
- [ ] Directory structure preserved
- [ ] File attributes set correctly
- [ ] Extraction progress reported
- [ ] Error handling for corrupted WIM files

**Dependencies:** WIM format library or implementation  
**Risk:** High (WIM format complexity)

---

#### 10.2.3: Boot Configuration
**Effort:** 4 hours  
**Location:** `crates/totalimage-acquire/src/winpe.rs`

**Implementation Steps:**

1. **Create BCD (Boot Configuration Data)** (2 hours)
   ```rust
   pub fn create_bcd(usb_root: &Path, boot_wim_path: &Path) -> Result<()> {
       // Create \Boot\BCD file
       // Configure boot entry for boot.wim
       // Set boot parameters
   }
   ```

2. **Set up boot files** (1.5 hours)
   - Copy bootmgr to USB root
   - Create \Boot directory structure
   - Set up boot.ini or BCD

3. **Make partition bootable** (30 min)
   - Set active partition flag (MBR)
   - Or set EFI system partition flag (GPT)

**Acceptance Criteria:**
- [ ] USB drive boots to WinPE
- [ ] Boot configuration valid
- [ ] Works on both BIOS and UEFI systems
- [ ] Tested on physical hardware

**Dependencies:** BCD format knowledge  
**Risk:** High (boot configuration is complex)

---

#### 10.2.4: Driver Injection Framework
**Effort:** 4 hours  
**Location:** `crates/totalimage-acquire/src/winpe.rs`

**Implementation Steps:**

1. **Design driver injection API** (1 hour)
   ```rust
   pub struct DriverInjector {
       winpe_path: PathBuf,
   }
   
   impl DriverInjector {
       pub fn inject_driver(&self, driver_path: &Path) -> Result<()> {
           // Add driver to WinPE image
           // Update driver store
           // Register driver in registry
       }
   }
   ```

2. **Implement driver injection** (2 hours)
   - Mount WIM file (or extract/modify/repack)
   - Copy driver files to appropriate locations
   - Update driver database

3. **Add driver validation** (1 hour)
   - Verify driver architecture matches WinPE
   - Check driver signatures
   - Validate driver dependencies

**Acceptance Criteria:**
- [ ] Drivers can be injected into WinPE
- [ ] Driver architecture validated
- [ ] Injected drivers load in WinPE
- [ ] Error handling for invalid drivers

**Dependencies:** WIM mounting/editing capability  
**Risk:** High (requires WIM modification)

---

### Task 10.3: TotalImage CLI Integration
**Duration:** 4 hours  
**Priority:** P1

#### 10.3.1: Add CLI Command
**Effort:** 2 hours  
**Location:** `crates/totalimage-cli/src/main.rs`

**Implementation Steps:**

1. **Add create-winpe-usb command** (1 hour)
   ```rust
   // In main.rs command matching
   "create-winpe-usb" => {
       cmd_create_winpe_usb(&args[1..])?;
   }
   ```

2. **Implement command handler** (1 hour)
   ```rust
   fn cmd_create_winpe_usb(args: &[String]) -> Result<()> {
       // Parse arguments:
       // --usb-device <path> (required)
       // --winpe-source <path> (optional, auto-detect)
       // --driver <path> (optional, repeatable)
       // --partition-table <mbr|gpt> (optional, default: gpt)
       
       // 1. Detect USB drive
       // 2. Confirm with user
       // 3. Create partition table
       // 4. Format FAT32
       // 5. Extract WinPE
       // 6. Configure boot
       // 7. Inject drivers (if any)
   }
   ```

**Acceptance Criteria:**
- [ ] CLI command added
- [ ] All options supported
- [ ] User confirmation required
- [ ] Progress reporting
- [ ] Error messages clear

**Dependencies:** Tasks 10.1, 10.2  
**Risk:** Low

---

#### 10.3.2: Add Tests
**Effort:** 2 hours  
**Location:** `crates/totalimage-acquire/src/winpe.rs` (tests)

**Implementation Steps:**

1. **Unit tests for USB detection** (30 min)
   - Mock platform-specific APIs
   - Test detection logic

2. **Unit tests for partition creation** (30 min)
   - Test MBR creation
   - Test GPT creation
   - Validate with existing parsers

3. **Integration test (manual)** (1 hour)
   - Document manual testing procedure
   - Test on physical USB drive
   - Verify bootability

**Acceptance Criteria:**
- [ ] Unit tests pass
- [ ] Manual testing procedure documented
- [ ] Tested on physical hardware

**Dependencies:** Tasks 10.1, 10.2, 10.3.1  
**Risk:** Low

---

### Iteration 10 Deliverables

- [ ] USB drive detection working (Windows, Linux, macOS)
- [ ] Partition table creation (MBR and GPT)
- [ ] FAT32 formatting
- [ ] WinPE source detection
- [ ] WIM file extraction
- [ ] Boot configuration
- [ ] Driver injection framework
- [ ] CLI command `create-winpe-usb`
- [ ] Documentation for WinPE USB creation

**Exit Criteria:**
- WinPE bootable USB can be created via CLI
- USB boots to WinPE on physical hardware
- Drivers can be injected
- All tests passing

**Estimated Completion:** End of Week 3

**Note:** This is the most complex feature. Consider breaking into smaller iterations if complexity is higher than estimated.

---

## Iteration 11: P2 Feature Completion
**Duration:** 2 weeks (44 hours)  
**Priority:** P2 (Medium)  
**Goal:** Complete all medium-priority features

---

### Task 11.1: GPT Backup Header Validation
**Duration:** 8 hours  
**Priority:** P2

#### 11.1.1: Implement Backup Header Reading
**Effort:** 3 hours  
**Location:** `crates/totalimage-zones/src/gpt/mod.rs`

**Implementation Steps:**

1. **Add backup header reading** (1.5 hours)
   ```rust
   impl GptZoneTable {
       pub fn read_backup_header(&self, device: &mut dyn ReadSeek) -> Result<GptHeader> {
           // Read backup header from last sector of disk
           // Validate signature
           // Verify CRC32
       }
   }
   ```

2. **Compare primary and backup headers** (1 hour)
   ```rust
   pub fn validate_backup_header(&self, primary: &GptHeader, backup: &GptHeader) -> Result<()> {
       // Compare critical fields:
       // - Disk GUID
       // - Partition entry LBA
       // - Number of partitions
       // - Partition entry size
       
       // If mismatch, log warning but don't fail (backup may be stale)
   }
   ```

3. **Add validation option** (30 min)
   ```rust
   pub struct GptConfig {
       pub validate_backup_header: bool, // Default: true
   }
   ```

**Acceptance Criteria:**
- [ ] Backup header read from disk end
- [ ] Backup header CRC32 validated
- [ ] Primary/backup comparison implemented
- [ ] Configurable validation
- [ ] Unit tests added

**Dependencies:** None  
**Risk:** Low

---

#### 11.1.2: Add Tests
**Effort:** 2 hours

**Implementation Steps:**

1. **Test backup header reading** (1 hour)
   - Create test GPT disk with backup header
   - Verify reading works
   - Test corrupted backup header handling

2. **Test validation logic** (1 hour)
   - Test matching headers
   - Test mismatched headers
   - Test missing backup header

**Acceptance Criteria:**
- [ ] All tests passing
- [ ] Edge cases covered

**Dependencies:** Task 11.1.1  
**Risk:** Low

---

### Task 11.2: ISO Joliet Extension
**Duration:** 8 hours  
**Priority:** P2

#### 11.2.1: Implement Joliet Parser
**Effort:** 5 hours  
**Location:** `crates/totalimage-territories/src/iso/joliet.rs` (new file)

**Implementation Steps:**

1. **Create Joliet types** (1 hour)
   ```rust
   pub struct JolietDescriptor {
       pub escape_sequences: [u8; 3],
       pub volume_identifier: String, // UTF-16BE
       pub system_identifier: String,
   }
   
   pub struct JolietDirectoryRecord {
       pub identifier: String, // UTF-16BE
       // ... other fields similar to ISO-9660
   }
   ```

2. **Parse supplementary volume descriptor** (2 hours)
   ```rust
   impl IsoTerritory {
       fn parse_joliet_descriptor(&mut self, sector: u32) -> Result<JolietDescriptor> {
           // Read sector 17 (supplementary volume descriptor)
           // Check escape sequences: 0x25, 0x2F, 0x40-0x4F
           // Parse UTF-16BE strings
       }
   }
   ```

3. **Integrate with directory parsing** (2 hours)
   ```rust
   impl IsoTerritory {
       fn parse_directory_record_joliet(&self, data: &[u8]) -> Result<DirectoryRecord> {
           // Parse directory record with UTF-16BE identifier
           // Use Joliet escape sequences
       }
   }
   ```

**Acceptance Criteria:**
- [ ] Joliet descriptor parsed correctly
- [ ] UTF-16BE filenames decoded
- [ ] Directory records use Joliet names
- [ ] Falls back to ISO-9660 if Joliet not present
- [ ] Unit tests added

**Dependencies:** totalimage-territories ISO implementation  
**Risk:** Medium (UTF-16BE encoding complexity)

---

#### 11.2.2: Add Tests
**Effort:** 3 hours

**Implementation Steps:**

1. **Create Joliet test fixture** (1.5 hours)
   - Generate or obtain ISO with Joliet extension
   - Document fixture creation

2. **Test Joliet parsing** (1.5 hours)
   - Test descriptor parsing
   - Test filename decoding
   - Test directory navigation
   - Test fallback to ISO-9660

**Acceptance Criteria:**
- [ ] All tests passing
- [ ] Real-world Joliet ISOs tested

**Dependencies:** Task 11.2.1  
**Risk:** Low

---

### Task 11.3: E01 Write Support
**Duration:** 16 hours  
**Priority:** P2

#### 11.3.1: Design E01 Writer Architecture
**Effort:** 2 hours

**Implementation Steps:**

1. **Research E01 format specification** (1 hour)
   - E01 file structure
   - Section types and ordering
   - CRC32 calculation
   - Multi-segment support

2. **Design writer API** (1 hour)
   ```rust
   pub struct E01Writer {
       output_path: PathBuf,
       segment_size: u64,
       compression: CompressionType,
   }
   
   impl E01Writer {
       pub fn new(output_path: PathBuf) -> Self;
       pub fn write_sector(&mut self, sector_data: &[u8]) -> Result<()>;
       pub fn finalize(&mut self) -> Result<()>;
   }
   ```

**Acceptance Criteria:**
- [ ] E01 format understood
- [ ] Writer API designed
- [ ] Architecture documented

**Dependencies:** E01 format specification  
**Risk:** Medium (format complexity)

---

#### 11.3.2: Implement E01 Writer
**Effort:** 10 hours  
**Location:** `crates/totalimage-acquire/src/e01_writer.rs` (new file)

**Implementation Steps:**

1. **Implement header writing** (2 hours)
   ```rust
   fn write_header(&mut self) -> Result<()> {
       // Write E01 header section
       // Set format version
       // Set compression type
   }
   ```

2. **Implement sector writing** (4 hours)
   ```rust
   fn write_sector(&mut self, sector_data: &[u8]) -> Result<()> {
       // Compress sector (zlib)
       // Write compressed data
       // Update sector table
       // Calculate CRC32
   }
   ```

3. **Implement multi-segment support** (2 hours)
   - Split into segments when size limit reached
   - Create new segment file
   - Update segment table

4. **Implement finalization** (2 hours)
   ```rust
   fn finalize(&mut self) -> Result<()> {
       // Write sector table
       // Write hash section (MD5/SHA1)
       // Write terminator section
       // Close file
   }
   ```

**Acceptance Criteria:**
- [ ] E01 files created correctly
- [ ] Compression working (zlib)
- [ ] Multi-segment support
- [ ] Hash calculation (MD5/SHA1)
- [ ] Created files readable by existing E01 parser
- [ ] Unit tests added

**Dependencies:** Task 11.3.1  
**Risk:** High (E01 format complexity)

---

#### 11.3.3: Add Tests
**Effort:** 4 hours

**Implementation Steps:**

1. **Test single-segment E01** (1.5 hours)
   - Create small E01 file
   - Verify readable by parser
   - Verify hash matches

2. **Test multi-segment E01** (1.5 hours)
   - Create large E01 file (>segment size)
   - Verify all segments created
   - Verify readable by parser

3. **Test compression** (1 hour)
   - Verify zlib compression working
   - Verify decompression matches original

**Acceptance Criteria:**
- [ ] All tests passing
- [ ] Roundtrip tests (write then read)

**Dependencies:** Task 11.3.2  
**Risk:** Low

---

### Task 11.4: CLI Hash Command
**Duration:** 4 hours  
**Priority:** P2

#### 11.4.1: Implement Hash Command
**Effort:** 3 hours  
**Location:** `crates/totalimage-cli/src/main.rs`

**Implementation Steps:**

1. **Add hash command** (1 hour)
   ```rust
   "hash" => {
       cmd_hash(&args[1..])?;
   }
   ```

2. **Implement hash calculation** (2 hours)
   ```rust
   fn cmd_hash(args: &[String]) -> Result<()> {
       // Parse arguments:
       // <file> (required)
       // --algorithm <md5|sha1|sha256> (optional, default: sha256)
       // --output <format> (optional, default: hex)
       
       // Read file
       // Calculate hash
       // Output in requested format
   }
   ```

**Acceptance Criteria:**
- [ ] Hash command implemented
- [ ] MD5, SHA1, SHA256 supported
   - [ ] Output formats: hex, base64
   - [ ] Progress reporting for large files
   - [ ] Unit tests added

**Dependencies:** Existing hash infrastructure (from acquire crate)  
**Risk:** Low

---

#### 11.4.2: Add Tests
**Effort:** 1 hour

**Implementation Steps:**

1. **Test hash calculation** (30 min)
   - Test known file hashes
   - Verify algorithm selection
   - Test output formats

2. **Test error handling** (30 min)
   - Test missing file
   - Test invalid algorithm
   - Test invalid output format

**Acceptance Criteria:**
- [ ] All tests passing

**Dependencies:** Task 11.4.1  
**Risk:** Low

---

### Task 11.5: exFAT/ISO Extraction Support
**Duration:** 4 hours  
**Priority:** P2

#### 11.5.1: Add exFAT Extraction
**Effort:** 2 hours  
**Location:** `crates/totalimage-web/src/main.rs`

**Implementation Steps:**

1. **Extend extract_file_from_vault** (1.5 hours)
   ```rust
   fn extract_file_from_vault(...) -> Result<Vec<u8>> {
       // ... existing FAT/NTFS code ...
       
       // Add exFAT support
       if let Ok(mut territory) = ExfatTerritory::parse(partition) {
           return territory.extract_file(&file_path)?;
       }
   }
   ```

2. **Add tests** (30 min)
   - Test exFAT file extraction
   - Test error handling

**Acceptance Criteria:**
- [ ] exFAT extraction working in Web API
- [ ] Tests added
- [ ] No regressions

**Dependencies:** Existing exFAT territory implementation  
**Risk:** Low

---

#### 11.5.2: Add ISO Extraction
**Effort:** 2 hours  
**Location:** `crates/totalimage-web/src/main.rs`

**Implementation Steps:**

1. **Extend extract_file_from_vault** (1.5 hours)
   ```rust
   // Add ISO support
   if let Ok(mut territory) = IsoTerritory::parse(partition) {
       return territory.extract_file(&file_path)?;
   }
   ```

2. **Add tests** (30 min)
   - Test ISO file extraction
   - Test Joliet filename handling

**Acceptance Criteria:**
- [ ] ISO extraction working in Web API
- [ ] Tests added
- [ ] No regressions

**Dependencies:** Existing ISO territory implementation  
**Risk:** Low

---

### Task 11.6: Property-Based Testing
**Duration:** 8 hours  
**Priority:** P2

#### 11.6.1: Set Up Proptest
**Effort:** 2 hours

**Implementation Steps:**

1. **Add proptest dependency** (30 min)
   ```toml
   [dev-dependencies]
   proptest = "1.4"
   ```

2. **Create property test infrastructure** (1.5 hours)
   - Set up proptest configuration
   - Create test utilities
   - Document property testing approach

**Acceptance Criteria:**
- [ ] Proptest configured
- [ ] Test infrastructure ready

**Dependencies:** None  
**Risk:** Low

---

#### 11.6.2: Implement Property Tests
**Effort:** 6 hours

**Implementation Steps:**

1. **VHD footer roundtrip test** (1.5 hours)
   ```rust
   proptest! {
       #[test]
       fn test_vhd_footer_roundtrip(
           current_size in 1u64..1_000_000_000,
           disk_type in 0u32..3u32,
       ) {
           let footer = create_test_footer(current_size, disk_type);
           let mut bytes = [0u8; 512];
           footer.serialize(&mut bytes);
           let parsed = VhdFooter::parse(&bytes).unwrap();
           prop_assert_eq!(parsed.current_size, footer.current_size);
       }
   }
   ```

2. **GPT header roundtrip test** (1.5 hours)
   - Test GPT header serialization/parsing
   - Verify CRC32 calculation

3. **FAT BPB roundtrip test** (1.5 hours)
   - Test BPB serialization/parsing
   - Verify cluster calculations

4. **MBR partition table test** (1.5 hours)
   - Test MBR creation/parsing
   - Verify partition calculations

**Acceptance Criteria:**
- [ ] Property tests for all major formats
- [ ] Tests discover edge cases
- [ ] All tests passing

**Dependencies:** Task 11.6.1  
**Risk:** Medium (may reveal bugs)

---

### Iteration 11 Deliverables

- [ ] GPT backup header validation
- [ ] ISO Joliet extension support
- [ ] E01 write support
- [ ] CLI hash command
- [ ] exFAT/ISO extraction in Web API
- [ ] Property-based testing infrastructure

**Exit Criteria:**
- All P2 features complete
- All tests passing
- No regressions

**Estimated Completion:** End of Week 5

---

## Iteration 12: P3 Enhancements (Optional)
**Duration:** 2 weeks (60+ hours)  
**Priority:** P3 (Low)  
**Goal:** Add nice-to-have features

---

### Task 12.1: CLI Tree Command
**Duration:** 4 hours  
**Priority:** P3

**Implementation Steps:**

1. **Add tree command** (2 hours)
   ```rust
   "tree" => {
       cmd_tree(&args[1..])?;
   }
   
   fn cmd_tree(args: &[String]) -> Result<()> {
       // Parse arguments
       // Load vault and territory
       // Recursively print directory tree
       // Use ASCII tree characters (│ ├ └)
   }
   ```

2. **Add tests** (1 hour)

3. **Add documentation** (1 hour)

**Acceptance Criteria:**
- [ ] Tree command implemented
- [ ] Works with all filesystems
- [ ] Tests added

**Dependencies:** None  
**Risk:** Low

---

### Task 12.2: AFF4 Write Support
**Duration:** Unknown (estimate 20+ hours)  
**Priority:** P3

**Note:** This is complex and may be deferred. Requires:
- AFF4 format specification research
- Writer implementation
- Compression support (Snappy, LZ4, Deflate)
- Multi-bevy support
- Turtle RDF metadata writing

**Recommendation:** Defer to future release unless specifically requested.

---

### Task 12.3: ext2/3/4 Filesystem Support
**Duration:** 40 hours  
**Priority:** P3

**Implementation Steps:**

1. **Research ext2/3/4 formats** (4 hours)
2. **Implement ext2 reader** (16 hours)
3. **Add ext3 journal support** (8 hours)
4. **Add ext4 extensions** (8 hours)
5. **Add tests** (4 hours)

**Note:** This is a significant feature. Consider as separate project phase.

---

### Task 12.4: Video Tutorials
**Duration:** 16 hours  
**Priority:** P3

**Implementation Steps:**

1. **Script writing** (4 hours)
   - Quick start tutorial
   - CLI usage tutorial
   - Web UI tutorial
   - Advanced features tutorial

2. **Recording** (8 hours)
   - Screen recording
   - Voiceover
   - Editing

3. **Post-production** (4 hours)
   - Editing
   - Captions
   - Upload to hosting platform

**Acceptance Criteria:**
- [ ] 4+ video tutorials created
- [ ] Tutorials cover main use cases
- [ ] Videos hosted and linked from documentation

**Dependencies:** None  
**Risk:** Low

---

### Iteration 12 Deliverables

- [ ] CLI tree command (if prioritized)
- [ ] Video tutorials (if prioritized)
- [ ] Other P3 features as prioritized

**Note:** P3 features are optional and can be implemented based on user demand.

---

## Iteration 13: Final Polish & 100% Validation
**Duration:** 1 week (16 hours)  
**Priority:** P1  
**Goal:** Validate 100% completion and final polish

---

### Task 13.1: Comprehensive Testing
**Duration:** 6 hours

**Implementation Steps:**

1. **Run full test suite** (1 hour)
   - All unit tests
   - All integration tests
   - All property tests
   - Fuzzing targets

2. **Manual testing checklist** (2 hours)
   - [ ] WinPE USB creation on physical hardware
   - [ ] All CLI commands
   - [ ] Web UI all features
   - [ ] MCP server all tools
   - [ ] Fire Marshal integration

3. **Performance testing** (2 hours)
   - Benchmark large file operations
   - Memory usage profiling
   - Concurrent request testing

4. **Security audit** (1 hour)
   - Review all security fixes
   - Verify no regressions
   - Check for new vulnerabilities

**Acceptance Criteria:**
- [ ] All tests passing
- [ ] Manual testing complete
- [ ] Performance acceptable
- [ ] Security audit passed

---

### Task 13.2: Documentation Finalization
**Duration:** 4 hours

**Implementation Steps:**

1. **Update README** (1 hour)
   - Reflect 100% completion
   - Update feature list
   - Update test count

2. **Update CHANGELOG** (1 hour)
   - Document all new features
   - List all fixes
   - Version bump to 1.0.0

3. **Final documentation review** (2 hours)
   - Check all links
   - Verify examples work
   - Update outdated information

**Acceptance Criteria:**
- [ ] README updated
- [ ] CHANGELOG complete
- [ ] All documentation accurate

---

### Task 13.3: Release Preparation
**Duration:** 6 hours

**Implementation Steps:**

1. **Version bump** (1 hour)
   - Update Cargo.toml versions
   - Update package.json versions
   - Tag git release

2. **Build release artifacts** (2 hours)
   - Build all binaries
   - Create Docker images
   - Package distributions

3. **Release notes** (2 hours)
   - Write comprehensive release notes
   - Highlight new features
   - Migration guide (if needed)

4. **Deployment validation** (1 hour)
   - Test Docker deployment
   - Test Kubernetes deployment
   - Verify all services start

**Acceptance Criteria:**
- [ ] All versions bumped
- [ ] Release artifacts built
- [ ] Release notes written
- [ ] Deployment validated

---

### Iteration 13 Deliverables

- [ ] 100% feature completion validated
- [ ] All tests passing
- [ ] Documentation complete
- [ ] Release artifacts ready
- [ ] Version 1.0.0 tagged

**Exit Criteria:**
- All P1 and P2 features complete
- All tests passing
- Documentation complete
- Ready for production release

**Estimated Completion:** End of Week 8

---

## Summary Timeline

| Iteration | Duration | Priority | Key Deliverables | Week |
|-----------|----------|----------|------------------|------|
| **Iteration 9** | 1 week (22h) | P1 | Security fixes, integration tests | Week 1 |
| **Iteration 10** | 2 weeks (32h) | P1 | WinPE bootable USB | Weeks 2-3 |
| **Iteration 11** | 2 weeks (44h) | P2 | P2 features complete | Weeks 4-5 |
| **Iteration 12** | 2 weeks (60+h) | P3 | P3 enhancements (optional) | Weeks 6-7 |
| **Iteration 13** | 1 week (16h) | P1 | Final polish, 100% validation | Week 8 |

**Total Timeline:**
- **P1+P2 Completion:** 5 weeks (98 hours)
- **Full 100% (including P3):** 8 weeks (158+ hours)

---

## Risk Management

### High Risk Items

| Risk | Mitigation | Contingency |
|------|------------|-------------|
| WinPE USB complexity | Phased approach, use libraries | Simplify to basic boot only |
| E01 write format complexity | Extensive research, reference implementation | Defer to future release |
| Integration test scope creep | Prioritize critical paths | Reduce to MVP test set |

### Medium Risk Items

| Risk | Mitigation | Contingency |
|------|------------|-------------|
| Property tests reveal bugs | Allocate buffer time | Fix critical bugs, defer others |
| Performance issues | Benchmark early | Optimize hot paths |

---

## Quality Gates

### After Iteration 9
- [ ] All P1 security issues resolved
- [ ] Integration test suite operational
- [ ] CI/CD running all tests

### After Iteration 10
- [ ] WinPE USB creation working
- [ ] Tested on physical hardware
- [ ] Documentation complete

### After Iteration 11
- [ ] All P2 features complete
- [ ] All tests passing
- [ ] No regressions

### After Iteration 13 (100% Complete)
- [ ] All P1 and P2 features complete
- [ ] All tests passing (unit, integration, property)
- [ ] Documentation complete and accurate
- [ ] Release artifacts ready
- [ ] Production deployment validated

---

## Success Metrics

### Completion Metrics
- **Feature Completeness:** 100% (P1+P2) or 100% (all priorities)
- **Test Coverage:** Maintain >322 tests, add integration tests
- **Security:** 100% P0 and P1 issues resolved
- **Documentation:** Complete and up-to-date

### Quality Metrics
- **Test Pass Rate:** 100%
- **Code Coverage:** >80%
- **Security Audit:** Pass external audit
- **Performance:** Meet benchmark targets

---

## Dependencies & Prerequisites

### External Dependencies
- **Windows ADK:** Required for WinPE USB (Iteration 10)
- **WIM format library:** May be needed for WinPE (Iteration 10)
- **Test fixtures:** Required for integration tests (Iteration 9)

### Internal Dependencies
- **Existing crates:** All iterations depend on existing TotalImage crates
- **Test infrastructure:** Integration tests depend on test framework setup

---

## Resource Requirements

### Development Team
- **Security specialist:** Iteration 9 (6 hours)
- **Backend developer:** All iterations (primary)
- **QA engineer:** Iterations 9, 11, 13 (testing focus)
- **DevOps engineer:** Iteration 13 (release preparation)

### Infrastructure
- **CI/CD:** Already set up
- **Test hardware:** USB drives for WinPE testing (Iteration 10)
- **Windows environment:** For WinPE development (Iteration 10)

---

## Change Management

### Scope Changes
- P3 features are explicitly optional and can be deferred
- WinPE USB can be simplified if complexity is too high
- E01 write support can be deferred if format is too complex

### Timeline Adjustments
- Buffer time included in estimates (20% overhead)
- Critical path: Iterations 9, 10, 11, 13 (P1+P2)
- P3 features can be added incrementally after P1+P2 complete

---

## Communication Plan

### Weekly Updates
- Progress report every Friday
- Blockers reported immediately
- Test results shared after each iteration

### Documentation Updates
- Update EXECUTION-PROGRESS.md after each iteration
- Update this plan as tasks complete
- Update README and CHANGELOG in Iteration 13

---

## Appendix A: Task Breakdown by Crate

### totalimage-core
- Security fixes (Iteration 9)

### totalimage-pipeline
- SEC-004 mmap validation (Iteration 9)

### totalimage-vaults
- E01 write support (Iteration 11)

### totalimage-zones
- GPT backup header validation (Iteration 11)

### totalimage-territories
- ISO Joliet extension (Iteration 11)
- exFAT/ISO extraction (Iteration 11)

### totalimage-acquire
- WinPE USB creation (Iteration 10)
- E01 writer (Iteration 11)

### totalimage-cli
- Hash command (Iteration 11)
- Tree command (Iteration 12)
- WinPE USB command (Iteration 10)

### totalimage-web
- exFAT/ISO extraction (Iteration 11)
- SEC-008 cache overflow fix (Iteration 9)

### totalimage-mcp
- Integration tests (Iteration 9)

### fire-marshal
- Integration tests (Iteration 9)

---

## Appendix B: Test Coverage Targets

| Crate | Current | Target | Gap | Iteration |
|-------|---------|--------|-----|-----------|
| totalimage-core | 80 | 85 | +5 | Iteration 9 |
| totalimage-pipeline | 9 | 12 | +3 | Iteration 9 |
| totalimage-vaults | 103 | 120 | +17 | Iterations 9, 11 |
| totalimage-zones | 34 | 42 | +8 | Iteration 11 |
| totalimage-territories | 0 | 15 | +15 | Iterations 9, 11 |
| totalimage-acquire | 3 | 25 | +22 | Iterations 10, 11 |
| totalimage-cli | 2 | 8 | +6 | Iterations 11, 12 |
| totalimage-web | 8 | 12 | +4 | Iterations 9, 11 |
| totalimage-mcp | 54 | 60 | +6 | Iteration 9 |
| fire-marshal | 2 | 5 | +3 | Iteration 9 |
| **Integration Tests** | 0 | 20+ | +20 | Iteration 9 |
| **Property Tests** | 0 | 10+ | +10 | Iteration 11 |
| **Total** | **295** | **414+** | **+119** | **All** |

---

## Appendix C: Code Quality Checklist

### Before Each Iteration
- [ ] Code formatted (`cargo fmt`)
- [ ] Lints clean (`cargo clippy`)
- [ ] All tests passing
- [ ] Documentation updated

### After Each Iteration
- [ ] All acceptance criteria met
- [ ] Tests added for new features
- [ ] Documentation updated
- [ ] Code review completed
- [ ] CI/CD passing

---

## Document Metadata

- **Created:** December 21, 2025 at 12:34:52 MST
- **Based On:** COMPLETE-GAP-ANALYSIS-REPORT.md
- **Status:** Planning Document
- **Next Review:** After Iteration 9 completion
- **Maintained By:** Development Team

---

**End of Implementation Plan**

