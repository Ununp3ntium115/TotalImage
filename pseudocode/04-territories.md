# Territories Module - Pseudocode Documentation

**Component:** `totalimage-territories`  
**Location:** `crates/totalimage-territories/src/`  
**Purpose:** Filesystem parsers (FAT, exFAT, ISO, NTFS)

---

## Table of Contents

1. [Overview](#overview)
2. [FAT Parser](#fat-parser)
3. [exFAT Parser](#exfat-parser)
4. [ISO Parser](#iso-parser)
5. [NTFS Parser](#ntfs-parser)
6. [Code References](#code-references)

---

## Overview

The territories module provides filesystem parsing:
- **FAT**: FAT12, FAT16, FAT32 with LFN support
- **exFAT**: Extended FAT for flash media
- **ISO**: ISO-9660 with Joliet extensions
- **NTFS**: Windows NTFS (read-only)

**Code Reference:** `crates/totalimage-territories/src/lib.rs`

---

## FAT Parser

File Allocation Table filesystem.

**Code Reference:** `crates/totalimage-territories/src/fat/mod.rs:1-940`

### FAT Structure

```pseudocode
STRUCTURE FatTerritory:
    bpb: BiosParameterBlock         // Boot Parameter Block
    fat_table: BYTE_ARRAY           // FAT allocation table
    identifier: STRING               // Filesystem identifier
    fat32_root_cluster: UINT32      // FAT32 root cluster (0 for FAT12/16)
END STRUCTURE
```

### BIOS Parameter Block

**Code Reference:** `crates/totalimage-territories/src/fat/types.rs:57-260`

```pseudocode
STRUCTURE BiosParameterBlock:
    bytes_per_sector: UINT16        // Usually 512
    sectors_per_cluster: UINT8      // Power of 2 (1, 2, 4, 8, 16, 32, 64, 128)
    reserved_sectors: UINT16         // Reserved sectors before FAT
    num_fats: UINT8                 // Number of FATs (usually 2)
    root_entries: UINT16            // Root directory entries (0 for FAT32)
    total_sectors_16: UINT16        // Total sectors (if < 65536)
    media_descriptor: UINT8         // Media type (0xF0 = floppy, 0xF8 = hard disk)
    sectors_per_fat_16: UINT16      // Sectors per FAT (0 for FAT32)
    sectors_per_track: UINT16       // CHS geometry
    num_heads: UINT16               // CHS geometry
    hidden_sectors: UINT32          // Hidden sectors before partition
    total_sectors_32: UINT32        // Total sectors (if >= 65536)
    fat_type: FatType               // FAT12, FAT16, or FAT32
END STRUCTURE

FUNCTION BiosParameterBlock.from_bytes(bytes: BYTE_ARRAY[512]) -> Result<BiosParameterBlock>:
    // Parse common BPB fields (offsets 11-35)
    bytes_per_sector = read_u16_le(bytes, 11)
    sectors_per_cluster = bytes[13]
    reserved_sectors = read_u16_le(bytes, 14)
    num_fats = bytes[16]
    root_entries = read_u16_le(bytes, 17)
    total_sectors_16 = read_u16_le(bytes, 19)
    media_descriptor = bytes[21]
    sectors_per_fat_16 = read_u16_le(bytes, 22)
    sectors_per_track = read_u16_le(bytes, 24)
    num_heads = read_u16_le(bytes, 26)
    hidden_sectors = read_u32_le(bytes, 28)
    total_sectors_32 = read_u32_le(bytes, 32)
    
    // Validate
    IF sectors_per_cluster == 0:
        RETURN Error::InvalidTerritory("Invalid sectors_per_cluster: 0")
    IF bytes_per_sector == 0:
        RETURN Error::InvalidTerritory("Invalid bytes_per_sector: 0")
    
    // Determine total sectors
    total_sectors = IF total_sectors_16 != 0 THEN total_sectors_16 ELSE total_sectors_32
    
    // Calculate FAT type based on cluster count
    root_dir_sectors = calculate_root_dir_sectors(root_entries, bytes_per_sector)
    sectors_per_fat = IF sectors_per_fat_16 != 0 THEN sectors_per_fat_16 ELSE read_u32_le(bytes, 36)
    
    fat_size = num_fats * sectors_per_fat
    non_data_sectors = reserved_sectors + fat_size + root_dir_sectors
    data_sectors = total_sectors - non_data_sectors
    cluster_count = data_sectors / sectors_per_cluster
    
    fat_type = IF cluster_count < 4085:
        FatType::Fat12
    ELSE IF cluster_count < 65525:
        FatType::Fat16
    ELSE:
        FatType::Fat32
    
    RETURN BiosParameterBlock {
        bytes_per_sector: bytes_per_sector,
        sectors_per_cluster: sectors_per_cluster,
        reserved_sectors: reserved_sectors,
        num_fats: num_fats,
        root_entries: root_entries,
        total_sectors_16: total_sectors_16,
        media_descriptor: media_descriptor,
        sectors_per_fat_16: sectors_per_fat_16,
        sectors_per_track: sectors_per_track,
        num_heads: num_heads,
        hidden_sectors: hidden_sectors,
        total_sectors_32: total_sectors_32,
        fat_type: fat_type
    }
END FUNCTION

FUNCTION BiosParameterBlock.serialize(bytes: MUTABLE<BYTE_ARRAY>):
    // Write common BPB fields
    write_u16_le(bytes, 11, this.bytes_per_sector)
    bytes[13] = this.sectors_per_cluster
    write_u16_le(bytes, 14, this.reserved_sectors)
    bytes[16] = this.num_fats
    write_u16_le(bytes, 17, this.root_entries)
    write_u16_le(bytes, 19, this.total_sectors_16)
    bytes[21] = this.media_descriptor
    write_u16_le(bytes, 22, this.sectors_per_fat_16)
    write_u16_le(bytes, 24, this.sectors_per_track)
    write_u16_le(bytes, 26, this.num_heads)
    write_u32_le(bytes, 28, this.hidden_sectors)
    write_u32_le(bytes, 32, this.total_sectors_32)
END FUNCTION
```

### FAT Parsing

**Code Reference:** `crates/totalimage-territories/src/fat/mod.rs:35-83`

```pseudocode
FUNCTION FatTerritory.parse(stream: ReadSeek) -> Result<FatTerritory>:
    // Read boot sector
    stream.seek(0)
    boot_sector = read_bytes(stream, 512)
    
    // Parse BPB
    bpb = BiosParameterBlock::from_bytes(boot_sector)
    
    // Calculate FAT size with checked arithmetic
    fat_size_u64 = checked_multiply_u32_to_u64(
        bpb.sectors_per_fat(),
        bpb.bytes_per_sector,
        "FAT table size"
    )
    
    // Validate allocation size
    fat_size = validate_allocation_size(
        fat_size_u64,
        MAX_FAT_TABLE_SIZE,
        "FAT table"
    )
    
    // Read FAT table
    stream.seek(bpb.fat_offset())
    fat_table = read_bytes(stream, fat_size)
    
    // Get FAT32 root cluster if applicable
    fat32_root_cluster = IF bpb.fat_type == FatType::Fat32:
        read_u32_le(boot_sector, 44)
    ELSE:
        0
    
    identifier = format("{} filesystem", bpb.fat_type)
    
    RETURN FatTerritory {
        bpb: bpb,
        fat_table: fat_table,
        identifier: identifier,
        fat32_root_cluster: fat32_root_cluster
    }
END FUNCTION
```

### FAT Entry Reading

**Code Reference:** `crates/totalimage-territories/src/fat/mod.rs:93-161`

```pseudocode
FUNCTION FatTerritory.read_fat_entry(cluster: UINT32) -> OPTIONAL<UINT32>:
    SWITCH this.bpb.fat_type:
        CASE FatType::Fat12:
            RETURN this.read_fat12_entry(cluster)
        CASE FatType::Fat16:
            RETURN this.read_fat16_entry(cluster)
        CASE FatType::Fat32:
            RETURN this.read_fat32_entry(cluster)
    END SWITCH
END FUNCTION

FUNCTION FatTerritory.read_fat12_entry(cluster: UINT32) -> OPTIONAL<UINT32>:
    // FAT12: 12 bits per entry, packed
    offset = (cluster + (cluster / 2)) as usize
    
    IF offset + 1 >= this.fat_table.length:
        RETURN NULL
    
    value = IF cluster & 1 == 0:
        // Even cluster: lower 12 bits
        read_u16_le(this.fat_table, offset) & 0x0FFF
    ELSE:
        // Odd cluster: upper 12 bits
        read_u16_le(this.fat_table, offset) >> 4
    
    // Check for end of chain
    IF value >= 0xFF8 OR value == 0 OR value == 1:
        RETURN NULL
    
    RETURN value as UINT32
END FUNCTION

FUNCTION FatTerritory.read_fat16_entry(cluster: UINT32) -> OPTIONAL<UINT32>:
    // FAT16: 16 bits per entry
    offset = cluster * 2
    
    IF offset + 1 >= this.fat_table.length:
        RETURN NULL
    
    value = read_u16_le(this.fat_table, offset)
    
    // Check for end of chain
    IF value >= 0xFFF8 OR value == 0 OR value == 1:
        RETURN NULL
    
    RETURN value as UINT32
END FUNCTION

FUNCTION FatTerritory.read_fat32_entry(cluster: UINT32) -> OPTIONAL<UINT32>:
    // FAT32: 28 bits per entry (top 4 bits reserved)
    offset = cluster * 4
    
    IF offset + 3 >= this.fat_table.length:
        RETURN NULL
    
    value = read_u32_le(this.fat_table, offset) & 0x0FFFFFFF
    
    // Check for end of chain
    IF value >= 0x0FFFFFF8 OR value == 0 OR value == 1:
        RETURN NULL
    
    RETURN value
END FUNCTION

FUNCTION FatTerritory.get_cluster_chain(start_cluster: UINT32) -> ARRAY<UINT32>:
    chain = []
    cluster = start_cluster
    count = 0
    max_clusters = 65536  // Prevent infinite loops
    
    WHILE count < max_clusters:
        IF cluster < 2:
            BREAK
        
        chain.append(cluster)
        
        next = this.read_fat_entry(cluster)
        IF next IS NULL:
            BREAK
        
        cluster = next
        count = count + 1
    END WHILE
    
    RETURN chain
END FUNCTION
```

### Directory Reading

**Code Reference:** `crates/totalimage-territories/src/fat/mod.rs:200-400`

```pseudocode
FUNCTION FatTerritory.read_directory(stream: ReadSeek, cluster: UINT32) -> Result<ARRAY<DirectoryEntry>>:
    // Get cluster chain
    cluster_chain = this.get_cluster_chain(cluster)
    
    // Read directory entries
    entries = []
    lfn_entries = []
    
    FOR EACH cluster IN cluster_chain:
        cluster_offset = this.cluster_to_offset(cluster)
        stream.seek(cluster_offset)
        
        bytes_per_cluster = this.bpb.bytes_per_cluster()
        cluster_data = read_bytes(stream, bytes_per_cluster)
        
        // Parse entries (32 bytes each)
        FOR i = 0 TO bytes_per_cluster - 32 STEP 32:
            entry_bytes = cluster_data[i:i + 32]
            
            // Check for end of directory
            IF entry_bytes[0] == 0x00:
                BREAK
            
            // Check for deleted entry
            IF entry_bytes[0] == 0xE5:
                CONTINUE
            
            // Check for LFN entry
            IF entry_bytes[11] == 0x0F:
                lfn_entry = LfnEntry::from_bytes(entry_bytes)
                lfn_entries.append(lfn_entry)
                CONTINUE
            
            // Parse regular entry
            entry = DirectoryEntry::from_bytes_with_lfn(entry_bytes, lfn_entries)
            IF entry IS NOT NULL:
                entries.append(entry)
                lfn_entries = []  // Clear LFN entries after use
        END FOR
    END FOR
    
    RETURN entries
END FUNCTION
```

---

## exFAT Parser

Extended FAT filesystem.

**Code Reference:** `crates/totalimage-territories/src/exfat/mod.rs:29-585`

```pseudocode
STRUCTURE ExfatTerritory:
    boot_sector: ExfatBootSector
    volume_label: OPTIONAL<STRING>
    bytes_per_sector: UINT32
    bytes_per_cluster: UINT32
    cluster_heap_offset: UINT64
    cluster_count: UINT32
    root_dir_cluster: UINT32
    volume_length: UINT64
    identifier: STRING
END STRUCTURE

FUNCTION ExfatTerritory.parse(reader: ReadSeek) -> Result<ExfatTerritory>:
    // Read boot sector
    boot_bytes = read_bytes(reader, 512)
    boot_sector = ExfatBootSector::parse(boot_bytes)
    
    bytes_per_sector = boot_sector.bytes_per_sector()
    bytes_per_cluster = boot_sector.bytes_per_cluster()
    cluster_heap_offset = boot_sector.cluster_heap_offset * bytes_per_sector
    volume_length = boot_sector.volume_length * bytes_per_sector
    
    identifier = format("exFAT {} clusters, {} bytes/cluster",
        boot_sector.cluster_count, bytes_per_cluster)
    
    RETURN ExfatTerritory {
        boot_sector: boot_sector,
        volume_label: NULL,
        bytes_per_sector: bytes_per_sector,
        bytes_per_cluster: bytes_per_cluster,
        cluster_heap_offset: cluster_heap_offset,
        cluster_count: boot_sector.cluster_count,
        root_dir_cluster: boot_sector.root_dir_cluster,
        volume_length: volume_length,
        identifier: identifier
    }
END FUNCTION
```

---

## ISO Parser

ISO-9660 filesystem with Joliet extensions.

**Code Reference:** `crates/totalimage-territories/src/iso/mod.rs`

```pseudocode
STRUCTURE IsoTerritory:
    primary_volume: PrimaryVolumeDescriptor
    joliet_volume: OPTIONAL<SupplementaryVolumeDescriptor>
    sector_size: UINT32
    identifier: STRING
END STRUCTURE

FUNCTION IsoTerritory.parse(stream: ReadSeek) -> Result<IsoTerritory>:
    // ISO-9660 volume descriptors start at sector 16 (LBA 16)
    descriptor_offset = 16 * 2048  // ISO uses 2048-byte sectors
    
    primary_volume = NULL
    joliet_volume = NULL
    
    WHILE true:
        stream.seek(descriptor_offset)
        descriptor_bytes = read_bytes(stream, 2048)
        
        descriptor_type = descriptor_bytes[0]
        
        SWITCH descriptor_type:
            CASE 1:  // Primary Volume Descriptor
                primary_volume = PrimaryVolumeDescriptor::parse(descriptor_bytes)
            
            CASE 2:  // Supplementary Volume Descriptor (Joliet)
                IF descriptor_bytes[1:6] == "CD001":
                    joliet_volume = SupplementaryVolumeDescriptor::parse(descriptor_bytes)
            
            CASE 255:  // Volume Descriptor Set Terminator
                BREAK
        END SWITCH
        
        descriptor_offset = descriptor_offset + 2048
    END WHILE
    
    IF primary_volume IS NULL:
        RETURN Error::InvalidTerritory("No primary volume descriptor found")
    
    identifier = IF joliet_volume IS NOT NULL:
        format("ISO-9660 (Joliet) filesystem")
    ELSE:
        format("ISO-9660 filesystem")
    
    RETURN IsoTerritory {
        primary_volume: primary_volume,
        joliet_volume: joliet_volume,
        sector_size: 2048,
        identifier: identifier
    }
END FUNCTION
```

---

## NTFS Parser

Windows NTFS filesystem (read-only).

**Code Reference:** `crates/totalimage-territories/src/ntfs/mod.rs:38-593`

```pseudocode
STRUCTURE NtfsTerritory:
    ntfs: Ntfs                      // Underlying NTFS structure (from ntfs crate)
    reader: ReadSeek                // Filesystem reader
    volume_info: NtfsVolumeInfo    // Volume information
    identifier: STRING               // Filesystem identifier
END STRUCTURE

FUNCTION NtfsTerritory.parse(reader: ReadSeek) -> Result<NtfsTerritory>:
    // Use ntfs crate for parsing
    ntfs = Ntfs::new(reader)
    
    // Get volume information
    volume_info = get_volume_info(ntfs)
    
    identifier = format("NTFS filesystem")
    
    RETURN NtfsTerritory {
        ntfs: ntfs,
        reader: reader,
        volume_info: volume_info,
        identifier: identifier
    }
END FUNCTION
```

---

## Code References

### File Structure

```
crates/totalimage-territories/src/
├── lib.rs              # Module exports
├── fat/
│   ├── mod.rs          # FAT parser (lines 1-940)
│   └── types.rs         # FAT types (lines 1-800+)
├── exfat/
│   ├── mod.rs          # exFAT parser (lines 1-585)
│   └── types.rs         # exFAT types
├── iso/
│   ├── mod.rs          # ISO parser
│   ├── types.rs         # ISO types
│   └── rockridge.rs    # Rock Ridge extensions
└── ntfs/
    ├── mod.rs          # NTFS parser (lines 1-593)
    ├── types.rs         # NTFS types
    └── lznt1.rs         # LZNT1 decompression
```

### Key Functions

#### FAT (`fat/mod.rs`)
- `FatTerritory::parse`: `crates/totalimage-territories/src/fat/mod.rs:35-83`
- `FatTerritory::read_fat_entry`: `crates/totalimage-territories/src/fat/mod.rs:93-99`
- `FatTerritory::read_fat12_entry`: `crates/totalimage-territories/src/fat/mod.rs:102-122`
- `FatTerritory::read_fat16_entry`: `crates/totalimage-territories/src/fat/mod.rs:125-139`
- `FatTerritory::read_fat32_entry`: `crates/totalimage-territories/src/fat/mod.rs:142-161`
- `FatTerritory::get_cluster_chain`: `crates/totalimage-territories/src/fat/mod.rs:164-188`

#### FAT Types (`fat/types.rs`)
- `BiosParameterBlock::from_bytes`: `crates/totalimage-territories/src/fat/types.rs:62-160`
- `BiosParameterBlock::serialize`: `crates/totalimage-territories/src/fat/types.rs:260-280`
- `DirectoryEntry::from_bytes_with_lfn`: Directory entry parsing
- `LfnEntry::from_bytes`: Long filename entry parsing

---

## Cross-References

- **Core Traits:** See [01-core.md](01-core.md#territory-trait)
- **Zone Usage:** See [03-zones.md](03-zones.md) (territories read from zones)
- **Vault Usage:** See [02-vaults.md](02-vaults.md) (zones read from vaults)

---

**Document Version:** 1.0.0  
**Last Updated:** December 21, 2025  
**Next:** [05-pipeline.md](05-pipeline.md) - I/O Abstractions
