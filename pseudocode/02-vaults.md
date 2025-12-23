# Vaults Module - Pseudocode Documentation

**Component:** `totalimage-vaults`  
**Location:** `crates/totalimage-vaults/src/`  
**Purpose:** Container format implementations (Raw, VHD, E01, AFF4)

---

## Table of Contents

1. [Overview](#overview)
2. [Vault Factory](#vault-factory)
3. [Raw Vault](#raw-vault)
4. [VHD Vault](#vhd-vault)
5. [E01 Vault](#e01-vault)
6. [AFF4 Vault](#aff4-vault)
7. [Code References](#code-references)

---

## Overview

The vaults module provides implementations for various disk image container formats:
- **RawVault**: Direct sector images (.img, .ima, .flp, .vfd, .dsk, .iso)
- **VhdVault**: Microsoft VHD format (Fixed and Dynamic)
- **E01Vault**: EnCase forensic format
- **Aff4Vault**: Advanced Forensic Format 4

**Code Reference:** `crates/totalimage-vaults/src/lib.rs:1-36`

---

## Vault Factory

Auto-detection and vault opening.

**Code Reference:** `crates/totalimage-vaults/src/factory.rs`

```pseudocode
FUNCTION detect_vault_type(path: Path) -> Result<VaultType>:
    // Get file extension
    extension = path.extension().to_lowercase()
    
    // Check extension-based detection
    SWITCH extension:
        CASE ".vhd", ".vhdx":
            RETURN VaultType::Vhd
        CASE ".e01":
            RETURN VaultType::E01
        CASE ".aff4":
            RETURN VaultType::Aff4
        CASE ".img", ".ima", ".flp", ".vfd", ".dsk", ".iso":
            RETURN VaultType::Raw
        DEFAULT:
            // Try content-based detection
            RETURN detect_by_content(path)
    END SWITCH
END FUNCTION

FUNCTION detect_by_content(path: Path) -> Result<VaultType>:
    file = open_file(path)
    
    // Read first few bytes
    header = read_bytes(file, 0, 512)
    
    // Check VHD signature (footer at end)
    file_size = get_file_size(file)
    IF file_size >= 512:
        footer = read_bytes(file, file_size - 512, 512)
        IF footer[0:8] == "conectix":
            RETURN VaultType::Vhd
    
    // Check E01 signature
    IF header[0:13] == "EVF\x09\x0D\x0A\xFF\x00\x01\x00\x00\x00":
        RETURN VaultType::E01
    
    // Check AFF4 signature
    IF header[0:4] == "AFF4":
        RETURN VaultType::Aff4
    
    // Default to Raw
    RETURN VaultType::Raw
END FUNCTION

FUNCTION open_vault(path: Path, config: VaultConfig) -> Result<Box<dyn Vault>>:
    vault_type = detect_vault_type(path)
    
    SWITCH vault_type:
        CASE VaultType::Raw:
            RETURN RawVault::open(path, config)
        CASE VaultType::Vhd:
            RETURN VhdVault::open(path, config)
        CASE VaultType::E01:
            RETURN E01Vault::open(path)
        CASE VaultType::Aff4:
            RETURN Aff4Vault::open(path)
    END SWITCH
END FUNCTION
```

---

## Raw Vault

Direct sector image container.

**Code Reference:** `crates/totalimage-vaults/src/raw.rs:41-128`

```pseudocode
STRUCTURE RawVault:
    pipeline: Box<dyn ReadSeek>    // I/O pipeline
    length: UINT64                  // Vault size in bytes
END STRUCTURE

FUNCTION RawVault.open(path: Path, config: VaultConfig) -> Result<RawVault>:
    // Open file
    file = File::open(path)
    IF file FAILED:
        RETURN Error::Io(file_error)
    
    // Get file size
    length = file.metadata().len()
    
    // Create pipeline
    IF config.use_mmap:
        pipeline = MmapPipeline::from_file(file)
    ELSE:
        pipeline = file
    
    RETURN RawVault {
        pipeline: Box::new(pipeline),
        length: length
    }
END FUNCTION

FUNCTION RawVault.from_stream(stream: ReadSeek, length: UINT64) -> RawVault:
    RETURN RawVault {
        pipeline: Box::new(stream),
        length: length
    }
END FUNCTION

FUNCTION RawVault.manufacture(size: UINT64) -> RawVault:
    // Create blank in-memory vault
    buffer = allocate_zeroed_buffer(size)
    cursor = Cursor::new(buffer)
    
    RETURN RawVault {
        pipeline: Box::new(cursor),
        length: size
    }
END FUNCTION

// Vault trait implementation
FUNCTION RawVault.identify() -> STRING:
    RETURN "Raw sector image"
END FUNCTION

FUNCTION RawVault.length() -> UINT64:
    RETURN this.length
END FUNCTION

FUNCTION RawVault.content() -> MUTABLE<ReadSeek>:
    RETURN this.pipeline
END FUNCTION
```

---

## VHD Vault

Microsoft Virtual Hard Disk format.

**Code Reference:** `crates/totalimage-vaults/src/vhd/mod.rs:50-1322`

### VHD Footer Structure

**Code Reference:** `crates/totalimage-vaults/src/vhd/types.rs`

```pseudocode
STRUCTURE VhdFooter:
    cookie: BYTE_ARRAY[8]          // "conectix"
    features: UINT32               // Feature flags
    file_format_version: UINT32    // Format version
    data_offset: UINT64            // Offset to data (0xFFFFFFFF for fixed)
    timestamp: UINT32              // Creation timestamp
    creator_application: BYTE_ARRAY[4]  // Creator app
    creator_version: UINT32        // Creator version
    creator_host_os: UINT32        // Host OS
    original_size: UINT64          // Original disk size
    current_size: UINT64           // Current disk size
    disk_geometry: DiskGeometry     // CHS geometry
    disk_type: VhdType             // Fixed, Dynamic, Differencing
    checksum: UINT32               // Footer checksum
    unique_id: UUID                // Unique identifier
    saved_state: BOOLEAN           // Saved state flag
    reserved: BYTE_ARRAY[427]       // Reserved bytes
END STRUCTURE

FUNCTION VhdFooter.parse(bytes: BYTE_ARRAY[512]) -> Result<VhdFooter>:
    // Verify cookie
    IF bytes[0:8] != "conectix":
        RETURN Error::InvalidVault("Invalid VHD cookie")
    
    // Parse fields
    footer = VhdFooter {
        cookie: bytes[0:8],
        features: read_u32_le(bytes, 8),
        file_format_version: read_u32_le(bytes, 12),
        data_offset: read_u64_le(bytes, 16),
        timestamp: read_u32_le(bytes, 24),
        creator_application: bytes[28:32],
        creator_version: read_u32_le(bytes, 32),
        creator_host_os: read_u32_le(bytes, 36),
        original_size: read_u64_le(bytes, 40),
        current_size: read_u64_le(bytes, 48),
        disk_geometry: DiskGeometry::parse(bytes, 56),
        disk_type: VhdType::from_u32(read_u32_le(bytes, 60)),
        checksum: read_u32_le(bytes, 64),
        unique_id: UUID::parse(bytes, 68),
        saved_state: bytes[84] != 0,
        reserved: bytes[85:512]
    }
    
    // Verify checksum
    IF NOT footer.verify_checksum():
        RETURN Error::ChecksumVerification("VHD footer checksum invalid")
    
    RETURN footer
END FUNCTION

FUNCTION VhdFooter.verify_checksum() -> BOOLEAN:
    calculated = this.calculate_checksum()
    RETURN calculated == this.checksum
END FUNCTION

FUNCTION VhdFooter.calculate_checksum() -> UINT32:
    // Serialize footer with checksum field zeroed
    bytes = this.serialize()
    bytes[64:68] = [0, 0, 0, 0]  // Zero checksum field
    
    // Calculate one's complement sum
    sum = 0
    FOR EACH byte IN bytes:
        sum = sum + byte
    checksum = NOT sum  // One's complement
    
    RETURN checksum
END FUNCTION
```

### VHD Vault Implementation

```pseudocode
STRUCTURE VhdVault:
    pipeline: Box<dyn ReadSeek>
    footer: VhdFooter
    dynamic_header: OPTIONAL<VhdDynamicHeader>
    bat: OPTIONAL<BlockAllocationTable>
END STRUCTURE

FUNCTION VhdVault.open(path: Path, config: VaultConfig) -> Result<VhdVault>:
    // Open file
    file = File::open(path)
    file_size = file.metadata().len()
    
    // Validate minimum size
    IF file_size < 512:
        RETURN Error::InvalidVault("File too small to be a VHD")
    
    // Read footer from last 512 bytes
    file.seek(file_size - 512)
    footer_bytes = read_bytes(file, 512)
    footer = VhdFooter::parse(footer_bytes)
    
    // Verify checksum
    IF NOT footer.verify_checksum():
        RETURN Error::ChecksumVerification("VHD footer checksum failed")
    
    // Handle VHD type
    SWITCH footer.disk_type:
        CASE VhdType::Fixed:
            RETURN open_fixed_vhd(path, footer, config)
        
        CASE VhdType::Dynamic:
            RETURN open_dynamic_vhd(path, footer, config)
        
        CASE VhdType::Differencing:
            RETURN open_differencing_vhd(path, footer, config)
    END SWITCH
END FUNCTION

FUNCTION open_fixed_vhd(path: Path, footer: VhdFooter, config: VaultConfig) -> Result<VhdVault>:
    // Fixed VHD: data is everything except footer
    file = File::open(path)
    data_size = file.metadata().len() - 512
    
    // Create pipeline
    IF config.use_mmap:
        pipeline = MmapPipeline::from_file(file)
    ELSE:
        pipeline = file
    
    // Limit to data size
    pipeline = PartialPipeline::new(pipeline, 0, data_size)
    
    RETURN VhdVault {
        pipeline: Box::new(pipeline),
        footer: footer,
        dynamic_header: NULL,
        bat: NULL
    }
END FUNCTION

FUNCTION open_dynamic_vhd(path: Path, footer: VhdFooter, config: VaultConfig) -> Result<VhdVault>:
    file = File::open(path)
    
    // Read dynamic header (after footer)
    file.seek(512)
    header_bytes = read_bytes(file, 1024)
    dynamic_header = VhdDynamicHeader::parse(header_bytes)
    
    // Verify header checksum
    IF NOT dynamic_header.verify_checksum():
        RETURN Error::ChecksumVerification("VHD dynamic header checksum failed")
    
    // Read Block Allocation Table (BAT)
    bat_offset = dynamic_header.table_offset
    bat_size = dynamic_header.max_table_entries * 4  // 4 bytes per entry
    file.seek(bat_offset)
    bat_bytes = read_bytes(file, bat_size)
    bat = BlockAllocationTable::parse(bat_bytes, dynamic_header.max_table_entries)
    
    // Create virtual pipeline that handles block mapping
    pipeline = VhdDynamicPipeline::new(file, dynamic_header, bat)
    
    RETURN VhdVault {
        pipeline: Box::new(pipeline),
        footer: footer,
        dynamic_header: dynamic_header,
        bat: bat
    }
END FUNCTION

// Vault trait implementation
FUNCTION VhdVault.identify() -> STRING:
    IF this.dynamic_header IS NOT NULL:
        RETURN "Microsoft VHD (Dynamic)"
    ELSE:
        RETURN "Microsoft VHD (Fixed)"
END FUNCTION

FUNCTION VhdVault.length() -> UINT64:
    RETURN this.footer.current_size
END FUNCTION

FUNCTION VhdVault.content() -> MUTABLE<ReadSeek>:
    RETURN this.pipeline
END FUNCTION
```

---

## E01 Vault

EnCase forensic format.

**Code Reference:** `crates/totalimage-vaults/src/e01/mod.rs:44-600`

```pseudocode
STRUCTURE E01Vault:
    reader: Box<dyn ReadSeek>
    file_header: E01FileHeader
    volume: E01VolumeSection
    chunk_table: ARRAY<E01ChunkInfo>
    hash: OPTIONAL<E01HashSection>
    cache: E01Cache
    identifier: STRING
END STRUCTURE

FUNCTION E01Vault.open(path: Path) -> Result<E01Vault>:
    file = File::open(path)
    reader = Box::new(file)
    RETURN E01Vault::from_reader(reader)
END FUNCTION

FUNCTION E01Vault.from_reader(reader: Box<dyn ReadSeek>) -> Result<E01Vault>:
    // Parse file header (13 bytes)
    header_bytes = read_bytes(reader, 0, 13)
    file_header = E01FileHeader::parse(header_bytes)
    
    // Parse sections
    section_offset = file_header.fields_start
    volume = NULL
    chunk_table = []
    hash = NULL
    
    WHILE true:
        // Read section descriptor
        section_bytes = read_bytes(reader, section_offset, 16)
        section = E01SectionDescriptor::parse(section_bytes)
        
        SWITCH section.section_type:
            CASE SectionType::Volume, SectionType::Disk:
                data = read_bytes(reader, section_offset + 16, section.section_size - 16)
                volume = E01VolumeSection::parse(data)
            
            CASE SectionType::Table, SectionType::Table2:
                data = read_bytes(reader, section_offset + 16, section.section_size - 16)
                chunk_table = parse_chunk_table(data)
            
            CASE SectionType::Hash:
                data = read_bytes(reader, section_offset + 16, 20)
                hash = E01HashSection::parse(data)
            
            CASE SectionType::Done, SectionType::Next:
                BREAK
        END SWITCH
        
        IF section.next_offset == 0:
            BREAK
        section_offset = section.next_offset
    END WHILE
    
    IF volume IS NULL:
        RETURN Error::InvalidVault("E01 missing volume section")
    
    // Calculate total size
    total_size = volume.media_size()
    
    // Create cache
    cache = E01Cache::new(total_size)
    
    // Build identifier
    identifier = format("E01 {} {} sectors ({} bytes/sector)",
        E01MediaType::from(volume.media_type),
        volume.sector_count,
        volume.bytes_per_sector)
    
    RETURN E01Vault {
        reader: reader,
        file_header: file_header,
        volume: volume,
        chunk_table: chunk_table,
        hash: hash,
        cache: cache,
        identifier: identifier
    }
END FUNCTION

FUNCTION E01Vault.read_at(offset: UINT64, buffer: MUTABLE<BYTE_ARRAY>) -> Result<UINT>:
    IF offset >= this.cache.total_size:
        RETURN 0
    
    // Calculate chunk
    chunk_size = this.volume.chunk_size()
    chunk_index = offset / chunk_size
    chunk_offset = offset % chunk_size
    
    // Check cache
    IF this.cache.cached_chunk != chunk_index:
        // Decompress chunk
        this.cache.cached_data = this.decompress_chunk(chunk_index)
        this.cache.cached_chunk = chunk_index
    END IF
    
    // Copy from cache
    available = this.cache.cached_data.length - chunk_offset
    to_read = MIN(buffer.length, available)
    buffer[0:to_read] = this.cache.cached_data[chunk_offset:chunk_offset + to_read]
    
    RETURN to_read
END FUNCTION

FUNCTION E01Vault.decompress_chunk(chunk_index: UINT) -> Result<BYTE_ARRAY>:
    chunk_info = this.chunk_table[chunk_index]
    
    // Read compressed data
    this.reader.seek(chunk_info.offset)
    compressed_data = read_bytes(this.reader, chunk_info.compressed_size)
    
    // Decompress if needed
    IF chunk_info.is_compressed:
        decompressed = zlib_decompress(compressed_data)
    ELSE:
        decompressed = compressed_data
    
    RETURN decompressed
END FUNCTION

// Vault trait implementation
FUNCTION E01Vault.identify() -> STRING:
    RETURN this.identifier
END FUNCTION

FUNCTION E01Vault.length() -> UINT64:
    RETURN this.cache.total_size
END FUNCTION

FUNCTION E01Vault.content() -> MUTABLE<ReadSeek>:
    // Return virtual reader that wraps decompression
    RETURN this  // Implements Read + Seek
END FUNCTION
```

---

## AFF4 Vault

Advanced Forensic Format 4.

**Code Reference:** `crates/totalimage-vaults/src/aff4/mod.rs`

```pseudocode
STRUCTURE Aff4Vault:
    reader: Box<dyn ReadSeek>
    volume: Aff4Volume
    streams: ARRAY<Aff4Stream>
    identifier: STRING
END STRUCTURE

FUNCTION Aff4Vault.open(path: Path) -> Result<Aff4Vault>:
    // AFF4 is a ZIP-based format
    zip_archive = ZipArchive::open(path)
    
    // Parse AFF4 volume descriptor
    volume = parse_aff4_volume(zip_archive)
    
    // Enumerate streams
    streams = []
    FOR EACH entry IN zip_archive.entries:
        IF entry.name.starts_with("aff4://"):
            stream = parse_aff4_stream(entry)
            streams.append(stream)
    END FOR
    
    RETURN Aff4Vault {
        reader: Box::new(zip_archive),
        volume: volume,
        streams: streams,
        identifier: format("AFF4 {}", volume.version)
    }
END FUNCTION
```

---

## Code References

### File Structure

```
crates/totalimage-vaults/src/
├── lib.rs              # Module exports (lines 1-36)
├── factory.rs          # Vault factory (auto-detection)
├── raw.rs              # Raw vault (lines 1-227)
├── vhd/
│   ├── mod.rs          # VHD vault (lines 1-1322)
│   └── types.rs        # VHD types (lines 1-500+)
├── e01/
│   ├── mod.rs          # E01 vault (lines 1-600+)
│   └── types.rs        # E01 types
└── aff4/
    ├── mod.rs          # AFF4 vault
    └── types.rs        # AFF4 types
```

### Key Functions by File

#### `factory.rs`
- `detect_vault_type`: Auto-detection logic
- `open_vault`: Unified vault opening
- `supported_formats`: List supported formats

#### `raw.rs`
- `RawVault::open`: `crates/totalimage-vaults/src/raw.rs:57-70`
- `RawVault::from_stream`: `crates/totalimage-vaults/src/raw.rs:78-83`
- `RawVault::manufacture`: `crates/totalimage-vaults/src/raw.rs:103-113`
- `RawVault::identify`: `crates/totalimage-vaults/src/raw.rs:117-119`
- `RawVault::length`: `crates/totalimage-vaults/src/raw.rs:121-123`
- `RawVault::content`: `crates/totalimage-vaults/src/raw.rs:125-127`

#### `vhd/mod.rs`
- `VhdVault::open`: `crates/totalimage-vaults/src/vhd/mod.rs:71-132`
- `VhdVault::identify`: VHD type identification
- `VhdVault::length`: Return current_size from footer
- Block mapping for dynamic VHDs

#### `e01/mod.rs`
- `E01Vault::open`: `crates/totalimage-vaults/src/e01/mod.rs:105-253`
- `E01Vault::from_reader`: Section parsing
- `E01Vault::read_at`: Chunk decompression
- `E01Vault::decompress_chunk`: zlib decompression

---

## Cross-References

- **Core Traits:** See [01-core.md](01-core.md#vault-trait)
- **Pipeline Usage:** See [05-pipeline.md](05-pipeline.md)
- **Zone Parsing:** See [03-zones.md](03-zones.md) (uses vault content)
- **Territory Parsing:** See [04-territories.md](04-territories.md) (uses zone content)

---

**Document Version:** 1.0.0  
**Last Updated:** December 21, 2025  
**Next:** [03-zones.md](03-zones.md) - Partition Table Parsers
