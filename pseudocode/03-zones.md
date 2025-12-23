# Zones Module - Pseudocode Documentation

**Component:** `totalimage-zones`  
**Location:** `crates/totalimage-zones/src/`  
**Purpose:** Partition table parsers (MBR, GPT)

---

## Table of Contents

1. [Overview](#overview)
2. [MBR Parser](#mbr-parser)
3. [GPT Parser](#gpt-parser)
4. [Code References](#code-references)

---

## Overview

The zones module provides partition table parsing:
- **MBR**: Master Boot Record (legacy BIOS)
- **GPT**: GUID Partition Table (modern UEFI)

**Code Reference:** `crates/totalimage-zones/src/lib.rs`

---

## MBR Parser

Master Boot Record partition table.

**Code Reference:** `crates/totalimage-zones/src/mbr/mod.rs:9-361`

### MBR Structure

```pseudocode
STRUCTURE MbrZoneTable:
    zones: ARRAY<Zone>              // Partition entries
    disk_signature: UINT32          // Disk signature at offset 0x1B8
    boot_signature: UINT16          // Boot signature (0xAA55)
END STRUCTURE

CONSTANTS:
    MBR_SIZE: UINT = 512                    // MBR is always 512 bytes
    BOOT_SIGNATURE: UINT16 = 0xAA55         // Required boot signature
    PARTITION_TABLE_OFFSET: UINT = 0x1BE    // First partition entry
    DISK_SIGNATURE_OFFSET: UINT = 0x1B8     // Disk signature location
    BOOT_SIGNATURE_OFFSET: UINT = 0x1FE     // Boot signature location
    PARTITION_ENTRY_SIZE: UINT = 16         // Each entry is 16 bytes
    NUM_PARTITIONS: UINT = 4                // MBR supports 4 primary partitions
END CONSTANTS
```

### MBR Parsing

**Code Reference:** `crates/totalimage-zones/src/mbr/mod.rs:68-136`

```pseudocode
FUNCTION MbrZoneTable.parse(stream: ReadSeek, sector_size: UINT32) -> Result<MbrZoneTable>:
    // Read entire MBR sector
    stream.seek(0)
    mbr = read_bytes(stream, MBR_SIZE)
    
    // Verify boot signature
    boot_signature = read_u16_le(mbr, BOOT_SIGNATURE_OFFSET)
    IF boot_signature != BOOT_SIGNATURE:
        RETURN Error::InvalidZoneTable(
            format("Invalid MBR boot signature: expected 0x{:04X}, got 0x{:04X}",
                BOOT_SIGNATURE, boot_signature)
        )
    
    // Read disk signature
    disk_signature = read_u32_le(mbr, DISK_SIGNATURE_OFFSET)
    
    // Parse partition entries
    zones = []
    FOR i = 0 TO NUM_PARTITIONS - 1:
        entry_offset = PARTITION_TABLE_OFFSET + (i * PARTITION_ENTRY_SIZE)
        entry = mbr[entry_offset:entry_offset + PARTITION_ENTRY_SIZE]
        
        // Parse entry fields
        status = entry[0]                    // Boot flag (0x80 = bootable)
        chs_start = CHSAddress::from_bytes(entry[1:4])
        partition_type = MbrPartitionType::from_byte(entry[4])
        chs_end = CHSAddress::from_bytes(entry[5:8])
        lba_start = read_u32_le(entry, 8)
        lba_length = read_u32_le(entry, 12)
        
        // Skip empty partitions
        IF partition_type == MbrPartitionType::Empty OR lba_length == 0:
            CONTINUE
        
        // Calculate byte offsets
        zone_offset = lba_start * sector_size
        zone_length = lba_length * sector_size
        
        // Create zone
        zone = Zone::new(
            index: i,
            offset: zone_offset,
            length: zone_length,
            zone_type: partition_type.name()
        )
        zones.append(zone)
    END FOR
    
    RETURN MbrZoneTable {
        zones: zones,
        disk_signature: disk_signature,
        boot_signature: boot_signature
    }
END FUNCTION

FUNCTION MbrZoneTable.is_gpt_protective() -> BOOLEAN:
    // Check if any partition is GPT protective (type 0xEE)
    RETURN this.zones.any(zone => zone.zone_type == "GPT Protective")
END FUNCTION

FUNCTION MbrZoneTable.serialize(sector_size: UINT32) -> BYTE_ARRAY:
    mbr = allocate_zeroed_buffer(MBR_SIZE)
    
    // Write disk signature
    write_u32_le(mbr, DISK_SIGNATURE_OFFSET, this.disk_signature)
    
    // Write partition entries
    FOR i = 0 TO MIN(this.zones.length, NUM_PARTITIONS) - 1:
        zone = this.zones[i]
        entry_offset = PARTITION_TABLE_OFFSET + (i * PARTITION_ENTRY_SIZE)
        
        // Status byte
        mbr[entry_offset] = IF i == 0 THEN 0x80 ELSE 0x00
        
        // CHS start
        lba_start = zone.offset / sector_size
        chs_start = CHSAddress::from_lba(lba_start, 255, 63)
        mbr[entry_offset + 1:entry_offset + 4] = chs_start.to_bytes()
        
        // Partition type
        partition_type = MbrPartitionType::from_name(zone.zone_type)
            OR MbrPartitionType::Fat32Lba
        mbr[entry_offset + 4] = partition_type.to_byte()
        
        // CHS end
        lba_end = (zone.offset + zone.length) / sector_size - 1
        chs_end = CHSAddress::from_lba(lba_end, 255, 63)
        mbr[entry_offset + 5:entry_offset + 8] = chs_end.to_bytes()
        
        // LBA start
        write_u32_le(mbr, entry_offset + 8, lba_start)
        
        // LBA length
        lba_length = zone.length / sector_size
        write_u32_le(mbr, entry_offset + 12, lba_length)
    END FOR
    
    // Write boot signature
    write_u16_le(mbr, BOOT_SIGNATURE_OFFSET, BOOT_SIGNATURE)
    
    RETURN mbr
END FUNCTION
```

### CHS Address

**Code Reference:** `crates/totalimage-zones/src/mbr/types.rs:119-162`

```pseudocode
STRUCTURE CHSAddress:
    cylinder: UINT16               // Cylinder (0-1023)
    head: UINT8                    // Head (0-255)
    sector: UINT8                   // Sector (1-63, 1-based)
END STRUCTURE

FUNCTION CHSAddress.from_bytes(bytes: BYTE_ARRAY[3]) -> CHSAddress:
    head = bytes[0]
    sector = bytes[1] & 0x3F        // Lower 6 bits
    cyl_high = (bytes[1] & 0xC0) << 2  // Upper 2 bits
    cyl_low = bytes[2]
    cylinder = cyl_high | cyl_low
    
    RETURN CHSAddress {
        cylinder: cylinder,
        head: head,
        sector: sector
    }
END FUNCTION

FUNCTION CHSAddress.to_bytes() -> BYTE_ARRAY[3]:
    cyl_high = (this.cylinder >> 8) & 0x03
    cyl_low = this.cylinder & 0xFF
    
    RETURN [
        this.head,
        (this.sector & 0x3F) | (cyl_high << 6),
        cyl_low
    ]
END FUNCTION

FUNCTION CHSAddress.from_lba(lba: UINT32, heads: UINT16, sectors_per_track: UINT16) -> CHSAddress:
    sectors_per_cylinder = heads * sectors_per_track
    cylinder = lba / sectors_per_cylinder
    remainder = lba % sectors_per_cylinder
    head = remainder / sectors_per_track
    sector = (remainder % sectors_per_track) + 1  // 1-based
    
    RETURN CHSAddress {
        cylinder: MIN(cylinder, 1023),
        head: MIN(head, 255),
        sector: MIN(sector, 63)
    }
END FUNCTION
```

---

## GPT Parser

GUID Partition Table.

**Code Reference:** `crates/totalimage-zones/src/gpt/mod.rs:9-850`

### GPT Structure

```pseudocode
STRUCTURE GptZoneTable:
    zones: ARRAY<Zone>              // Partition entries
    header: GptHeader               // Primary GPT header
    backup_header: OPTIONAL<GptHeader>  // Backup header (if validated)
END STRUCTURE

CONSTANTS:
    GPT_HEADER_LBA: UINT64 = 1              // GPT header at LBA 1
    GPT_SIGNATURE: BYTE_ARRAY[8] = "EFI PART"  // GPT signature
    GPT_REVISION: UINT32 = 0x00010000       // GPT revision 1.0
    DEFAULT_PARTITION_ENTRY_SIZE: UINT32 = 128  // Standard entry size
    MAX_PARTITION_ENTRIES: UINT32 = 128      // Default max entries
END CONSTANTS
```

### GPT Header

**Code Reference:** `crates/totalimage-zones/src/gpt/types.rs`

```pseudocode
STRUCTURE GptHeader:
    signature: BYTE_ARRAY[8]        // "EFI PART"
    revision: UINT32                // GPT revision
    header_size: UINT32             // Header size (usually 92)
    header_crc32: UINT32            // Header CRC32
    reserved: UINT32                 // Reserved (must be 0)
    current_lba: UINT64             // This header's LBA
    backup_lba: UINT64              // Backup header LBA
    first_usable_lba: UINT64        // First usable LBA
    last_usable_lba: UINT64         // Last usable LBA
    disk_guid: UUID                 // Disk GUID
    partition_entries_lba: UINT64   // Partition entries array LBA
    num_partition_entries: UINT32   // Number of entries
    partition_entry_size: UINT32    // Size of each entry
    partition_entries_crc32: UINT32 // Entries array CRC32
END STRUCTURE

FUNCTION GptHeader.from_bytes(bytes: BYTE_ARRAY) -> OPTIONAL<GptHeader>:
    // Verify signature
    IF bytes[0:8] != GPT_SIGNATURE:
        RETURN NULL
    
    header = GptHeader {
        signature: bytes[0:8],
        revision: read_u32_le(bytes, 8),
        header_size: read_u32_le(bytes, 12),
        header_crc32: read_u32_le(bytes, 16),
        reserved: read_u32_le(bytes, 20),
        current_lba: read_u64_le(bytes, 24),
        backup_lba: read_u64_le(bytes, 32),
        first_usable_lba: read_u64_le(bytes, 40),
        last_usable_lba: read_u64_le(bytes, 48),
        disk_guid: UUID::parse(bytes, 56),
        partition_entries_lba: read_u64_le(bytes, 72),
        num_partition_entries: read_u32_le(bytes, 80),
        partition_entry_size: read_u32_le(bytes, 84),
        partition_entries_crc32: read_u32_le(bytes, 88)
    }
    
    RETURN header
END FUNCTION

FUNCTION GptHeader.verify_header_crc32(bytes: BYTE_ARRAY) -> BOOLEAN:
    // Zero out CRC32 field for calculation
    bytes_copy = bytes.copy()
    bytes_copy[16:20] = [0, 0, 0, 0]
    
    // Calculate CRC32
    calculated = crc32(bytes_copy)
    
    RETURN calculated == this.header_crc32
END FUNCTION

FUNCTION GptHeader.verify_partition_entries_crc32(entries_bytes: BYTE_ARRAY) -> BOOLEAN:
    // Calculate CRC32 of entries array
    calculated = crc32(entries_bytes)
    
    RETURN calculated == this.partition_entries_crc32
END FUNCTION
```

### GPT Parsing

**Code Reference:** `crates/totalimage-zones/src/gpt/mod.rs:80-200`

```pseudocode
FUNCTION GptZoneTable.parse_with_config(
    stream: ReadSeek,
    sector_size: UINT32,
    config: GptConfig
) -> Result<GptZoneTable>:
    // GPT header is at LBA 1
    header_offset = GPT_HEADER_LBA * sector_size
    stream.seek(header_offset)
    
    // Read GPT header
    header_bytes = read_bytes(stream, sector_size)
    header = GptHeader::from_bytes(header_bytes)
    IF header IS NULL:
        RETURN Error::InvalidZoneTable("Invalid GPT header signature")
    
    // Verify header CRC32
    IF NOT header.verify_header_crc32(header_bytes):
        RETURN Error::ChecksumVerification("GPT header CRC32 verification failed")
    
    // Read partition entries
    entries_offset = header.partition_entries_lba * sector_size
    stream.seek(entries_offset)
    
    // Read all entries at once for CRC32 verification
    total_entries_size = header.num_partition_entries * header.partition_entry_size
    all_entries_bytes = read_bytes(stream, total_entries_size)
    
    // Verify entries CRC32
    IF NOT header.verify_partition_entries_crc32(all_entries_bytes):
        RETURN Error::ChecksumVerification("GPT partition entries CRC32 verification failed")
    
    // Parse individual entries
    zones = []
    FOR i = 0 TO header.num_partition_entries - 1:
        entry_start = i * header.partition_entry_size
        entry_bytes = all_entries_bytes[entry_start:entry_start + header.partition_entry_size]
        entry = GptPartitionEntry::from_bytes(entry_bytes)
        
        // Skip unused partitions
        IF entry.is_unused():
            CONTINUE
        
        // Calculate byte offsets
        zone_offset = entry.first_lba * sector_size
        zone_length = entry.size_lba() * sector_size
        
        // Use partition name if available
        zone_type = IF entry.name IS NOT EMPTY:
            format("{} ({})", entry.partition_type_guid.name(), entry.name)
        ELSE:
            entry.partition_type_guid.name()
        
        zone = Zone::new(
            index: i,
            offset: zone_offset,
            length: zone_length,
            zone_type: zone_type
        )
        zones.append(zone)
    END FOR
    
    // Read backup header if validation enabled
    backup_header = NULL
    IF config.validate_backup_header:
        backup_header = read_backup_header(stream, sector_size, header)
        IF backup_header IS NOT NULL:
            // Validate backup header matches primary
            IF NOT validate_backup_header(header, backup_header):
                RETURN Error::ChecksumVerification("Backup header mismatch")
    END IF
    
    RETURN GptZoneTable {
        zones: zones,
        header: header,
        backup_header: backup_header
    }
END FUNCTION

FUNCTION read_backup_header(
    stream: ReadSeek,
    sector_size: UINT32,
    primary_header: GptHeader
) -> OPTIONAL<GptHeader>:
    // Backup header is at last LBA
    backup_offset = primary_header.backup_lba * sector_size
    stream.seek(backup_offset)
    
    header_bytes = read_bytes(stream, sector_size)
    backup_header = GptHeader::from_bytes(header_bytes)
    
    IF backup_header IS NULL:
        RETURN NULL
    
    // Verify backup header CRC32
    IF NOT backup_header.verify_header_crc32(header_bytes):
        RETURN NULL
    
    RETURN backup_header
END FUNCTION
```

---

## Code References

### File Structure

```
crates/totalimage-zones/src/
├── lib.rs              # Module exports
├── mbr/
│   ├── mod.rs          # MBR parser (lines 1-494)
│   └── types.rs         # MBR types (lines 1-237)
└── gpt/
    ├── mod.rs          # GPT parser (lines 1-850)
    └── types.rs        # GPT types (lines 1-400+)
```

### Key Functions

#### MBR (`mbr/mod.rs`)
- `MbrZoneTable::parse`: `crates/totalimage-zones/src/mbr/mod.rs:68-136`
- `MbrZoneTable::is_gpt_protective`: `crates/totalimage-zones/src/mbr/mod.rs:152-154`
- `MbrZoneTable::serialize`: `crates/totalimage-zones/src/mbr/mod.rs:160-202`
- `MbrZoneTable::identify`: ZoneTable trait implementation
- `MbrZoneTable::enumerate_zones`: ZoneTable trait implementation

#### MBR Types (`mbr/types.rs`)
- `MbrPartitionType::from_byte`: `crates/totalimage-zones/src/mbr/types.rs:45-63`
- `MbrPartitionType::to_byte`: `crates/totalimage-zones/src/mbr/types.rs:66-84`
- `MbrPartitionType::name`: `crates/totalimage-zones/src/mbr/types.rs:87-105`
- `CHSAddress::from_bytes`: `crates/totalimage-zones/src/mbr/types.rs:132-144`
- `CHSAddress::to_bytes`: `crates/totalimage-zones/src/mbr/types.rs:147-152`
- `CHSAddress::from_lba`: `crates/totalimage-zones/src/mbr/types.rs:154-168`

#### GPT (`gpt/mod.rs`)
- `GptZoneTable::parse`: `crates/totalimage-zones/src/gpt/mod.rs:61-63`
- `GptZoneTable::parse_with_config`: `crates/totalimage-zones/src/gpt/mod.rs:80-200`
- `GptZoneTable::read_backup_header`: Backup header reading
- `GptZoneTable::validate_backup_header`: Backup header validation

#### GPT Types (`gpt/types.rs`)
- `GptHeader::from_bytes`: Header parsing
- `GptHeader::verify_header_crc32`: Header checksum verification
- `GptHeader::verify_partition_entries_crc32`: Entries checksum verification
- `GptPartitionEntry::from_bytes`: Entry parsing
- `GptPartitionEntry::is_unused`: Check if entry is unused

---

## Cross-References

- **Core Traits:** See [01-core.md](01-core.md#zonetable-trait)
- **Vault Usage:** See [02-vaults.md](02-vaults.md) (zones read from vault content)
- **Territory Parsing:** See [04-territories.md](04-territories.md) (territories read from zones)

---

**Document Version:** 1.0.0  
**Last Updated:** December 21, 2025  
**Next:** [04-territories.md](04-territories.md) - Filesystem Parsers
