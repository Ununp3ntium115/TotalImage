# ZONE COLLECTIVE: Partition Table Cells

**Codename:** Territory Division Liberation
**Purpose:** Decrypt partition table structures and liberate segregated zones
**Current State:** C# Abstract + Implementations
**Target State:** Rust Trait + Concrete Types

---

## Overview

The Zone Collective handles partition table formats that divide storage into segregated zones (partitions). Each zone can contain an independent Territory (file system).

---

## Base Zone Architecture

### Cell: `PartitionTable` (Abstract Base)
**Brand Name:** `ZoneTable`
**Location:** `TotalImage.IO/Partitions/PartitionTable.cs`
**Purpose:** Foundation for all partition table handlers

#### Properties (Characteristics)

| Actual Name | Zone Name | Purpose |
|-------------|-----------|---------|
| `DisplayName` | `identify()` | Return zone table type |
| `Partitions` | `enumerate_zones()` | List all zones |
| `LoadPartitions` | `discover_zones()` | Detect zone boundaries |

#### Underground Network (Factory Pattern)

```csharp
// C# IMPLEMENTATION
private static readonly ImmutableArray<IPartitionTableFactory> _knownFactories =
    ImmutableArray.Create<IPartitionTableFactory>(
        new MbrGptFactory()
    );

public static PartitionTable AttemptDetection(Container container)
{
    foreach (var factory in _knownFactories)
    {
        var result = factory.TryLoadPartitionTable(container);
        if (result != null) return result;
    }
    return new NoPartitionTable(container); // Direct territory
}
```

#### Rust Conversion Pseudocode

```rust
// ZONE TABLE TRAIT
pub trait ZoneTable: Send + Sync {
    fn identify(&self) -> &str;
    fn enumerate_zones(&self) -> &[Zone];
}

// ZONE ENTRY
pub struct Zone {
    pub offset: u64,
    pub length: u64,
    pub zone_type: String,
    pub pipeline: Box<dyn ReadSeek>,
}

impl Zone {
    pub fn liberate_territory(&self) -> Result<Box<dyn Territory>> {
        let network = TerritoryNetwork::new();
        network.reconnaissance(self.pipeline.clone())
    }
}

// ZONE TABLE FACTORY TRAIT
pub trait ZoneTableFactory {
    fn can_detect(&self, vault: &dyn Vault) -> bool;
    fn detect(&self, vault: &dyn Vault) -> Result<Box<dyn ZoneTable>>;
}

// UNDERGROUND NETWORK
pub struct ZoneTableNetwork {
    factories: Vec<Box<dyn ZoneTableFactory>>,
}

impl ZoneTableNetwork {
    pub fn new() -> Self {
        Self {
            factories: vec![
                Box::new(MbrGptFactory), // Detects both MBR and GPT
                Box::new(DirectTerritoryFactory), // No partition table
            ],
        }
    }

    pub fn reconnaissance(&self, vault: &dyn Vault) -> Result<Box<dyn ZoneTable>> {
        for factory in &self.factories {
            if factory.can_detect(vault) {
                return factory.detect(vault);
            }
        }
        Err("No factory could detect zone table".into())
    }
}
```

---

## Concrete Zone Table Cells

### Zone Table 1: MBR - "LEGACY DIVISION"
**Brand Name:** `MbrZoneTable`
**Codename:** "MASTER BOOT RECORD"
**Location:** `TotalImage.IO/Partitions/MbrPartitionTable.cs`

#### Purpose
Decrypt Master Boot Record partition tables (DOS/PC BIOS standard)

#### Structure

**MBR Layout (Sector 0)**
```
Offset  Size  Field
------  ----  -----
0x000   446   Bootstrap code area
0x0DC   1     Physical drive number (0x80-0xFF)
0x0DD   1     Timestamp seconds
0x0DE   1     Timestamp minutes
0x0DF   1     Timestamp hours
0x1B8   4     Disk signature (serial number)
0x1BC   2     Reserved (usually 0x0000)
0x1BE   16    Partition entry 1
0x1CE   16    Partition entry 2
0x1DE   16    Partition entry 3
0x1EE   16    Partition entry 4
0x1FE   2     Boot signature (0xAA55)
```

**Partition Entry (16 bytes)**
```
Offset  Size  Field
------  ----  -----
0x00    1     Status (0x80=bootable, 0x00=inactive)
0x01    3     CHS address of first sector
0x04    1     Partition type (see type table)
0x05    3     CHS address of last sector
0x08    4     LBA of first sector
0x0C    4     Number of sectors
```

**CHS Address (3 bytes)**
```
Byte 0: Head (0-255)
Byte 1: Sector (bits 0-5, 1-63) + Cylinder high bits (6-7)
Byte 2: Cylinder low bits (0-7)
Combined: 10-bit cylinder (0-1023), 8-bit head, 6-bit sector
```

#### Partition Type Codes (Manifesto Types)

| Code | Type | Liberation Status |
|------|------|-------------------|
| 0x00 | Empty | Unoccupied zone |
| 0x01 | FAT12 | FAT12 territory |
| 0x04 | FAT16 (< 32MB) | FAT16 territory |
| 0x05 | Extended | Contains logical zones |
| 0x06 | FAT16B | FAT16 territory |
| 0x07 | NTFS/exFAT/HPFS | Corporate territories |
| 0x0B | FAT32 | FAT32 territory |
| 0x0C | FAT32 (LBA) | FAT32 territory (LBA) |
| 0x0E | FAT16B (LBA) | FAT16 territory (LBA) |
| 0x0F | Extended (LBA) | Extended zones (LBA) |
| 0x82 | Linux Swap | Linux swap space |
| 0x83 | Linux Native | Linux ext/etc territories |
| 0xEE | GPT Protective | GPT protection marker |
| 0xEF | EFI System | EFI system territory |

#### Rust Pseudocode

```rust
// MBR ZONE TABLE
pub struct MbrZoneTable {
    vault: Arc<dyn Vault>,
    sector_size: u32,
    drive_number: u8,
    timestamp_hours: u8,
    timestamp_minutes: u8,
    timestamp_seconds: u8,
    serial_number: u32,
    zones: Vec<Zone>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum MbrPartitionType {
    Empty = 0x00,
    Fat12 = 0x01,
    Fat16 = 0x04,
    Extended = 0x05,
    Fat16B = 0x06,
    HpfsNtfsExFat = 0x07,
    Fat32 = 0x0B,
    Fat32Lba = 0x0C,
    Fat16BLba = 0x0E,
    ExtendedLba = 0x0F,
    LinuxSwap = 0x82,
    LinuxNative = 0x83,
    GptProtective = 0xEE,
    EfiSystemPartition = 0xEF,
}

impl MbrPartitionType {
    pub fn from_byte(b: u8) -> Self {
        match b {
            0x00 => Self::Empty,
            0x01 => Self::Fat12,
            0x04 => Self::Fat16,
            0x05 => Self::Extended,
            0x06 => Self::Fat16B,
            0x07 => Self::HpfsNtfsExFat,
            0x0B => Self::Fat32,
            0x0C => Self::Fat32Lba,
            0x0E => Self::Fat16BLba,
            0x0F => Self::ExtendedLba,
            0x82 => Self::LinuxSwap,
            0x83 => Self::LinuxNative,
            0xEE => Self::GptProtective,
            0xEF => Self::EfiSystemPartition,
            _ => Self::Empty, // Unknown types treated as empty
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Empty => "Empty",
            Self::Fat12 => "FAT12",
            Self::Fat16 => "FAT16",
            Self::Extended => "Extended Partition",
            Self::Fat16B => "FAT16B",
            Self::HpfsNtfsExFat => "HPFS/NTFS/exFAT",
            Self::Fat32 => "FAT32",
            Self::Fat32Lba => "FAT32 (LBA)",
            Self::Fat16BLba => "FAT16B (LBA)",
            Self::ExtendedLba => "Extended Partition (LBA)",
            Self::LinuxSwap => "Linux swap",
            Self::LinuxNative => "Linux native",
            Self::GptProtective => "GPT Protective Partition",
            Self::EfiSystemPartition => "EFI System Partition",
        }
    }
}

#[repr(C, packed)]
pub struct CHSAddress {
    pub head: u8,
    pub sector_cyl_high: u8,
    pub cyl_low: u8,
}

impl CHSAddress {
    pub fn parse(bytes: &[u8]) -> Self {
        Self {
            head: bytes[0],
            sector_cyl_high: bytes[1],
            cyl_low: bytes[2],
        }
    }

    pub fn cylinder(&self) -> u16 {
        ((self.sector_cyl_high as u16 & 0xC0) << 2) | (self.cyl_low as u16)
    }

    pub fn head(&self) -> u8 {
        self.head
    }

    pub fn sector(&self) -> u8 {
        self.sector_cyl_high & 0x3F
    }
}

pub struct MbrZone {
    pub active: bool,
    pub partition_type: MbrPartitionType,
    pub chs_start: CHSAddress,
    pub chs_end: CHSAddress,
    pub lba_start: u32,
    pub lba_length: u32,
    pub zone: Zone,
}

impl MbrZoneTable {
    pub fn decrypt(vault: Arc<dyn Vault>, sector_size: u32) -> Result<Self> {
        let mut pipeline = vault.expose_pipeline();

        // READ MBR METADATA
        pipeline.seek(SeekFrom::Start(0xDC))?;
        let mut metadata = [0u8; 4];
        pipeline.read_exact(&mut metadata)?;
        let drive_number = metadata[0];
        let timestamp_seconds = metadata[1];
        let timestamp_minutes = metadata[2];
        let timestamp_hours = metadata[3];

        // READ DISK SIGNATURE
        pipeline.seek(SeekFrom::Start(0x1B8))?;
        let mut sig_bytes = [0u8; 4];
        pipeline.read_exact(&mut sig_bytes)?;
        let serial_number = u32::from_le_bytes(sig_bytes);

        // VERIFY BOOT SIGNATURE
        pipeline.seek(SeekFrom::Start(0x1FE))?;
        let mut boot_sig = [0u8; 2];
        pipeline.read_exact(&mut boot_sig)?;
        if u16::from_le_bytes(boot_sig) != 0xAA55 {
            return Err("Invalid MBR boot signature".into());
        }

        // READ PARTITION TABLE (4 entries)
        pipeline.seek(SeekFrom::Start(0x1BE))?;
        let mut partition_table = [0u8; 64];
        pipeline.read_exact(&mut partition_table)?;

        let mut zones = Vec::new();

        for i in 0..4 {
            let entry = &partition_table[i * 16..(i + 1) * 16];

            let status = entry[0];
            let partition_type = MbrPartitionType::from_byte(entry[4]);

            if partition_type == MbrPartitionType::Empty {
                continue; // Skip empty entries
            }

            let chs_start = CHSAddress::parse(&entry[1..4]);
            let chs_end = CHSAddress::parse(&entry[5..8]);
            let lba_start = u32::from_le_bytes([entry[8], entry[9], entry[10], entry[11]]);
            let lba_length = u32::from_le_bytes([entry[12], entry[13], entry[14], entry[15]]);

            let offset = (lba_start as u64) * (sector_size as u64);
            let length = (lba_length as u64) * (sector_size as u64);

            // Create partial pipeline for this zone
            let zone_pipeline = Box::new(
                PartialPipeline::new(
                    vault.expose_pipeline(),
                    offset,
                    length
                )?
            );

            let zone = Zone {
                offset,
                length,
                zone_type: partition_type.name().to_string(),
                pipeline: zone_pipeline,
            };

            zones.push(zone);
        }

        Ok(MbrZoneTable {
            vault,
            sector_size,
            drive_number,
            timestamp_hours,
            timestamp_minutes,
            timestamp_seconds,
            serial_number,
            zones,
        })
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
```

---

### Zone Table 2: GPT - "MODERN DIVISION"
**Brand Name:** `GptZoneTable`
**Codename:** "GUID PARTITION TABLE"
**Location:** `TotalImage.IO/Partitions/GptPartitionTable.cs`

#### Purpose
Decrypt GUID Partition Table (modern UEFI standard)

#### Structure

**GPT Layout**
```
LBA 0:   Protective MBR (for compatibility)
LBA 1:   GPT Header
LBA 2-33: Partition Entry Array (typically)
LBA ...: Partition data
LBA -34 to -2: Backup Partition Entry Array
LBA -1:  Backup GPT Header
```

**GPT Header (LBA 1, 92 bytes minimum)**
```
Offset  Size  Field
------  ----  -----
0x00    8     Signature ("EFI PART")
0x08    4     Revision (usually 0x00010000)
0x0C    4     Header size in bytes (usually 92)
0x10    4     CRC32 of header (with this field zeroed)
0x14    4     Reserved (must be zero)
0x18    8     Current LBA (location of this header)
0x20    8     Backup LBA (location of backup header)
0x28    8     First usable LBA
0x30    8     Last usable LBA
0x38    16    Disk GUID
0x48    8     Partition entry array LBA (usually 2)
0x50    4     Number of partition entries (usually 128)
0x54    4     Size of each partition entry (usually 128)
0x58    4     CRC32 of partition entry array
0x5C    *     Reserved (to end of sector)
```

**GPT Partition Entry (128 bytes)**
```
Offset  Size  Field
------  ----  -----
0x00    16    Partition type GUID
0x10    16    Unique partition GUID
0x20    8     First LBA
0x28    8     Last LBA (inclusive)
0x30    8     Attribute flags
0x38    72    Partition name (UTF-16LE, 36 characters)
```

**Attribute Flags**
```
Bit 0: Platform required (system partition)
Bit 1: EFI firmware should ignore this
Bit 2: Legacy BIOS bootable
Bits 3-47: Reserved
Bits 48-63: Partition type-specific
```

#### Known Partition Type GUIDs

| GUID | Type | Liberation Status |
|------|------|-------------------|
| `00000000-0000-0000-0000-000000000000` | Empty | Unoccupied |
| `C12A7328-F81F-11D2-BA4B-00A0C93EC93B` | EFI System | EFI boot territory |
| `21686148-6449-6E6F-744E-656564454649` | BIOS Boot | GRUB boot territory |
| `E3C9E316-0B5C-4DB8-817D-F92DF00215AE` | MS Reserved | Corporate reserved |
| `EBD0A0A2-B9E5-4433-87C0-68B6B72699C7` | Basic Data | MS file system territory |
| `DE94BBA4-06D1-4D40-A16A-BFD50179D6AC` | Windows RE | Windows recovery |
| `0FC63DAF-8483-4772-8E79-3D69D8477DE4` | Linux Data | Linux file system |
| `44479540-F297-41B2-9AF7-D131D5F0458A` | Linux Root x86 | Linux root (x86) |
| `4F68BCE3-E8CD-4DB1-96E7-FBCAF984B709` | Linux Root x64 | Linux root (x64) |
| `69DAD710-2CE4-4E3C-B16C-21A1D49ABED3` | Linux Root ARM32 | Linux root (ARM32) |
| `B921B045-1DF0-41C3-AF44-4C6F280D3FAE` | Linux Root ARM64 | Linux root (ARM64) |
| `BC13C2FF-59E6-4262-A352-B275FD6F7172` | Linux Boot | Linux boot partition |
| `0657FD6D-A4AB-43C4-84E5-0933C84B4F4F` | Linux Swap | Linux swap space |
| `933AC7E1-2EB4-4F13-B844-0E14E2AEF915` | Linux Home | Linux home partition |
| `48465300-0000-11AA-AA11-00306543ECAC` | HFS+ | Apple HFS+ |
| `7C3457EF-0000-11AA-AA11-00306543ECAC` | APFS | Apple APFS |

#### Rust Pseudocode

```rust
use uuid::Uuid;

// GPT HEADER
#[repr(C, packed)]
pub struct GptHeader {
    pub signature: [u8; 8],           // "EFI PART"
    pub revision: u32,
    pub header_size: u32,
    pub header_crc32: u32,
    pub reserved: u32,
    pub current_lba: u64,
    pub backup_lba: u64,
    pub first_usable_lba: u64,
    pub last_usable_lba: u64,
    pub disk_guid: [u8; 16],
    pub partition_entry_lba: u64,
    pub num_partition_entries: u32,
    pub partition_entry_size: u32,
    pub partition_array_crc32: u32,
}

impl GptHeader {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 92 {
            return Err("GPT header too small".into());
        }

        let signature = bytes[0..8].try_into().unwrap();
        if &signature != b"EFI PART" {
            return Err("Invalid GPT signature".into());
        }

        Ok(Self {
            signature,
            revision: u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
            header_size: u32::from_le_bytes(bytes[12..16].try_into().unwrap()),
            header_crc32: u32::from_le_bytes(bytes[16..20].try_into().unwrap()),
            reserved: u32::from_le_bytes(bytes[20..24].try_into().unwrap()),
            current_lba: u64::from_le_bytes(bytes[24..32].try_into().unwrap()),
            backup_lba: u64::from_le_bytes(bytes[32..40].try_into().unwrap()),
            first_usable_lba: u64::from_le_bytes(bytes[40..48].try_into().unwrap()),
            last_usable_lba: u64::from_le_bytes(bytes[48..56].try_into().unwrap()),
            disk_guid: bytes[56..72].try_into().unwrap(),
            partition_entry_lba: u64::from_le_bytes(bytes[72..80].try_into().unwrap()),
            num_partition_entries: u32::from_le_bytes(bytes[80..84].try_into().unwrap()),
            partition_entry_size: u32::from_le_bytes(bytes[84..88].try_into().unwrap()),
            partition_array_crc32: u32::from_le_bytes(bytes[88..92].try_into().unwrap()),
        })
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u64)]
pub enum GptPartitionFlags {
    PlatformRequired = 1 << 0,
    EfiFirmwareIgnore = 1 << 1,
    LegacyBiosBootable = 1 << 2,
}

pub struct GptZone {
    pub type_guid: Uuid,
    pub partition_guid: Uuid,
    pub first_lba: u64,
    pub last_lba: u64,
    pub flags: u64,
    pub name: String,
    pub zone: Zone,
}

pub struct GptZoneTable {
    vault: Arc<dyn Vault>,
    sector_size: u32,
    header: GptHeader,
    zones: Vec<Zone>,
}

impl GptZoneTable {
    pub fn decrypt(vault: Arc<dyn Vault>, sector_size: u32) -> Result<Self> {
        let mut pipeline = vault.expose_pipeline();

        // READ GPT HEADER (LBA 1)
        pipeline.seek(SeekFrom::Start(sector_size as u64))?;
        let mut header_bytes = vec![0u8; 92];
        pipeline.read_exact(&mut header_bytes)?;
        let header = GptHeader::parse(&header_bytes)?;

        // TODO: Verify CRC32

        // READ PARTITION ENTRY ARRAY
        let table_offset = header.partition_entry_lba * sector_size as u64;
        pipeline.seek(SeekFrom::Start(table_offset))?;

        let mut zones = Vec::new();

        for i in 0..header.num_partition_entries {
            let mut entry_bytes = vec![0u8; header.partition_entry_size as usize];
            pipeline.read_exact(&mut entry_bytes)?;

            let type_guid = Uuid::from_bytes_le(
                entry_bytes[0..16].try_into().unwrap()
            );

            if type_guid == Uuid::nil() {
                continue; // Empty entry
            }

            let partition_guid = Uuid::from_bytes_le(
                entry_bytes[16..32].try_into().unwrap()
            );

            let first_lba = u64::from_le_bytes(
                entry_bytes[32..40].try_into().unwrap()
            );
            let last_lba = u64::from_le_bytes(
                entry_bytes[40..48].try_into().unwrap()
            );
            let flags = u64::from_le_bytes(
                entry_bytes[48..56].try_into().unwrap()
            );

            // Parse UTF-16LE partition name
            let name_bytes = &entry_bytes[56..128];
            let name = String::from_utf16_lossy(
                &name_bytes
                    .chunks_exact(2)
                    .map(|c| u16::from_le_bytes([c[0], c[1]]))
                    .take_while(|&c| c != 0)
                    .collect::<Vec<u16>>()
            );

            let offset = first_lba * sector_size as u64;
            let length = (last_lba - first_lba + 1) * sector_size as u64;

            let zone_pipeline = Box::new(
                PartialPipeline::new(
                    vault.expose_pipeline(),
                    offset,
                    length
                )?
            );

            let zone_type = Self::guid_to_type_name(&type_guid);

            let zone = Zone {
                offset,
                length,
                zone_type,
                pipeline: zone_pipeline,
            };

            zones.push(zone);
        }

        Ok(GptZoneTable {
            vault,
            sector_size,
            header,
            zones,
        })
    }

    fn guid_to_type_name(guid: &Uuid) -> String {
        // Map GUIDs to friendly names
        match guid.to_string().to_uppercase().as_str() {
            "C12A7328-F81F-11D2-BA4B-00A0C93EC93B" => "EFI System Partition",
            "21686148-6449-6E6F-744E-656564454649" => "BIOS Boot Partition",
            "E3C9E316-0B5C-4DB8-817D-F92DF00215AE" => "Microsoft Reserved",
            "EBD0A0A2-B9E5-4433-87C0-68B6B72699C7" => "Basic Data Partition",
            "DE94BBA4-06D1-4D40-A16A-BFD50179D6AC" => "Windows Recovery",
            "0FC63DAF-8483-4772-8E79-3D69D8477DE4" => "Linux Data Partition",
            "0657FD6D-A4AB-43C4-84E5-0933C84B4F4F" => "Linux Swap",
            "48465300-0000-11AA-AA11-00306543ECAC" => "HFS+",
            "7C3457EF-0000-11AA-AA11-00306543ECAC" => "APFS",
            _ => format!("Unknown ({})", guid),
        }.to_string()
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
```

---

### Zone Table 3: NoPartitionTable - "DIRECT TERRITORY"
**Brand Name:** `DirectTerritory`
**Codename:** "NO DIVISIONS"
**Location:** `TotalImage.IO/Partitions/NoPartitionTable.cs`

#### Purpose
Represents a storage device with no partition table - entire device is one territory

```rust
pub struct DirectTerritory {
    vault: Arc<dyn Vault>,
    zone: Zone,
}

impl DirectTerritory {
    pub fn new(vault: Arc<dyn Vault>) -> Self {
        let length = vault.length();
        let zone = Zone {
            offset: 0,
            length,
            zone_type: "Entire disk".to_string(),
            pipeline: Box::new(vault.expose_pipeline()),
        };

        Self { vault, zone }
    }
}

impl ZoneTable for DirectTerritory {
    fn identify(&self) -> &str {
        "No Partition Table"
    }

    fn enumerate_zones(&self) -> &[Zone] {
        std::slice::from_ref(&self.zone)
    }
}
```

---

## Combined MBR/GPT Factory

```rust
pub struct MbrGptFactory;

impl ZoneTableFactory for MbrGptFactory {
    fn can_detect(&self, vault: &dyn Vault) -> bool {
        // Check for boot signature
        let mut pipeline = vault.expose_pipeline();
        pipeline.seek(SeekFrom::Start(0x1FE)).ok()?;
        let mut sig = [0u8; 2];
        pipeline.read_exact(&mut sig).ok()?;
        u16::from_le_bytes(sig) == 0xAA55
    }

    fn detect(&self, vault: &dyn Vault) -> Result<Box<dyn ZoneTable>> {
        let mut pipeline = vault.expose_pipeline();

        // Check if it's GPT (protective MBR + GPT header)
        pipeline.seek(SeekFrom::Start(0x1C2))?; // Partition type of first entry
        let mut part_type = [0u8; 1];
        pipeline.read_exact(&mut part_type)?;

        if part_type[0] == 0xEE {
            // GPT protective partition detected
            pipeline.seek(SeekFrom::Start(512))?; // LBA 1
            let mut gpt_sig = [0u8; 8];
            pipeline.read_exact(&mut gpt_sig)?;

            if &gpt_sig == b"EFI PART" {
                return Ok(Box::new(GptZoneTable::decrypt(Arc::new(vault), 512)?));
            }
        }

        // Default to MBR
        Ok(Box::new(MbrZoneTable::decrypt(Arc::new(vault), 512)?))
    }
}
```

---

## Solidarity Dependencies

```toml
[dependencies]
# UUID OPERATIONS (for GPT)
uuid = { version = "1.6", features = ["v4"] }

# CRC32 VERIFICATION
crc32fast = "1.3"

# UTF-16 ENCODING (for GPT partition names)
encoding_rs = "0.8"
```

---

## Status

- ✅ ZoneTable trait designed
- ✅ MBR ZoneTable pseudocode complete
- ✅ GPT ZoneTable pseudocode complete
- ✅ DirectTerritory pseudocode complete
- ✅ MBR/GPT factory pseudocode complete
- ⏳ Extended partition support pending
- ⏳ Integration with Vault and Territory layers pending

**Next Action:** Document Front Collective (UI layer)
