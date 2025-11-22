# TERRITORY COLLECTIVE: FileSystem Cells

**Codename:** Data Autonomy Liberation
**Purpose:** Decrypt and navigate proprietary file system structures
**Current State:** C# Abstract + Implementations
**Target State:** Rust Trait + Concrete Types

---

## Overview

The Territory Collective handles various file system formats. Each territory provides autonomous access to files and directories, abstracting the underlying storage structure.

---

## Base Territory Architecture

### Cell: `FileSystem` (Abstract Base)
**Brand Name:** `Territory`
**Location:** `TotalImage.IO/FileSystems/FileSystem.cs`
**Purpose:** Foundation for all file system handlers

#### Properties (Characteristics)

| Actual Name | Territory Name | Purpose |
|-------------|----------------|---------|
| `DisplayName` | `identify()` | Return territory type |
| `VolumeLabel` | `banner()` | Get/set manifesto name |
| `RootDirectory` | `headquarters()` | Access root directory cell |
| `TotalFreeSpace` | `liberated_space()` | Available storage |
| `TotalSize` | `domain_size()` | Total territory size |
| `AllocationUnitSize` | `block_size()` | Minimum allocation unit |
| `SupportsSubdirectories` | `hierarchical()` | Can organize into branches |

#### Underground Network (Factory Pattern)

```csharp
// C# IMPLEMENTATION
private static readonly ImmutableArray<IFileSystemFactory> _knownFactories =
[
    new FatFactory(),
    new IsoFactory(),
    new ExFatFactory()
];

public static FileSystem AttemptDetection(Stream stream)
{
    foreach (var factory in _knownFactories)
    {
        var result = factory.TryLoadFileSystem(stream);
        if (result != null) return result;
    }
    return new RawFileSystem(stream); // Fallback
}
```

#### Rust Conversion Pseudocode

```rust
// TERRITORY BASE TRAIT
pub trait Territory: Send + Sync {
    // IDENTIFICATION
    fn identify(&self) -> &str;
    fn banner(&self) -> Result<String>;
    fn set_banner(&mut self, label: &str) -> Result<()>;

    // STRUCTURE ACCESS
    fn headquarters(&self) -> &dyn DirectoryCell;
    fn domain_size(&self) -> u64;
    fn liberated_space(&self) -> u64;
    fn block_size(&self) -> u64;
    fn hierarchical(&self) -> bool;

    // PIPELINE ACCESS
    fn pipeline(&self) -> &dyn ReadSeek;
}

// TERRITORY FACTORY TRAIT
pub trait TerritoryFactory {
    fn can_liberate(&self, pipeline: &mut dyn ReadSeek) -> bool;
    fn liberate(&self, pipeline: Box<dyn ReadSeek>) -> Result<Box<dyn Territory>>;
}

// UNDERGROUND NETWORK
pub struct TerritoryNetwork {
    factories: Vec<Box<dyn TerritoryFactory>>,
}

impl TerritoryNetwork {
    pub fn new() -> Self {
        Self {
            factories: vec![
                Box::new(FatTerritoryFactory),
                Box::new(IsoTerritoryFactory),
                Box::new(ExFatTerritoryFactory),
                Box::new(RawTerritoryFactory), // Fallback
            ],
        }
    }

    pub fn reconnaissance(&self, mut pipeline: Box<dyn ReadSeek>)
        -> Result<Box<dyn Territory>> {
        for factory in &self.factories {
            pipeline.seek(SeekFrom::Start(0))?;
            if factory.can_liberate(&mut *pipeline) {
                pipeline.seek(SeekFrom::Start(0))?;
                return factory.liberate(pipeline);
            }
        }
        Err("No factory could liberate this territory".into())
    }
}
```

---

## Concrete Territory Cells

### Territory 1: FAT Family - "LEGACY LIBERATION"
**Brand Names:** `Fat12Territory`, `Fat16Territory`, `Fat32Territory`
**Location:** `TotalImage.IO/FileSystems/FAT/`
**Supported Variants:** FAT12, FAT16, FAT32

#### Core Structures

**BPB (BIOS Parameter Block) - "TERRITORY MANIFESTO"**
```
Offset  Size  Field
------  ----  -----
0x00    3     Jump instruction
0x03    8     OEM identifier
0x0B    2     Bytes per logical sector
0x0D    1     Logical sectors per cluster
0x0E    2     Reserved logical sectors
0x10    1     Number of FATs
0x11    2     Root directory entries (FAT12/16)
0x13    2     Total logical sectors (small)
0x15    1     Media descriptor
0x16    2     Logical sectors per FAT (FAT12/16)
0x18    2     Physical sectors per track
0x1A    2     Number of heads
0x1C    4     Hidden sectors
0x20    4     Total logical sectors (large)

--- FAT12/16 Extended BPB ---
0x24    1     Physical drive number
0x26    1     Extended boot signature
0x27    4     Volume serial number
0x2B    11    Volume label
0x36    8     File system type

--- FAT32 Extended BPB ---
0x24    4     Sectors per FAT
0x28    2     Flags
0x2A    2     Version
0x2C    4     Root cluster
0x30    2     FSInfo sector
0x32    2     Backup boot sector
0x42    1     Physical drive number
0x44    1     Extended boot signature
0x45    4     Volume serial number
0x49    11    Volume label
0x54    8     File system type
```

**FAT (File Allocation Table) - "CLUSTER MAP"**
- Array of cluster chain entries
- FAT12: 12-bit entries
- FAT16: 16-bit entries
- FAT32: 28-bit entries (4 bits reserved)

Special values:
- `0x000`: Free cluster
- `0xFF0-0xFF6` (FAT12): Reserved
- `0xFF7`: Bad cluster
- `0xFF8-0xFFF`: End of chain
- `0x002+`: Next cluster in chain

**Directory Entry - "OCCUPANT RECORD"**
```
Offset  Size  Field
------  ----  -----
0x00    11    8.3 filename (or LFN sequence)
0x0B    1     Attributes (Archive, Dir, VolumeID, etc.)
0x0C    1     Reserved (NT case info)
0x0D    1     Creation time fine (10ms units)
0x0E    2     Creation time
0x10    2     Creation date
0x12    2     Last access date
0x14    2     High word of first cluster (FAT32)
0x16    2     Last modification time
0x18    2     Last modification date
0x1A    2     Low word of first cluster
0x1C    4     File size in bytes
```

**LFN (Long File Name) Entry - "EXTENDED IDENTITY"**
- Sequence number + 13 Unicode characters
- Multiple entries precede 8.3 entry
- Checksum validates association

#### Actions (Operations)

| C# Method | Action Name | Pseudocode Index |
|-----------|-------------|------------------|
| `ReadClusterChain` | `trace_allocation()` | FAT-001 |
| `GetFreeClusterCount` | `count_liberated()` | FAT-002 |
| `EnumerateDirectory` | `census()` | FAT-003 |
| `ReadFile` | `extract_data()` | FAT-004 |
| `Create` | `establish_territory()` | FAT-005 |

#### Rust Pseudocode

```rust
// BPB MANIFESTO STRUCTURE
#[repr(C, packed)]
pub struct BiosParameterBlock {
    pub jump: [u8; 3],
    pub oem: [u8; 8],
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sectors: u16,
    pub num_fats: u8,
    pub root_entries: u16,
    pub total_sectors_16: u16,
    pub media_descriptor: u8,
    pub sectors_per_fat_16: u16,
    pub sectors_per_track: u16,
    pub num_heads: u16,
    pub hidden_sectors: u32,
    pub total_sectors_32: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FatType {
    Fat12,
    Fat16,
    Fat32,
}

impl BiosParameterBlock {
    pub fn detect_fat_type(&self) -> FatType {
        let total_sectors = if self.total_sectors_16 != 0 {
            self.total_sectors_16 as u32
        } else {
            self.total_sectors_32
        };

        let root_dir_sectors = ((self.root_entries * 32) + (self.bytes_per_sector - 1))
            / self.bytes_per_sector;

        let fat_size = if self.sectors_per_fat_16 != 0 {
            self.sectors_per_fat_16 as u32
        } else {
            // FAT32 - read from extended BPB
            0 // TODO: Parse FAT32 BPB
        };

        let data_sectors = total_sectors
            - (self.reserved_sectors as u32)
            - (self.num_fats as u32 * fat_size)
            - root_dir_sectors as u32;

        let cluster_count = data_sectors / self.sectors_per_cluster as u32;

        if cluster_count < 4085 {
            FatType::Fat12
        } else if cluster_count < 65525 {
            FatType::Fat16
        } else {
            FatType::Fat32
        }
    }
}

// FAT TERRITORY IMPLEMENTATION
pub struct FatTerritory {
    pipeline: Box<dyn ReadSeek>,
    manifesto: BiosParameterBlock,
    fat_type: FatType,
    cluster_map: Vec<u32>,
    root_dir: FatDirectoryCell,
}

impl FatTerritory {
    pub fn liberate(mut pipeline: Box<dyn ReadSeek>) -> Result<Self> {
        // READ MANIFESTO
        pipeline.seek(SeekFrom::Start(0))?;
        let mut bpb_bytes = [0u8; 512];
        pipeline.read_exact(&mut bpb_bytes)?;
        let manifesto = BiosParameterBlock::parse(&bpb_bytes)?;

        let fat_type = manifesto.detect_fat_type();

        // READ CLUSTER MAP (File Allocation Table)
        let fat_offset = manifesto.reserved_sectors as u64
            * manifesto.bytes_per_sector as u64;
        let fat_size = if manifesto.sectors_per_fat_16 != 0 {
            manifesto.sectors_per_fat_16 as u64 * manifesto.bytes_per_sector as u64
        } else {
            // FAT32: read from extended BPB
            0 // TODO
        };

        pipeline.seek(SeekFrom::Start(fat_offset))?;
        let mut fat_bytes = vec![0u8; fat_size as usize];
        pipeline.read_exact(&mut fat_bytes)?;

        let cluster_map = match fat_type {
            FatType::Fat12 => Self::parse_fat12(&fat_bytes),
            FatType::Fat16 => Self::parse_fat16(&fat_bytes),
            FatType::Fat32 => Self::parse_fat32(&fat_bytes),
        }?;

        // LOCATE HEADQUARTERS (Root Directory)
        let root_dir_offset = fat_offset + (manifesto.num_fats as u64 * fat_size);
        let root_dir = FatDirectoryCell::load_root(
            &mut *pipeline,
            root_dir_offset,
            manifesto.root_entries,
            fat_type,
        )?;

        Ok(FatTerritory {
            pipeline,
            manifesto,
            fat_type,
            cluster_map,
            root_dir,
        })
    }

    // ACTION: FAT-001 - Trace Allocation
    pub fn trace_allocation(&self, start_cluster: u32) -> Vec<u32> {
        let mut chain = Vec::new();
        let mut cluster = start_cluster;

        loop {
            if cluster < 2 || cluster >= self.cluster_map.len() as u32 {
                break;
            }

            chain.push(cluster);
            let next = self.cluster_map[cluster as usize];

            // Check for end-of-chain marker
            let eoc = match self.fat_type {
                FatType::Fat12 => next >= 0xFF8,
                FatType::Fat16 => next >= 0xFFF8,
                FatType::Fat32 => next >= 0x0FFFFFF8,
            };

            if eoc {
                break;
            }

            cluster = next;
        }

        chain
    }

    // ACTION: FAT-002 - Count Liberated Clusters
    pub fn count_liberated(&self) -> u32 {
        self.cluster_map.iter()
            .skip(2) // First 2 entries are reserved
            .filter(|&&entry| entry == 0)
            .count() as u32
    }

    // HELPER: Parse FAT12 cluster map
    fn parse_fat12(bytes: &[u8]) -> Result<Vec<u32>> {
        let entry_count = (bytes.len() * 2) / 3;
        let mut entries = vec![0u32; entry_count];

        for i in 0..entry_count {
            let byte_offset = (i * 3) / 2;
            let value = if i % 2 == 0 {
                // Even entry: low 12 bits of 2 bytes
                (bytes[byte_offset] as u32) | ((bytes[byte_offset + 1] as u32 & 0x0F) << 8)
            } else {
                // Odd entry: high 12 bits of 2 bytes
                ((bytes[byte_offset] as u32 & 0xF0) >> 4) | ((bytes[byte_offset + 1] as u32) << 4)
            };
            entries[i] = value;
        }

        Ok(entries)
    }

    // HELPER: Parse FAT16 cluster map
    fn parse_fat16(bytes: &[u8]) -> Result<Vec<u32>> {
        let mut entries = Vec::with_capacity(bytes.len() / 2);
        for chunk in bytes.chunks_exact(2) {
            let value = u16::from_le_bytes([chunk[0], chunk[1]]) as u32;
            entries.push(value);
        }
        Ok(entries)
    }

    // HELPER: Parse FAT32 cluster map
    fn parse_fat32(bytes: &[u8]) -> Result<Vec<u32>> {
        let mut entries = Vec::with_capacity(bytes.len() / 4);
        for chunk in bytes.chunks_exact(4) {
            let value = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            entries.push(value & 0x0FFFFFFF); // Mask off top 4 bits
        }
        Ok(entries)
    }

    // ACTION: FAT-004 - Extract Data
    pub fn extract_data(&mut self, start_cluster: u32, size: u64) -> Result<Vec<u8>> {
        let chain = self.trace_allocation(start_cluster);
        let cluster_size = self.manifesto.sectors_per_cluster as u64
            * self.manifesto.bytes_per_sector as u64;

        let mut data = Vec::with_capacity(size as usize);

        for cluster in chain {
            let sector = self.cluster_to_sector(cluster);
            let offset = sector * self.manifesto.bytes_per_sector as u64;

            self.pipeline.seek(SeekFrom::Start(offset))?;
            let mut cluster_data = vec![0u8; cluster_size as usize];
            self.pipeline.read_exact(&mut cluster_data)?;

            let to_copy = (size - data.len() as u64).min(cluster_size) as usize;
            data.extend_from_slice(&cluster_data[..to_copy]);

            if data.len() >= size as usize {
                break;
            }
        }

        Ok(data)
    }

    fn cluster_to_sector(&self, cluster: u32) -> u64 {
        let root_dir_sectors = ((self.manifesto.root_entries as u32 * 32)
            + (self.manifesto.bytes_per_sector as u32 - 1))
            / self.manifesto.bytes_per_sector as u32;

        let first_data_sector = self.manifesto.reserved_sectors as u32
            + (self.manifesto.num_fats as u32 * self.manifesto.sectors_per_fat_16 as u32)
            + root_dir_sectors;

        (first_data_sector + ((cluster - 2) * self.manifesto.sectors_per_cluster as u32)) as u64
    }
}

impl Territory for FatTerritory {
    fn identify(&self) -> &str {
        match self.fat_type {
            FatType::Fat12 => "FAT12",
            FatType::Fat16 => "FAT16",
            FatType::Fat32 => "FAT32",
        }
    }

    fn banner(&self) -> Result<String> {
        // Try root directory volume label first
        // Fall back to BPB volume label
        Ok(String::new()) // TODO: Implement
    }

    fn set_banner(&mut self, _label: &str) -> Result<()> {
        Err("Not implemented".into())
    }

    fn headquarters(&self) -> &dyn DirectoryCell {
        &self.root_dir
    }

    fn domain_size(&self) -> u64 {
        let total_sectors = if self.manifesto.total_sectors_16 != 0 {
            self.manifesto.total_sectors_16 as u64
        } else {
            self.manifesto.total_sectors_32 as u64
        };
        total_sectors * self.manifesto.bytes_per_sector as u64
    }

    fn liberated_space(&self) -> u64 {
        let free_clusters = self.count_liberated();
        let cluster_size = self.manifesto.sectors_per_cluster as u64
            * self.manifesto.bytes_per_sector as u64;
        free_clusters as u64 * cluster_size
    }

    fn block_size(&self) -> u64 {
        self.manifesto.sectors_per_cluster as u64
            * self.manifesto.bytes_per_sector as u64
    }

    fn hierarchical(&self) -> bool {
        true
    }

    fn pipeline(&self) -> &dyn ReadSeek {
        &*self.pipeline
    }
}
```

---

### Territory 2: ISO-9660 - "OPTICAL LIBERATION"
**Brand Name:** `IsoTerritory`
**Location:** `TotalImage.IO/FileSystems/ISO/`
**Variants:** ISO-9660, High Sierra, Joliet

#### Core Structures

**Volume Descriptor - "OPTICAL MANIFESTO"**
```
Offset  Size  Field
------  ----  -----
0x00    1     Type (1=Primary, 2=Supplementary, 255=Terminator)
0x01    5     Standard Identifier ("CD001" or "CDROM")
0x06    1     Version (1)
0x07    ...   Type-specific data
```

**Primary Volume Descriptor**
- System/Volume Identifiers
- Volume Space Size (blocks)
- Logical Block Size (usually 2048)
- Path Table Size/Location
- Root Directory Record
- Volume Set, Publisher, Data Preparer IDs
- Creation/Modification Timestamps

**Directory Record - "OCCUPANT MANIFEST"**
```
Offset  Size  Field
------  ----  -----
0x00    1     Record length
0x01    1     Extended attribute record length
0x02    8     Location of extent (LBA)
0x0A    8     Data length
0x12    7     Recording date/time
0x19    1     File flags (Hidden, Dir, Associated, etc.)
0x1A    1     File unit size
0x1B    1     Interleave gap size
0x1C    4     Volume sequence number
0x20    1     Length of file identifier
0x21    ...   File identifier (variable)
       ...    Padding field (if necessary)
       ...    System use area
```

#### Rust Pseudocode

```rust
// VOLUME DESCRIPTOR TYPES
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum VolumeDescriptorType {
    BootRecord = 0,
    PrimaryVolumeDescriptor = 1,
    SupplementaryVolumeDescriptor = 2,
    VolumePartitionDescriptor = 3,
    VolumeDescriptorSetTerminator = 255,
}

// PRIMARY VOLUME DESCRIPTOR
#[repr(C, packed)]
pub struct PrimaryVolumeDescriptor {
    pub type_code: u8,
    pub identifier: [u8; 5],      // "CD001" or "CDROM" (High Sierra)
    pub version: u8,
    // ... many fields ...
    pub volume_space_size: BothEndian<u32>,
    pub volume_set_size: BothEndian<u16>,
    pub volume_sequence_number: BothEndian<u16>,
    pub logical_block_size: BothEndian<u16>,
    pub path_table_size: BothEndian<u32>,
    pub l_path_table: u32,
    pub m_path_table: u32,
    pub root_directory_record: [u8; 34],
    pub volume_identifier: [u8; 32],
    // ... more fields ...
}

// BOTH-ENDIAN INTEGER (ISO stores as both LE and BE)
#[repr(C, packed)]
pub struct BothEndian<T> {
    pub little: T,
    pub big: T,
}

// ISO TERRITORY
pub struct IsoTerritory {
    pipeline: Box<dyn ReadSeek>,
    volume_descriptors: Vec<VolumeDescriptor>,
    primary_descriptor: PrimaryVolumeDescriptor,
    root_dir: IsoDirectoryCell,
    is_high_sierra: bool,
    is_joliet: bool,
}

impl IsoTerritory {
    pub fn liberate(mut pipeline: Box<dyn ReadSeek>) -> Result<Self> {
        // Volume descriptors start at sector 16 (0x8000 bytes)
        pipeline.seek(SeekFrom::Start(0x8000))?;

        let mut volume_descriptors = Vec::new();
        let mut primary_descriptor = None;

        loop {
            let mut desc_bytes = [0u8; 2048];
            pipeline.read_exact(&mut desc_bytes)?;

            let vd_type = desc_bytes[0];
            let identifier = &desc_bytes[1..6];

            // Check for High Sierra vs ISO 9660
            let is_high_sierra = identifier == b"CDROM";
            let is_iso9660 = identifier == b"CD001";

            if !is_high_sierra && !is_iso9660 {
                return Err("Invalid volume descriptor".into());
            }

            if vd_type == 255 {
                // Terminator
                break;
            }

            if vd_type == 1 {
                // Primary Volume Descriptor
                primary_descriptor = Some(
                    PrimaryVolumeDescriptor::parse(&desc_bytes)?
                );
            }

            volume_descriptors.push(VolumeDescriptor::parse(&desc_bytes)?);
        }

        let primary = primary_descriptor.ok_or("No primary volume descriptor")?;

        // Check for Joliet (supplementary descriptor with escape sequences)
        let is_joliet = volume_descriptors.iter().any(|vd| {
            matches!(vd, VolumeDescriptor::Supplementary(s) if s.is_joliet())
        });

        // Parse root directory
        let root_dir = IsoDirectoryCell::parse(&primary.root_directory_record, &mut *pipeline)?;

        Ok(IsoTerritory {
            pipeline,
            volume_descriptors,
            primary_descriptor: primary,
            root_dir,
            is_high_sierra,
            is_joliet,
        })
    }
}

impl Territory for IsoTerritory {
    fn identify(&self) -> &str {
        if self.is_high_sierra {
            "High Sierra"
        } else if self.is_joliet {
            "ISO 9660 + Joliet"
        } else {
            "ISO 9660"
        }
    }

    fn banner(&self) -> Result<String> {
        Ok(String::from_utf8_lossy(&self.primary_descriptor.volume_identifier)
            .trim()
            .to_string())
    }

    fn set_banner(&mut self, _label: &str) -> Result<()> {
        Err("ISO is read-only".into())
    }

    fn headquarters(&self) -> &dyn DirectoryCell {
        &self.root_dir
    }

    fn domain_size(&self) -> u64 {
        let block_count = self.primary_descriptor.volume_space_size.little;
        let block_size = self.primary_descriptor.logical_block_size.little;
        block_count as u64 * block_size as u64
    }

    fn liberated_space(&self) -> u64 {
        0 // ISO is read-only, no free space concept
    }

    fn block_size(&self) -> u64 {
        self.primary_descriptor.logical_block_size.little as u64
    }

    fn hierarchical(&self) -> bool {
        true
    }

    fn pipeline(&self) -> &dyn ReadSeek {
        &*self.pipeline
    }
}
```

---

### Territory 3: exFAT - "EXTENDED LIBERATION"
**Brand Name:** `ExFatTerritory`
**Location:** `TotalImage.IO/FileSystems/ExFAT/`
**Purpose:** Liberate Microsoft's extended FAT file system

#### Key Differences from FAT
- No FAT12/16/32 cluster limits
- Supports files > 4GB
- UTC timestamps
- Allocation bitmap instead of FAT
- TexFAT extensions (optional)

---

### Territory 4: RAW - "UNORGANIZED TERRITORY"
**Brand Name:** `RawTerritory`
**Location:** `TotalImage.IO/FileSystems/RAW/`
**Purpose:** Fallback for unrecognized file systems

Presents raw sector data as a single file.

---

## Directory and File Cells

### Directory Cell Interface

```rust
pub trait DirectoryCell {
    fn name(&self) -> &str;
    fn list_occupants(&self) -> Result<Vec<OccupantInfo>>;
    fn enter(&self, name: &str) -> Result<Box<dyn DirectoryCell>>;
    fn extract_file(&self, name: &str) -> Result<Vec<u8>>;
}

pub struct OccupantInfo {
    pub name: String,
    pub is_directory: bool,
    pub size: u64,
    pub created: Option<SystemTime>,
    pub modified: Option<SystemTime>,
    pub attributes: u32,
}
```

---

## Solidarity Dependencies

```toml
[dependencies]
# TIMESTAMP OPERATIONS
chrono = "0.4"

# ENCODING (for FAT/ISO character sets)
encoding_rs = "0.8"

# UNICODE (for Joliet/LFN)
unicode-normalization = "0.1"

# ERROR HANDLING
thiserror = "1.0"
anyhow = "1.0"
```

---

## Status

- ✅ Territory trait designed
- ✅ FAT Territory pseudocode complete
- ✅ ISO Territory pseudocode complete
- ⏳ exFAT Territory pending
- ⏳ Directory/File cell traits pending
- ⏳ Integration with Vault layer pending

**Next Action:** Document Zone Collective (Partition layer)
