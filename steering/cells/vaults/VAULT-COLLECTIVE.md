# VAULT COLLECTIVE: Container Format Cells

**Codename:** Underground Storage Liberation
**Purpose:** Decrypt and sabotage proprietary disk image container formats
**Current State:** C# Abstract + Implementations
**Target State:** Rust Trait + Concrete Types

---

## Overview

The Vault Collective handles various container formats that encapsulate disk images. Each vault type provides transparent access to the underlying sector data, regardless of the proprietary format used.

---

## Base Vault Architecture

### Cell: `Container` (Abstract Base)
**Brand Name:** `BaseVault`
**Location:** `TotalImage.IO/Containers/Container.cs`
**Purpose:** Foundation for all container format handlers

#### Actions (Methods)

| Actual Name | Action Name | Function |
|-------------|-------------|----------|
| `Content` | `expose_pipeline()` | Provide stream access to content |
| `DisplayName` | `identify()` | Return vault type identifier |
| `PartitionTable` | `discover_zones()` | Detect and load partition structure |
| `SaveImage` | `export_vault()` | Write container to disk |
| `CalculateMd5Hash` | `fingerprint_md5()` | Generate MD5 identity hash |
| `CalculateSha1Hash` | `fingerprint_sha1()` | Generate SHA1 identity hash |
| `LoadPartitionTable` | `reconnaissance_zones()` | Auto-detect partition layout |

#### State Variables

| C# Field | Rust Equivalent | Purpose |
|----------|-----------------|---------|
| `backingFile: MemoryMappedFile?` | `backing: Option<Mmap>` | Memory-mapped file for direct action |
| `containerStream: Stream` | `stream: Box<dyn Read + Seek>` | Pipeline to container data |
| `_partitionTable: PartitionTable?` | `zones: Option<ZoneTable>` | Cached zone structure |

#### Pseudocode (Rust Conversion)

```rust
// VAULT BASE TRAIT
pub trait Vault: Send + Sync {
    // RECONNAISSANCE ACTIONS
    fn expose_pipeline(&self) -> &dyn ReadSeek;
    fn identify(&self) -> &str;
    fn length(&self) -> u64;

    // ZONE DISCOVERY
    fn discover_zones(&mut self) -> Result<&ZoneTable> {
        if self.zones_cache().is_none() {
            let zones = reconnaissance_zones(self.expose_pipeline())?;
            self.set_zones_cache(zones);
        }
        Ok(self.zones_cache().as_ref().unwrap())
    }

    // FINGERPRINTING ACTIONS
    async fn fingerprint_md5(&mut self) -> Result<String> {
        let mut hasher = Md5::new();
        let mut pipeline = self.expose_pipeline();
        pipeline.seek(SeekFrom::Start(0))?;
        io::copy(&mut pipeline, &mut hasher)?;
        Ok(format!("{:x}", hasher.finalize()))
    }

    async fn fingerprint_sha1(&mut self) -> Result<String> {
        let mut hasher = Sha1::new();
        let mut pipeline = self.expose_pipeline();
        pipeline.seek(SeekFrom::Start(0))?;
        io::copy(&mut pipeline, &mut hasher)?;
        Ok(format!("{:x}", hasher.finalize()))
    }

    // EXPORT ACTION
    fn export_vault(&self, path: &Path) -> Result<()> {
        let temp_path = path.with_extension("tmp");
        let mut output = File::create(&temp_path)?;
        let mut pipeline = self.expose_pipeline();
        pipeline.seek(SeekFrom::Start(0))?;
        io::copy(&mut pipeline, &mut output)?;
        output.sync_all()?;
        fs::rename(temp_path, path)?;
        Ok(())
    }

    // CACHE ACCESS
    fn zones_cache(&self) -> &Option<ZoneTable>;
    fn set_zones_cache(&mut self, zones: ZoneTable);
}

// VAULT INITIALIZATION METHODS
pub enum VaultAccess {
    DirectAction(Mmap),     // Memory-mapped for speed
    Pipeline(File),         // Stream-based access
}

pub struct VaultConfig {
    pub use_direct_action: bool,  // Enable memory mapping
}
```

---

## Concrete Vault Cells

### Cell 1: `RawContainer`
**Brand Name:** `RawVault`
**Codename:** "NAKED TRUTH"
**Location:** `TotalImage.IO/Containers/RawContainer.cs`

#### Purpose
Handles unencapsulated raw sector images (.img, .ima, .flp, .vfd, .dsk, .iso)

#### Actions

| Actual Name | Action Name | Pseudocode |
|-------------|-------------|------------|
| `Content` | `expose_pipeline()` | `return containerStream` |
| `DisplayName` | `identify()` | `return "Raw sector image"` |
| `CreateImage` | `manufacture_vault()` | Create new blank image |

#### Rust Conversion Pseudocode

```rust
pub struct RawVault {
    pipeline: Box<dyn ReadSeek>,
    zones: Option<ZoneTable>,
}

impl RawVault {
    pub fn open(path: &Path, config: VaultConfig) -> Result<Self> {
        let pipeline: Box<dyn ReadSeek> = if config.use_direct_action {
            // DIRECT ACTION: Memory-map the file
            let file = File::open(path)?;
            let mmap = unsafe { Mmap::map(&file)? };
            Box::new(Cursor::new(mmap))
        } else {
            // PIPELINE: Regular file stream
            Box::new(File::open(path)?)
        };

        Ok(RawVault {
            pipeline,
            zones: None,
        })
    }

    pub fn manufacture_vault(bpb: BiosParamBlock, tracks: u8, write_bpb: bool)
        -> Result<Self> {
        // PROPAGANDA ACTION: Create new image
        let sector_size = bpb.bytes_per_sector;
        let sectors_per_track = bpb.sectors_per_track;
        let heads = bpb.num_heads;

        let total_size = sector_size * sectors_per_track * heads * (tracks as u32);
        let mut buffer = vec![0u8; total_size as usize];

        if write_bpb {
            // Inject BPB manifesto into boot sector
            bpb.write_to(&mut buffer[0..512])?;
        }

        let pipeline = Box::new(Cursor::new(buffer));
        Ok(RawVault {
            pipeline,
            zones: None,
        })
    }
}

impl Vault for RawVault {
    fn expose_pipeline(&self) -> &dyn ReadSeek {
        &*self.pipeline
    }

    fn identify(&self) -> &str {
        "Raw sector image"
    }

    fn length(&self) -> u64 {
        self.pipeline.stream_len().unwrap_or(0)
    }

    fn zones_cache(&self) -> &Option<ZoneTable> {
        &self.zones
    }

    fn set_zones_cache(&mut self, zones: ZoneTable) {
        self.zones = Some(zones);
    }
}
```

---

### Cell 2: `VhdContainer`
**Brand Name:** `MicrosoftVault`
**Codename:** "CORPORATE SABOTAGE"
**Location:** `TotalImage.IO/Containers/VHD/VhdContainer.cs`

#### Purpose
Decrypt and sabotage Microsoft Virtual Hard Disk (.vhd) format

#### Structures

**VHD Footer (Manifesto)**
- Cookie: "conectix" (8 bytes)
- Features, Version, Data Offset
- Timestamp, Creator App, OS
- Original/Current Size
- Disk Geometry (C/H/S)
- Disk Type (Fixed/Dynamic/Differencing)
- Checksum, UUID
- Saved State

**VHD Dynamic Header**
- Cookie: "cxsparse" (8 bytes)
- Data Offset, Table Offset
- Header Version, Max Table Entries
- Block Size, Checksum, Parent UUID
- Parent Timestamp, Parent Name

**VHD Block Allocation Table (BAT)**
- Array of sector offsets for dynamic blocks

#### Actions

| Actual Name | Action Name | Pseudocode Index |
|-------------|-------------|------------------|
| Constructor | `decrypt_vault()` | VHD-001 |
| `Content` | `expose_pipeline()` | VHD-002 |
| `Footer` | `read_manifesto()` | VHD-003 |
| `DynamicHeader` | `read_dynamic_manifesto()` | VHD-004 |

#### Rust Conversion Pseudocode

```rust
// VHD STRUCTURES
#[repr(C)]
pub struct VhdFooter {
    cookie: [u8; 8],              // "conectix"
    features: u32,
    version: u32,
    data_offset: u64,
    timestamp: u32,
    creator_app: [u8; 4],
    creator_version: u32,
    creator_os: u32,
    original_size: u64,
    current_size: u64,
    geometry: DiskGeometry,
    disk_type: VhdType,
    checksum: u32,
    uuid: [u8; 16],
    saved_state: u8,
    reserved: [u8; 427],
}

#[repr(C)]
pub struct VhdDynamicHeader {
    cookie: [u8; 8],              // "cxsparse"
    data_offset: u64,
    table_offset: u64,
    header_version: u32,
    max_table_entries: u32,
    block_size: u32,
    checksum: u32,
    parent_uuid: [u8; 16],
    parent_timestamp: u32,
    reserved: u32,
    parent_unicode_name: [u16; 256],
    // ... parent locators
}

#[derive(Debug, Clone, Copy)]
pub enum VhdType {
    None = 0,
    Reserved1 = 1,
    FixedHardDisk = 2,
    DynamicHardDisk = 3,
    DifferencingHardDisk = 4,
    Reserved5 = 5,
    Reserved6 = 6,
}

pub struct BlockAllocationTable {
    entries: Vec<u32>,            // Sector offsets
    block_size: u32,
}

// MICROSOFT VAULT IMPLEMENTATION
pub struct MicrosoftVault {
    base_pipeline: Box<dyn ReadSeek>,
    content_pipeline: Box<dyn ReadSeek>,
    manifesto: VhdFooter,
    dynamic_manifesto: Option<VhdDynamicHeader>,
    bat: Option<BlockAllocationTable>,
    zones: Option<ZoneTable>,
}

impl MicrosoftVault {
    // ACTION: VHD-001 - Decrypt Vault
    pub fn decrypt_vault(path: &Path, config: VaultConfig) -> Result<Self> {
        let mut base_pipeline: Box<dyn ReadSeek> = if config.use_direct_action {
            let file = File::open(path)?;
            let mmap = unsafe { Mmap::map(&file)? };
            Box::new(Cursor::new(mmap))
        } else {
            Box::new(File::open(path)?)
        };

        // READ MANIFESTO from footer (last 512 bytes)
        base_pipeline.seek(SeekFrom::End(-512))?;
        let mut footer_bytes = [0u8; 512];
        base_pipeline.read_exact(&mut footer_bytes)?;
        let manifesto = VhdFooter::parse(&footer_bytes)?;

        // VERIFY MANIFESTO INTEGRITY
        if !manifesto.verify_checksum() {
            return Err("VHD manifesto corrupted: checksum failed".into());
        }

        // HANDLE DYNAMIC/DIFFERENCING VAULTS
        let (content_pipeline, dynamic_manifesto, bat) =
            if manifesto.disk_type == VhdType::DynamicHardDisk ||
               manifesto.disk_type == VhdType::DifferencingHardDisk {

            // Read dynamic manifesto
            base_pipeline.seek(SeekFrom::Start(manifesto.data_offset))?;
            let mut dynamic_header = [0u8; 1024];
            base_pipeline.read_exact(&mut dynamic_header)?;
            let dyn_hdr = VhdDynamicHeader::parse(&dynamic_header)?;

            if !dyn_hdr.verify_checksum() {
                return Err("VHD dynamic manifesto corrupted".into());
            }

            // Read Block Allocation Table
            base_pipeline.seek(SeekFrom::Start(dyn_hdr.table_offset))?;
            let bat_size = dyn_hdr.max_table_entries * 4;
            let mut bat_bytes = vec![0u8; bat_size as usize];
            base_pipeline.read_exact(&mut bat_bytes)?;
            let bat = BlockAllocationTable::parse(&bat_bytes, dyn_hdr.block_size)?;

            // Create dynamic content pipeline
            let content = Box::new(
                VhdDynamicPipeline::new(base_pipeline, &bat, dyn_hdr.block_size)?
            );

            (content, Some(dyn_hdr), Some(bat))
        } else {
            // FIXED VAULT: Content is everything except last 512 bytes
            let total_len = base_pipeline.stream_len()?;
            let content = Box::new(
                PartialPipeline::new(base_pipeline, 0, total_len - 512)?
            );
            (content, None, None)
        };

        Ok(MicrosoftVault {
            base_pipeline,
            content_pipeline,
            manifesto,
            dynamic_manifesto,
            bat,
            zones: None,
        })
    }

    // ACTION: VHD-003 - Read Manifesto
    pub fn read_manifesto(&self) -> &VhdFooter {
        &self.manifesto
    }

    // ACTION: VHD-004 - Read Dynamic Manifesto
    pub fn read_dynamic_manifesto(&self) -> Option<&VhdDynamicHeader> {
        self.dynamic_manifesto.as_ref()
    }
}

impl Vault for MicrosoftVault {
    // ACTION: VHD-002 - Expose Pipeline
    fn expose_pipeline(&self) -> &dyn ReadSeek {
        &*self.content_pipeline
    }

    fn identify(&self) -> &str {
        "Microsoft VHD"
    }

    fn length(&self) -> u64 {
        self.manifesto.current_size
    }

    fn zones_cache(&self) -> &Option<ZoneTable> {
        &self.zones
    }

    fn set_zones_cache(&mut self, zones: ZoneTable) {
        self.zones = Some(zones);
    }
}

// DYNAMIC VHD PIPELINE
// Translates linear sector reads to block-based reads via BAT
struct VhdDynamicPipeline {
    base: Box<dyn ReadSeek>,
    bat: BlockAllocationTable,
    block_size: u32,
    position: u64,
    virtual_size: u64,
}

impl Read for VhdDynamicPipeline {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // Calculate which block we're in
        let block_idx = self.position / (self.block_size as u64);
        let block_offset = self.position % (self.block_size as u64);

        // Look up physical sector from BAT
        let sector_offset = self.bat.entries[block_idx as usize];

        if sector_offset == 0xFFFFFFFF {
            // Unallocated block - return zeros
            let to_read = buf.len().min((self.block_size as u64 - block_offset) as usize);
            buf[..to_read].fill(0);
            self.position += to_read as u64;
            Ok(to_read)
        } else {
            // Read from physical location
            let physical_pos = (sector_offset as u64 * 512) + block_offset;
            self.base.seek(SeekFrom::Start(physical_pos))?;
            let read = self.base.read(buf)?;
            self.position += read as u64;
            Ok(read)
        }
    }
}

impl Seek for VhdDynamicPipeline {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let new_pos = match pos {
            SeekFrom::Start(n) => n,
            SeekFrom::End(n) => (self.virtual_size as i64 + n) as u64,
            SeekFrom::Current(n) => (self.position as i64 + n) as u64,
        };
        self.position = new_pos;
        Ok(new_pos)
    }
}
```

---

## Additional Vault Cells

### Cell 3: `NhdContainer` - "LEGACY LIBERATION"
- **Format:** T98-Next HD format (.nhd)
- **Structure:** 256-byte header + raw sectors
- **Special:** Japanese PC-98 emulator format

### Cell 4: `ImzContainer` - "COMPRESSED SABOTAGE"
- **Format:** WinImage compressed (.imz)
- **Structure:** Proprietary compression wrapper
- **Dependency:** Requires decompression pipeline

### Cell 5: `Anex86Container` - "ANEX SABOTAGE"
- **Format:** Anex86 disk images (.fdi, .hdi)
- **Structure:** Header + sector mapping table
- **Special:** PC-98 emulator format with exotic geometries

### Cell 6: `PCjsContainer` - "JSON VAULT"
- **Format:** PCjs.org JSON format
- **Structure:** JSON metadata + base64 sector data
- **Special:** Web-based emulator format

---

## Underground Network (Factory Pattern)

### Vault Detection System

```rust
pub trait VaultFactory {
    fn can_decrypt(&self, path: &Path) -> bool;
    fn decrypt(&self, path: &Path, config: VaultConfig) -> Result<Box<dyn Vault>>;
}

pub struct VaultNetwork {
    factories: Vec<Box<dyn VaultFactory>>,
}

impl VaultNetwork {
    pub fn new() -> Self {
        Self {
            factories: vec![
                Box::new(MicrosoftVaultFactory),
                Box::new(NhdVaultFactory),
                Box::new(ImzVaultFactory),
                Box::new(Anex86VaultFactory),
                Box::new(PCjsVaultFactory),
                Box::new(RawVaultFactory),  // Always last (fallback)
            ],
        }
    }

    pub fn reconnaissance_and_decrypt(&self, path: &Path, config: VaultConfig)
        -> Result<Box<dyn Vault>> {
        for factory in &self.factories {
            if factory.can_decrypt(path) {
                return factory.decrypt(path, config);
            }
        }
        Err("No vault factory could decrypt this format".into())
    }
}

// Example factory implementation
pub struct MicrosoftVaultFactory;

impl VaultFactory for MicrosoftVaultFactory {
    fn can_decrypt(&self, path: &Path) -> bool {
        // Check file extension
        if let Some(ext) = path.extension() {
            if ext.to_str().unwrap_or("").to_lowercase() == "vhd" {
                // Verify cookie signature
                if let Ok(mut file) = File::open(path) {
                    file.seek(SeekFrom::End(-512)).ok()?;
                    let mut cookie = [0u8; 8];
                    file.read_exact(&mut cookie).ok()?;
                    return &cookie == b"conectix";
                }
            }
        }
        false
    }

    fn decrypt(&self, path: &Path, config: VaultConfig) -> Result<Box<dyn Vault>> {
        Ok(Box::new(MicrosoftVault::decrypt_vault(path, config)?))
    }
}
```

---

## Solidarity Dependencies

### Required Rust Crates

```toml
[dependencies]
# PIPELINE OPERATIONS
bytes = "1.5"
tokio = { version = "1.35", features = ["io-util", "fs"] }

# DIRECT ACTION (Memory Mapping)
memmap2 = "0.9"

# FINGERPRINTING
md5 = "0.7"
sha1 = "0.10"

# SABOTAGE OPERATIONS
uuid = { version = "1.6", features = ["v4"] }
chrono = "0.4"

# ERROR HANDLING
thiserror = "1.0"
anyhow = "1.0"
```

---

## Status

- ✅ Base Vault trait designed
- ✅ RawVault pseudocode complete
- ✅ MicrosoftVault pseudocode complete
- ⏳ NhdVault, ImzVault, Anex86Vault, PCjsVault pending
- ⏳ Vault Factory pattern pending
- ⏳ Integration with Zone (Partition) system pending

**Next Action:** Document Territory Collective (FileSystem layer)
