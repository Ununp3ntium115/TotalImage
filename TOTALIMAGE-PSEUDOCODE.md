# TotalImage Pseudocode Specification

**Version:** 1.0
**Target Platform:** PYRO
**Purpose:** Complete system specification enabling reconstruction in any programming language

---

## Table of Contents

1. [System Overview](#1-system-overview)
2. [Core Abstractions](#2-core-abstractions)
3. [Vault Implementations](#3-vault-implementations)
4. [Zone Implementations](#4-zone-implementations)
5. [Territory Implementations](#5-territory-implementations)
6. [Pipeline & I/O](#6-pipeline--io)
7. [Security Requirements](#7-security-requirements)
8. [Algorithms & Utilities](#8-algorithms--utilities)
9. [Command-Line Interface](#9-command-line-interface)
10. [Web API](#10-web-api)
11. [MCP Server](#11-mcp-server)

---

## 1. System Overview

### 1.1 Architecture Diagram

```
User Interface Layer
├── CLI (totalimage-cli)
├── Web API (totalimage-web)
└── MCP Server (totalimage-mcp)
         ↓
Processing Layer
├── Pipeline (streaming I/O)
└── Acquire (image creation)
         ↓
Analysis Layer
├── Vaults (container formats)
├── Zones (partition tables)
└── Territories (filesystems)
         ↓
Core Layer
├── Traits & Interfaces
├── Error Handling
└── Security Validation
```

### 1.2 Data Flow

```
INPUT: Forensic disk image file (VHD, E01, AFF4, raw)
  ↓
STEP 1: Vault Layer
  - Detect container format
  - Open vault (decompress, seek, read sectors)
  ↓
STEP 2: Zone Layer
  - Parse partition table (MBR or GPT)
  - Enumerate partitions (zones)
  ↓
STEP 3: Territory Layer
  - Parse filesystem (FAT, NTFS, exFAT, ISO)
  - List files and directories
  ↓
STEP 4: File Extraction
  - Locate file clusters/runs
  - Read file data
  - Handle compression/attributes
  ↓
OUTPUT: Extracted files, metadata, analysis results
```

### 1.3 Terminology

- **Vault**: Container format (VHD, E01, AFF4, Raw)
- **Zone**: Partition on disk (MBR/GPT partition)
- **Territory**: Filesystem (FAT32, NTFS, exFAT, ISO-9660)
- **Pipeline**: Streaming I/O abstraction
- **Acquire**: Image creation/acquisition

---

## 2. Core Abstractions

### 2.1 Vault Interface

```pseudocode
INTERFACE Vault:
    FUNCTION name() -> String
        PURPOSE: Return human-readable vault type name
        EXAMPLE: "Microsoft VHD", "EnCase E01"

    FUNCTION content() -> ReadSeekStream
        PURPOSE: Provide seekable byte stream to disk content
        RETURNS: Stream positioned at logical sector 0
        NOTES:
            - Stream must support seek() and read()
            - Vault handles decompression transparently

    FUNCTION sector_size() -> Integer
        PURPOSE: Return logical sector size in bytes
        DEFAULT: 512
        VALID_VALUES: 512, 1024, 2048, 4096

    FUNCTION total_size() -> Integer
        PURPOSE: Return total disk size in bytes
        NOTES: Logical size, not container file size
```

### 2.2 Zone Interface (Partition Table)

```pseudocode
INTERFACE ZoneTable:
    FUNCTION identify() -> String
        PURPOSE: Return partition table type
        EXAMPLE: "Master Boot Record", "GUID Partition Table"

    FUNCTION enumerate_zones() -> Array<Zone>
        PURPOSE: List all partitions
        RETURNS: Array of Zone structures

STRUCTURE Zone:
    index: Integer           // Partition number (0-based)
    offset: Integer          // Byte offset from disk start
    length: Integer          // Partition size in bytes
    zone_type: String        // Partition type (e.g., "FAT32", "NTFS")
```

### 2.3 Territory Interface (Filesystem)

```pseudocode
INTERFACE Territory:
    FUNCTION identify() -> String
        PURPOSE: Return filesystem type
        EXAMPLE: "FAT32", "NTFS", "ISO-9660"

    FUNCTION list_files(path: String) -> Array<FileEntry>
        PURPOSE: List files in directory
        PARAMETERS:
            path: Directory path (e.g., "/", "/Windows/System32")
        RETURNS: Array of file entries

    FUNCTION read_file(path: String) -> ByteArray
        PURPOSE: Read complete file contents
        PARAMETERS:
            path: Full file path
        RETURNS: File data as byte array
        ERRORS: FileNotFound, PermissionDenied, CorruptedData

    FUNCTION extract_file(path: String, output_path: String) -> Boolean
        PURPOSE: Extract file to output path
        RETURNS: True on success

STRUCTURE FileEntry:
    name: String             // Filename
    path: String             // Full path
    size: Integer            // File size in bytes
    is_directory: Boolean    // True if directory
    created: DateTime        // Creation timestamp
    modified: DateTime       // Modification timestamp
    attributes: Integer      // File attributes (hidden, system, etc.)
```

### 2.4 Error Handling

```pseudocode
ENUMERATION ErrorType:
    IO_ERROR                 // File system I/O failure
    PARSE_ERROR             // Invalid data structure
    CHECKSUM_ERROR          // CRC/hash verification failed
    UNSUPPORTED_FORMAT      // Format not recognized
    CORRUPTION             // Data corruption detected
    SECURITY_VIOLATION     // Security limit exceeded
    INVALID_OPERATION      // Operation not allowed

STRUCTURE Error:
    type: ErrorType
    message: String
    source_file: String      // Optional: file where error occurred
    source_line: Integer     // Optional: line number

FUNCTION handle_error(error: Error) -> Void:
    IF error.type == SECURITY_VIOLATION:
        LOG critical security event
        ABORT immediately
    ELSE IF error.type == CHECKSUM_ERROR:
        LOG warning
        CONTINUE with degraded mode
    ELSE:
        LOG error
        PROPAGATE to caller
```

---

## 3. Vault Implementations

### 3.1 Raw Vault (Uncompressed Disk Image)

```pseudocode
CLASS RawVault IMPLEMENTS Vault:
    FIELDS:
        file_path: String
        file_handle: FileHandle
        use_mmap: Boolean
        sector_size: Integer

    FUNCTION open(path: String, config: VaultConfig) -> RawVault:
        file_handle = open_file(path, READ_ONLY)

        IF config.use_mmap AND platform_supports_mmap():
            validate_file_for_mmap(file_handle)
            mmap_region = create_memory_map(file_handle)
            RETURN RawVault with memory-mapped I/O
        ELSE:
            RETURN RawVault with standard file I/O

    FUNCTION validate_file_for_mmap(file: FileHandle) -> Void:
        // SEC-004: Memory-mapped file validation
        file_type = get_file_type(file)
        IF file_type NOT IN [REGULAR_FILE, BLOCK_DEVICE]:
            THROW SecurityViolation("Cannot mmap special files")

        file_size = get_file_size(file)
        CONST MAX_MMAP_SIZE = 16 * 1024 * 1024 * 1024  // 16 GB
        IF file_size > MAX_MMAP_SIZE:
            THROW SecurityViolation("File too large for mmap")

    FUNCTION content() -> ReadSeekStream:
        IF using memory-mapped I/O:
            RETURN MmapStream(mmap_region)
        ELSE:
            RETURN FileStream(file_handle)

    FUNCTION sector_size() -> Integer:
        RETURN this.sector_size  // Default: 512

    FUNCTION total_size() -> Integer:
        RETURN get_file_size(file_handle)
```

### 3.2 VHD Vault (Microsoft Virtual Hard Disk)

```pseudocode
CLASS VhdVault IMPLEMENTS Vault:
    FIELDS:
        file_path: String
        file_handle: FileHandle
        footer: VhdFooter
        header: VhdHeader          // For dynamic/differencing VHDs
        bat: BlockAllocationTable  // For dynamic VHDs
        disk_type: VhdDiskType     // FIXED, DYNAMIC, DIFFERENCING
        sector_size: Integer

    FUNCTION open(path: String, config: VaultConfig) -> VhdVault:
        file = open_file(path, READ_ONLY)

        // Read VHD footer (last 512 bytes)
        footer = read_vhd_footer(file)

        IF footer.disk_type == FIXED:
            // Fixed VHD: data is contiguous, footer at end
            RETURN VhdVault with simple file I/O

        ELSE IF footer.disk_type == DYNAMIC OR footer.disk_type == DIFFERENCING:
            // Dynamic VHD: read header and BAT
            header = read_vhd_header(file, footer.data_offset)
            bat = read_block_allocation_table(file, header)
            RETURN VhdVault with BAT-based block reading

        ELSE:
            THROW UnsupportedFormat("Unknown VHD disk type")

    FUNCTION read_vhd_footer(file: FileHandle) -> VhdFooter:
        file_size = get_file_size(file)
        seek(file, file_size - 512)
        footer_bytes = read_bytes(file, 512)

        // Parse footer structure
        footer = VhdFooter()
        footer.cookie = footer_bytes[0:8]
        IF footer.cookie != "conectix":
            THROW ParseError("Invalid VHD signature")

        footer.features = read_u32_be(footer_bytes, 8)
        footer.file_format_version = read_u32_be(footer_bytes, 12)
        footer.data_offset = read_u64_be(footer_bytes, 16)
        footer.timestamp = read_u32_be(footer_bytes, 24)
        footer.creator_app = footer_bytes[28:32]
        footer.creator_version = read_u32_be(footer_bytes, 32)
        footer.creator_host_os = footer_bytes[36:40]
        footer.original_size = read_u64_be(footer_bytes, 40)
        footer.current_size = read_u64_be(footer_bytes, 48)
        footer.disk_geometry = read_disk_geometry(footer_bytes, 56)
        footer.disk_type = read_u32_be(footer_bytes, 60)
        footer.checksum = read_u32_be(footer_bytes, 64)
        footer.unique_id = footer_bytes[68:84]
        footer.saved_state = footer_bytes[84]

        // Verify checksum
        calculated_checksum = calculate_vhd_checksum(footer_bytes)
        IF calculated_checksum != footer.checksum:
            THROW ChecksumError("VHD footer checksum mismatch")

        RETURN footer

    FUNCTION read_block(block_number: Integer) -> ByteArray:
        // For dynamic VHD: look up block in BAT
        IF disk_type == DYNAMIC OR disk_type == DIFFERENCING:
            bat_entry = bat[block_number]

            IF bat_entry == 0xFFFFFFFF:
                // Unallocated block: return zeros
                RETURN zero_filled_block(header.block_size)

            // Read block from file
            block_offset = bat_entry * sector_size
            seek(file_handle, block_offset)

            // Block has sector bitmap (512 bytes) + data
            bitmap = read_bytes(file_handle, 512)
            data = read_bytes(file_handle, header.block_size)

            RETURN data

        ELSE IF disk_type == FIXED:
            // Fixed VHD: simple linear read
            block_offset = block_number * block_size
            seek(file_handle, block_offset)
            RETURN read_bytes(file_handle, block_size)

    FUNCTION content() -> ReadSeekStream:
        RETURN VhdStream(this)

    CLASS VhdStream IMPLEMENTS ReadSeekStream:
        vault: VhdVault
        position: Integer

        FUNCTION seek(offset: Integer) -> Void:
            this.position = offset

        FUNCTION read(buffer: ByteArray, length: Integer) -> Integer:
            bytes_read = 0
            WHILE bytes_read < length:
                block_num = position / vault.header.block_size
                block_offset = position % vault.header.block_size

                block_data = vault.read_block(block_num)
                bytes_to_copy = min(length - bytes_read,
                                    block_data.length - block_offset)

                copy_bytes(block_data, block_offset,
                          buffer, bytes_read, bytes_to_copy)

                bytes_read += bytes_to_copy
                position += bytes_to_copy

            RETURN bytes_read
```

### 3.3 E01 Vault (EnCase Expert Witness Format)

```pseudocode
CLASS E01Vault IMPLEMENTS Vault:
    FIELDS:
        segments: Array<E01Segment>
        header: E01Header
        volume: E01Volume
        sectors_section: E01SectorsSection
        chunk_table: Array<ChunkEntry>
        sector_size: Integer

    STRUCTURE ChunkEntry:
        offset: Integer          // File offset to compressed chunk
        size: Integer           // Compressed size
        uncompressed_size: Integer

    FUNCTION open(path: String, config: VaultConfig) -> E01Vault:
        // E01 files can be segmented (.E01, .E02, etc.)
        segments = discover_segments(path)

        // Read file header from first segment
        first_segment = open_file(segments[0], READ_ONLY)
        file_header = read_e01_file_header(first_segment)

        // Parse sections
        header = read_header_section(first_segment)
        volume = read_volume_section(first_segment)

        // Build chunk table from table section
        chunk_table = build_chunk_table(segments)

        RETURN E01Vault(segments, header, volume, chunk_table)

    FUNCTION discover_segments(base_path: String) -> Array<String>:
        // E01 naming: image.E01, image.E02, ..., image.E99, image.EAA, ...
        segments = []
        base = remove_extension(base_path)

        // Check .E01 through .E99
        FOR i FROM 1 TO 99:
            segment_path = base + ".E" + zero_pad(i, 2)
            IF file_exists(segment_path):
                segments.append(segment_path)
            ELSE:
                BREAK

        // Check .EAA, .EAB, etc. if .E99 exists
        IF segments.length == 99:
            FOR i FROM 0 TO 675:  // 26*26 = 676 combinations
                letter1 = 'A' + (i / 26)
                letter2 = 'A' + (i % 26)
                segment_path = base + ".E" + letter1 + letter2
                IF file_exists(segment_path):
                    segments.append(segment_path)
                ELSE:
                    BREAK

        RETURN segments

    FUNCTION read_e01_file_header(file: FileHandle) -> E01FileHeader:
        seek(file, 0)
        header_bytes = read_bytes(file, 13)

        signature = header_bytes[0:8]
        IF signature != "EVF\x09\x0D\x0A\xFF\x00":
            THROW ParseError("Invalid E01 signature")

        fields_start = header_bytes[8]
        segment_number = read_u16_le(header_bytes, 9)
        fields_end = read_u16_le(header_bytes, 11)

        RETURN E01FileHeader(signature, segment_number)

    FUNCTION build_chunk_table(segments: Array<FileHandle>) -> Array<ChunkEntry>:
        chunk_table = []

        FOR EACH segment IN segments:
            // Scan for table section (type "table")
            table_section = find_section(segment, "table")
            IF table_section == NULL:
                CONTINUE

            // Parse chunk entries
            num_entries = table_section.size / 4  // Each entry is 4 bytes
            FOR i FROM 0 TO num_entries - 1:
                entry_offset = table_section.offset + (i * 4)
                entry_value = read_u32_le(segment, entry_offset)

                // Entry is offset from start of sectors section
                chunk_offset = entry_value
                chunk_table.append(ChunkEntry(chunk_offset, 0, 0))

        RETURN chunk_table

    FUNCTION read_chunk(chunk_index: Integer) -> ByteArray:
        chunk = chunk_table[chunk_index]

        // Find which segment contains this chunk
        segment_file = find_segment_for_offset(chunk.offset)

        // Read compressed chunk
        seek(segment_file, chunk.offset)
        compressed_data = read_bytes(segment_file, chunk.size)

        // Decompress using zlib
        uncompressed_data = zlib_decompress(compressed_data)

        RETURN uncompressed_data

    FUNCTION content() -> ReadSeekStream:
        RETURN E01Stream(this)
```

### 3.4 AFF4 Vault (Advanced Forensic Format 4)

```pseudocode
CLASS AFF4Vault IMPLEMENTS Vault:
    FIELDS:
        container: ZipArchive
        metadata: TurtleMetadata
        image_stream_urn: String
        bevies: Array<Bevy>
        compression_method: CompressionMethod
        sector_size: Integer

    ENUMERATION CompressionMethod:
        STORED      // No compression
        DEFLATE     // zlib/deflate
        SNAPPY      // Snappy compression
        LZ4         // LZ4 compression

    FUNCTION open(path: String, config: VaultConfig) -> AFF4Vault:
        // AFF4 is a ZIP container
        container = open_zip(path)

        // Read RDF metadata (Turtle format)
        metadata = read_turtle_metadata(container, "information.turtle")

        // Find image stream URN
        image_stream_urn = find_image_stream_urn(metadata)

        // Parse bevies (compressed segments)
        bevies = parse_bevies(container, image_stream_urn, metadata)

        // Determine compression method
        compression_method = get_compression_method(metadata, image_stream_urn)

        RETURN AFF4Vault(container, metadata, image_stream_urn, bevies, compression_method)

    FUNCTION read_turtle_metadata(zip: ZipArchive, filename: String) -> TurtleMetadata:
        turtle_data = zip.read_file(filename)
        metadata = parse_turtle_rdf(turtle_data)
        RETURN metadata

    FUNCTION parse_bevies(zip: ZipArchive, stream_urn: String, metadata: TurtleMetadata) -> Array<Bevy>:
        bevies = []
        bevy_size = get_property(metadata, stream_urn, "blockSize")
        chunk_size = get_property(metadata, stream_urn, "chunkSize")

        // Enumerate bevy files in ZIP
        bevy_index = 0
        WHILE TRUE:
            bevy_filename = format_bevy_filename(stream_urn, bevy_index)
            IF NOT zip.contains(bevy_filename):
                BREAK

            bevy_data = zip.read_file(bevy_filename)
            bevy = Bevy(bevy_index, bevy_data, bevy_size, chunk_size)
            bevies.append(bevy)
            bevy_index += 1

        RETURN bevies

    FUNCTION read_chunk(chunk_index: Integer, bevy_size: Integer) -> ByteArray:
        bevy_index = chunk_index / chunks_per_bevy
        chunk_in_bevy = chunk_index % chunks_per_bevy

        bevy = bevies[bevy_index]
        compressed_chunk = bevy.get_chunk(chunk_in_bevy)

        // Decompress based on method
        IF compression_method == SNAPPY:
            uncompressed = snappy_decompress(compressed_chunk)
        ELSE IF compression_method == LZ4:
            uncompressed = lz4_decompress(compressed_chunk)
        ELSE IF compression_method == DEFLATE:
            uncompressed = zlib_decompress(compressed_chunk)
        ELSE:
            uncompressed = compressed_chunk  // STORED

        RETURN uncompressed

    FUNCTION content() -> ReadSeekStream:
        RETURN AFF4Stream(this)
```

---

## 4. Zone Implementations

### 4.1 MBR (Master Boot Record)

```pseudocode
CLASS MbrZoneTable IMPLEMENTS ZoneTable:
    FIELDS:
        zones: Array<Zone>
        boot_code: ByteArray[446]
        partition_entries: Array<MbrPartitionEntry>[4]
        boot_signature: Integer

    STRUCTURE MbrPartitionEntry:
        boot_flag: Integer           // 0x80 = bootable, 0x00 = not bootable
        chs_start: CHS              // CHS address of first sector
        partition_type: Integer      // 0x0C = FAT32, 0x07 = NTFS, etc.
        chs_end: CHS                // CHS address of last sector
        lba_start: Integer          // LBA of first sector
        lba_length: Integer         // Number of sectors

    STRUCTURE CHS:
        cylinder: Integer           // 10 bits
        head: Integer               // 8 bits
        sector: Integer             // 6 bits

    FUNCTION parse(stream: ReadSeekStream, sector_size: Integer) -> MbrZoneTable:
        seek(stream, 0)
        mbr_bytes = read_bytes(stream, 512)

        // Verify boot signature (0xAA55 at offset 510-511)
        boot_sig = read_u16_le(mbr_bytes, 510)
        IF boot_sig != 0xAA55:
            THROW ParseError("Invalid MBR boot signature")

        // Read boot code (first 446 bytes)
        boot_code = mbr_bytes[0:446]

        // Parse 4 partition entries (16 bytes each, starting at offset 446)
        partition_entries = []
        zones = []

        FOR i FROM 0 TO 3:
            entry_offset = 446 + (i * 16)
            entry = parse_partition_entry(mbr_bytes, entry_offset)
            partition_entries.append(entry)

            // Skip unused entries (type 0x00)
            IF entry.partition_type == 0x00:
                CONTINUE

            // Create Zone from entry
            zone_offset = entry.lba_start * sector_size
            zone_length = entry.lba_length * sector_size
            zone_type = get_partition_type_name(entry.partition_type)

            zone = Zone(i, zone_offset, zone_length, zone_type)
            zones.append(zone)

        RETURN MbrZoneTable(zones, boot_code, partition_entries, boot_sig)

    FUNCTION parse_partition_entry(mbr: ByteArray, offset: Integer) -> MbrPartitionEntry:
        entry = MbrPartitionEntry()
        entry.boot_flag = mbr[offset]
        entry.chs_start = parse_chs(mbr, offset + 1)
        entry.partition_type = mbr[offset + 4]
        entry.chs_end = parse_chs(mbr, offset + 5)
        entry.lba_start = read_u32_le(mbr, offset + 8)
        entry.lba_length = read_u32_le(mbr, offset + 12)

        RETURN entry

    FUNCTION parse_chs(data: ByteArray, offset: Integer) -> CHS:
        head = data[offset]
        sector_cylinder = read_u16_be(data, offset + 1)

        sector = sector_cylinder & 0x3F          // Lower 6 bits
        cylinder = (sector_cylinder >> 6) & 0x3FF // Upper 10 bits

        RETURN CHS(cylinder, head, sector)

    FUNCTION get_partition_type_name(type_code: Integer) -> String:
        CONST TYPE_NAMES = {
            0x01: "FAT12",
            0x04: "FAT16 (< 32 MB)",
            0x06: "FAT16",
            0x07: "NTFS",
            0x0B: "FAT32",
            0x0C: "FAT32 LBA",
            0x0E: "FAT16 LBA",
            0x0F: "Extended (LBA)",
            0x82: "Linux Swap",
            0x83: "Linux Filesystem",
            0xEE: "GPT Protective"
        }

        IF type_code IN TYPE_NAMES:
            RETURN TYPE_NAMES[type_code]
        ELSE:
            RETURN "Unknown (" + hex(type_code) + ")"
```

### 4.2 GPT (GUID Partition Table)

```pseudocode
CLASS GptZoneTable IMPLEMENTS ZoneTable:
    FIELDS:
        zones: Array<Zone>
        header: GptHeader
        backup_header: GptHeader  // Optional
        partition_entries: Array<GptPartitionEntry>

    STRUCTURE GptHeader:
        signature: ByteArray[8]          // "EFI PART"
        revision: Integer                // 0x00010000 for GPT 1.0
        header_size: Integer             // 92 bytes
        header_crc32: Integer
        reserved: Integer                // Must be 0
        current_lba: Integer             // LBA of this header
        backup_lba: Integer              // LBA of backup header
        first_usable_lba: Integer        // First usable block
        last_usable_lba: Integer         // Last usable block
        disk_guid: GUID                  // Unique disk identifier
        partition_entries_lba: Integer   // LBA of partition entries
        num_partition_entries: Integer   // Typically 128
        partition_entry_size: Integer    // Typically 128 bytes
        partition_entries_crc32: Integer

    STRUCTURE GptPartitionEntry:
        partition_type_guid: GUID        // Partition type
        unique_partition_guid: GUID      // Unique partition ID
        first_lba: Integer               // First block of partition
        last_lba: Integer                // Last block of partition
        attributes: Integer              // Attribute flags
        partition_name: String           // UTF-16LE, max 36 chars

    FUNCTION parse(stream: ReadSeekStream, sector_size: Integer) -> GptZoneTable:
        // GPT header is at LBA 1 (sector 1)
        seek(stream, sector_size)
        header_bytes = read_bytes(stream, sector_size)

        header = parse_gpt_header(header_bytes)

        // Verify header CRC32
        IF NOT verify_gpt_header_crc32(header_bytes):
            THROW ChecksumError("GPT header CRC32 verification failed")

        // Read partition entries
        entries_lba = header.partition_entries_lba
        entries_offset = entries_lba * sector_size
        seek(stream, entries_offset)

        total_entries_size = header.num_partition_entries * header.partition_entry_size
        entries_bytes = read_bytes(stream, total_entries_size)

        // Verify partition entries CRC32
        calculated_crc = crc32(entries_bytes)
        IF calculated_crc != header.partition_entries_crc32:
            THROW ChecksumError("GPT partition entries CRC32 verification failed")

        // Parse individual partition entries
        partition_entries = []
        zones = []

        FOR i FROM 0 TO header.num_partition_entries - 1:
            entry_start = i * header.partition_entry_size
            entry_end = entry_start + header.partition_entry_size
            entry_bytes = entries_bytes[entry_start:entry_end]

            entry = parse_gpt_partition_entry(entry_bytes)
            partition_entries.append(entry)

            // Skip unused partitions (all zeros in type GUID)
            IF entry.partition_type_guid.is_zero():
                CONTINUE

            // Create Zone from entry
            zone_offset = entry.first_lba * sector_size
            zone_length = (entry.last_lba - entry.first_lba + 1) * sector_size
            zone_type = get_partition_type_name(entry.partition_type_guid) +
                       " (" + entry.partition_name + ")"

            zone = Zone(i, zone_offset, zone_length, zone_type)
            zones.append(zone)

        // Optionally read and validate backup header
        backup_header = read_backup_header(stream, header, sector_size)

        RETURN GptZoneTable(zones, header, backup_header, partition_entries)

    FUNCTION parse_gpt_header(bytes: ByteArray) -> GptHeader:
        header = GptHeader()
        header.signature = bytes[0:8]

        IF header.signature != "EFI PART":
            THROW ParseError("Invalid GPT signature")

        header.revision = read_u32_le(bytes, 8)
        header.header_size = read_u32_le(bytes, 12)
        header.header_crc32 = read_u32_le(bytes, 16)
        header.reserved = read_u32_le(bytes, 20)
        header.current_lba = read_u64_le(bytes, 24)
        header.backup_lba = read_u64_le(bytes, 32)
        header.first_usable_lba = read_u64_le(bytes, 40)
        header.last_usable_lba = read_u64_le(bytes, 48)
        header.disk_guid = parse_guid(bytes, 56)
        header.partition_entries_lba = read_u64_le(bytes, 72)
        header.num_partition_entries = read_u32_le(bytes, 80)
        header.partition_entry_size = read_u32_le(bytes, 84)
        header.partition_entries_crc32 = read_u32_le(bytes, 88)

        RETURN header

    FUNCTION verify_gpt_header_crc32(bytes: ByteArray) -> Boolean:
        stored_crc = read_u32_le(bytes, 16)

        // Zero out CRC field for calculation
        temp_bytes = copy(bytes)
        write_u32_le(temp_bytes, 16, 0)

        calculated_crc = crc32(temp_bytes[0:92])  // Header is 92 bytes
        RETURN calculated_crc == stored_crc

    FUNCTION read_backup_header(stream: ReadSeekStream, primary_header: GptHeader, sector_size: Integer) -> GptHeader:
        backup_lba = primary_header.backup_lba
        backup_offset = backup_lba * sector_size

        seek(stream, backup_offset)
        backup_bytes = read_bytes(stream, sector_size)

        backup_header = parse_gpt_header(backup_bytes)

        // Verify backup header CRC32
        IF NOT verify_gpt_header_crc32(backup_bytes):
            THROW ChecksumError("GPT backup header CRC32 verification failed")

        // Validate backup matches primary
        validate_backup_header(primary_header, backup_header)

        RETURN backup_header

    FUNCTION validate_backup_header(primary: GptHeader, backup: GptHeader) -> Void:
        // Compare critical fields
        IF primary.disk_guid != backup.disk_guid:
            THROW ParseError("Backup header disk GUID mismatch")

        IF primary.partition_entries_lba != backup.partition_entries_lba:
            LOG warning "Backup header partition entries LBA mismatch"

        // Note: current_lba and backup_lba should be swapped
        IF primary.current_lba != backup.backup_lba:
            LOG warning "Header LBA swap mismatch"
```

---

## 5. Territory Implementations

### 5.1 FAT32 Filesystem

```pseudocode
CLASS FatTerritory IMPLEMENTS Territory:
    FIELDS:
        stream: ReadSeekStream
        bpb: BiosParameterBlock
        fat_type: FatType  // FAT12, FAT16, or FAT32
        fat_table: Array<Integer>
        root_directory_offset: Integer

    ENUMERATION FatType:
        FAT12
        FAT16
        FAT32

    STRUCTURE BiosParameterBlock:
        bytes_per_sector: Integer
        sectors_per_cluster: Integer
        reserved_sectors: Integer
        num_fats: Integer
        root_entries: Integer
        total_sectors: Integer
        media_descriptor: Integer
        sectors_per_fat: Integer
        // FAT32 specific:
        fat32_sectors_per_fat: Integer
        root_cluster: Integer
        fsinfo_sector: Integer
        backup_boot_sector: Integer

    FUNCTION parse(stream: ReadSeekStream, offset: Integer, length: Integer) -> FatTerritory:
        seek(stream, offset)
        boot_sector = read_bytes(stream, 512)

        // Parse BIOS Parameter Block
        bpb = parse_bpb(boot_sector)

        // Determine FAT type
        fat_type = determine_fat_type(bpb)

        // Read FAT table
        fat_offset = offset + (bpb.reserved_sectors * bpb.bytes_per_sector)
        fat_table = read_fat_table(stream, fat_offset, bpb, fat_type)

        // Calculate root directory offset
        IF fat_type == FAT32:
            root_directory_offset = cluster_to_offset(bpb.root_cluster, bpb)
        ELSE:
            // FAT12/FAT16: root dir after FATs
            root_directory_offset = fat_offset +
                (bpb.num_fats * bpb.sectors_per_fat * bpb.bytes_per_sector)

        RETURN FatTerritory(stream, bpb, fat_type, fat_table, root_directory_offset)

    FUNCTION parse_bpb(boot_sector: ByteArray) -> BiosParameterBlock:
        bpb = BiosParameterBlock()

        // Common BPB fields (offsets 11-35)
        bpb.bytes_per_sector = read_u16_le(boot_sector, 11)
        bpb.sectors_per_cluster = boot_sector[13]
        bpb.reserved_sectors = read_u16_le(boot_sector, 14)
        bpb.num_fats = boot_sector[16]
        bpb.root_entries = read_u16_le(boot_sector, 17)

        total_sectors_16 = read_u16_le(boot_sector, 19)
        bpb.media_descriptor = boot_sector[21]
        bpb.sectors_per_fat = read_u16_le(boot_sector, 22)

        // Total sectors (use 32-bit field if 16-bit is 0)
        IF total_sectors_16 == 0:
            bpb.total_sectors = read_u32_le(boot_sector, 32)
        ELSE:
            bpb.total_sectors = total_sectors_16

        // FAT32 extended BPB (if sectors_per_fat == 0)
        IF bpb.sectors_per_fat == 0:
            bpb.fat32_sectors_per_fat = read_u32_le(boot_sector, 36)
            bpb.root_cluster = read_u32_le(boot_sector, 44)
            bpb.fsinfo_sector = read_u16_le(boot_sector, 48)
            bpb.backup_boot_sector = read_u16_le(boot_sector, 50)

        RETURN bpb

    FUNCTION determine_fat_type(bpb: BiosParameterBlock) -> FatType:
        // Calculate data region size
        root_dir_sectors = ((bpb.root_entries * 32) +
                           (bpb.bytes_per_sector - 1)) / bpb.bytes_per_sector

        fat_size = IF bpb.sectors_per_fat != 0 THEN bpb.sectors_per_fat
                   ELSE bpb.fat32_sectors_per_fat

        data_sectors = bpb.total_sectors -
                      (bpb.reserved_sectors +
                       (bpb.num_fats * fat_size) +
                       root_dir_sectors)

        cluster_count = data_sectors / bpb.sectors_per_cluster

        // Determine type by cluster count
        IF cluster_count < 4085:
            RETURN FAT12
        ELSE IF cluster_count < 65525:
            RETURN FAT16
        ELSE:
            RETURN FAT32

    FUNCTION read_fat_table(stream: ReadSeekStream, fat_offset: Integer, bpb: BiosParameterBlock, fat_type: FatType) -> Array<Integer>:
        fat_size = IF bpb.sectors_per_fat != 0 THEN bpb.sectors_per_fat
                   ELSE bpb.fat32_sectors_per_fat
        fat_bytes = fat_size * bpb.bytes_per_sector

        // SEC-008: Validate FAT size to prevent overflow
        CONST MAX_FAT_ALLOCATION = 100 * 1024 * 1024  // 100 MB
        IF fat_bytes > MAX_FAT_ALLOCATION:
            THROW SecurityViolation("FAT table too large")

        seek(stream, fat_offset)
        fat_data = read_bytes(stream, fat_bytes)

        // Parse FAT entries based on type
        fat_table = []

        IF fat_type == FAT12:
            // 12-bit entries (1.5 bytes each)
            FOR i FROM 0 TO (fat_bytes * 8 / 12) - 1:
                byte_offset = (i * 3) / 2
                IF i % 2 == 0:
                    // Even entry: lower 12 bits of 2 bytes
                    entry = read_u16_le(fat_data, byte_offset) & 0x0FFF
                ELSE:
                    // Odd entry: upper 12 bits of 2 bytes
                    entry = read_u16_le(fat_data, byte_offset) >> 4
                fat_table.append(entry)

        ELSE IF fat_type == FAT16:
            // 16-bit entries (2 bytes each)
            FOR i FROM 0 TO (fat_bytes / 2) - 1:
                entry = read_u16_le(fat_data, i * 2)
                fat_table.append(entry)

        ELSE:  // FAT32
            // 32-bit entries (4 bytes each, lower 28 bits used)
            FOR i FROM 0 TO (fat_bytes / 4) - 1:
                entry = read_u32_le(fat_data, i * 4) & 0x0FFFFFFF
                fat_table.append(entry)

        RETURN fat_table

    FUNCTION list_files(path: String) -> Array<FileEntry>:
        // Resolve path to directory cluster
        directory_cluster = resolve_path_to_cluster(path)

        // Read directory entries
        entries = read_directory_entries(directory_cluster)

        RETURN entries

    FUNCTION read_directory_entries(cluster: Integer) -> Array<FileEntry>:
        entries = []
        current_cluster = cluster

        WHILE current_cluster < get_eoc_marker():
            // Read cluster data
            cluster_data = read_cluster(current_cluster)

            // Parse directory entries (32 bytes each)
            FOR offset FROM 0 TO cluster_data.length - 32 STEP 32:
                entry_bytes = cluster_data[offset:offset+32]

                // Skip deleted entries (first byte = 0xE5)
                IF entry_bytes[0] == 0xE5:
                    CONTINUE

                // End of directory (first byte = 0x00)
                IF entry_bytes[0] == 0x00:
                    RETURN entries

                // Parse directory entry
                entry = parse_directory_entry(entry_bytes)
                entries.append(entry)

            // Follow cluster chain
            current_cluster = fat_table[current_cluster]

        RETURN entries

    FUNCTION parse_directory_entry(bytes: ByteArray) -> FileEntry:
        entry = FileEntry()

        // Filename (8.3 format)
        name = bytes[0:8].trim()
        extension = bytes[8:11].trim()
        entry.name = IF extension.is_empty() THEN name ELSE name + "." + extension

        // Attributes
        entry.attributes = bytes[11]
        entry.is_directory = (entry.attributes & 0x10) != 0

        // Timestamps
        entry.created = parse_fat_datetime(bytes, 14, 13)
        entry.modified = parse_fat_datetime(bytes, 22, 24)

        // Cluster and size
        cluster_high = read_u16_le(bytes, 20)
        cluster_low = read_u16_le(bytes, 26)
        entry.first_cluster = (cluster_high << 16) | cluster_low
        entry.size = read_u32_le(bytes, 28)

        RETURN entry

    FUNCTION read_file(path: String) -> ByteArray:
        // Resolve path to file entry
        file_entry = resolve_path_to_file(path)

        // Read file clusters
        data = []
        current_cluster = file_entry.first_cluster
        bytes_remaining = file_entry.size

        WHILE current_cluster < get_eoc_marker() AND bytes_remaining > 0:
            cluster_data = read_cluster(current_cluster)
            bytes_to_copy = min(cluster_data.length, bytes_remaining)
            data.append(cluster_data[0:bytes_to_copy])

            bytes_remaining -= bytes_to_copy
            current_cluster = fat_table[current_cluster]

        RETURN concatenate(data)

    FUNCTION read_cluster(cluster_number: Integer) -> ByteArray:
        cluster_offset = cluster_to_offset(cluster_number, bpb)
        cluster_size = bpb.sectors_per_cluster * bpb.bytes_per_sector

        seek(stream, cluster_offset)
        RETURN read_bytes(stream, cluster_size)

    FUNCTION cluster_to_offset(cluster: Integer, bpb: BiosParameterBlock) -> Integer:
        // First cluster is cluster 2 (clusters 0 and 1 are reserved)
        data_start = bpb.reserved_sectors +
                     (bpb.num_fats * fat_size) +
                     root_dir_sectors

        cluster_offset = data_start * bpb.bytes_per_sector +
                        ((cluster - 2) * bpb.sectors_per_cluster * bpb.bytes_per_sector)

        RETURN cluster_offset

    FUNCTION get_eoc_marker() -> Integer:
        IF fat_type == FAT12:
            RETURN 0x0FF8  // End of chain marker for FAT12
        ELSE IF fat_type == FAT16:
            RETURN 0xFFF8  // End of chain marker for FAT16
        ELSE:
            RETURN 0x0FFFFFF8  // End of chain marker for FAT32
```

*(Continuing in next message due to length...)*

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Sonnet 4.5 (1M context) <noreply@anthropic.com>

### 5.2 NTFS Filesystem (Simplified)

```pseudocode
CLASS NtfsTerritory IMPLEMENTS Territory:
    FIELDS:
        stream: ReadSeekStream
        boot_sector: NtfsBootSector
        mft: MasterFileTable
        cluster_size: Integer

    STRUCTURE NtfsBootSector:
        bytes_per_sector: Integer
        sectors_per_cluster: Integer
        mft_cluster: Integer
        mft_mirror_cluster: Integer
        file_record_size: Integer
        index_buffer_size: Integer

    FUNCTION parse(stream: ReadSeekStream, offset: Integer, length: Integer) -> NtfsTerritory:
        seek(stream, offset)
        boot_sector_bytes = read_bytes(stream, 512)
        
        boot_sector = parse_ntfs_boot_sector(boot_sector_bytes)
        cluster_size = boot_sector.sectors_per_cluster * boot_sector.bytes_per_sector
        
        // Read Master File Table
        mft_offset = offset + (boot_sector.mft_cluster * cluster_size)
        mft = read_mft(stream, mft_offset, boot_sector.file_record_size)
        
        RETURN NtfsTerritory(stream, boot_sector, mft, cluster_size)

    FUNCTION read_mft(stream: ReadSeekStream, mft_offset: Integer, record_size: Integer) -> MasterFileTable:
        // MFT is itself a file (file record 0)
        // For simplicity, cache first N entries
        CONST INITIAL_MFT_ENTRIES = 100
        
        mft_entries = []
        FOR i FROM 0 TO INITIAL_MFT_ENTRIES - 1:
            entry_offset = mft_offset + (i * record_size)
            seek(stream, entry_offset)
            entry_bytes = read_bytes(stream, record_size)
            entry = parse_mft_entry(entry_bytes)
            mft_entries.append(entry)
        
        RETURN MasterFileTable(mft_entries)

NOTE: Full NTFS implementation is complex. This pseudocode shows basic structure.
      For production, consider NTFS-3G or similar library integration.
```

### 5.3 ISO-9660 Filesystem with Joliet

```pseudocode
CLASS IsoTerritory IMPLEMENTS Territory:
    FIELDS:
        stream: ReadSeekStream
        primary_descriptor: PrimaryVolumeDescriptor
        supplementary_descriptor: SupplementaryVolumeDescriptor  // Joliet
        is_joliet: Boolean
        sector_size: Integer

    STRUCTURE PrimaryVolumeDescriptor:
        volume_identifier: String
        volume_space_size: Integer
        logical_block_size: Integer
        path_table_size: Integer
        path_table_lba: Integer
        root_directory_record: DirectoryRecord

    FUNCTION parse(stream: ReadSeekStream, offset: Integer, length: Integer) -> IsoTerritory:
        sector_size = 2048  // ISO sectors are always 2048 bytes
        
        // Read Volume Descriptor Set starting at sector 16
        seek(stream, offset + (16 * sector_size))
        
        primary_descriptor = NULL
        supplementary_descriptor = NULL
        
        WHILE TRUE:
            descriptor_bytes = read_bytes(stream, sector_size)
            descriptor_type = descriptor_bytes[0]
            
            IF descriptor_type == 1:  // Primary Volume Descriptor
                primary_descriptor = parse_primary_volume_descriptor(descriptor_bytes)
            ELSE IF descriptor_type == 2:  // Supplementary Volume Descriptor (Joliet)
                supp_desc = parse_supplementary_volume_descriptor(descriptor_bytes)
                IF supp_desc.is_joliet():
                    supplementary_descriptor = supp_desc
            ELSE IF descriptor_type == 255:  // Volume Descriptor Set Terminator
                BREAK
        
        is_joliet = (supplementary_descriptor != NULL)
        
        RETURN IsoTerritory(stream, primary_descriptor, supplementary_descriptor, is_joliet, sector_size)

    FUNCTION is_joliet_descriptor(supp_desc: SupplementaryVolumeDescriptor) -> Boolean:
        // Joliet escape sequences at offset 88-90
        escape_seq = supp_desc.volume_identifier[0:3]
        RETURN escape_seq IN ["%/@", "%/C", "%/E"]  // Joliet levels 1, 2, 3

    FUNCTION list_files(path: String) -> Array<FileEntry>:
        // Use Joliet descriptor if available, otherwise primary
        root_record = IF is_joliet THEN supplementary_descriptor.root_directory_record
                      ELSE primary_descriptor.root_directory_record
        
        directory_record = resolve_path_to_directory(path, root_record)
        entries = read_directory_records(directory_record)
        
        RETURN entries

    FUNCTION read_directory_records(dir_record: DirectoryRecord) -> Array<FileEntry>:
        extent_lba = dir_record.extent_location
        extent_length = dir_record.data_length
        
        seek(stream, extent_lba * sector_size)
        directory_data = read_bytes(stream, extent_length)
        
        entries = []
        offset = 0
        
        WHILE offset < extent_length:
            record_length = directory_data[offset]
            IF record_length == 0:
                BREAK
            
            record_bytes = directory_data[offset:offset + record_length]
            entry = parse_directory_record(record_bytes)
            
            // Decode filename (Joliet uses UTF-16BE)
            entry.name = IF is_joliet THEN decode_utf16be(entry.file_identifier)
                        ELSE decode_ascii(entry.file_identifier)
            
            entries.append(entry)
            offset += record_length
        
        RETURN entries

    FUNCTION decode_utf16be(data: ByteArray) -> String:
        // Joliet filenames are UTF-16BE (UCS-2)
        IF data.length % 2 != 0:
            RETURN ""  // Invalid UTF-16
        
        utf16_chars = []
        FOR i FROM 0 TO data.length - 2 STEP 2:
            codepoint = (data[i] << 8) | data[i + 1]
            IF codepoint == 0 OR codepoint == 0x003B:  // Null or ';' (version separator)
                BREAK
            utf16_chars.append(codepoint)
        
        RETURN utf16_decode(utf16_chars)
```

---

## 6. Pipeline & I/O

### 6.1 PartialPipeline (Windowed I/O)

```pseudocode
CLASS PartialPipeline IMPLEMENTS ReadSeekStream:
    PURPOSE: Provide window into subset of larger stream
    
    FIELDS:
        inner_stream: ReadSeekStream
        window_offset: Integer
        window_length: Integer
        current_position: Integer

    FUNCTION new(stream: ReadSeekStream, offset: Integer, length: Integer) -> PartialPipeline:
        RETURN PartialPipeline(stream, offset, length, 0)

    FUNCTION seek(offset: Integer) -> Void:
        IF offset < 0 OR offset > window_length:
            THROW IOError("Seek outside window bounds")
        
        current_position = offset

    FUNCTION read(buffer: ByteArray, length: Integer) -> Integer:
        bytes_available = window_length - current_position
        bytes_to_read = min(length, bytes_available)
        
        IF bytes_to_read <= 0:
            RETURN 0  // EOF
        
        // Seek to absolute position in inner stream
        absolute_position = window_offset + current_position
        inner_stream.seek(absolute_position)
        
        bytes_read = inner_stream.read(buffer, bytes_to_read)
        current_position += bytes_read
        
        RETURN bytes_read

    FUNCTION length() -> Integer:
        RETURN window_length
```

### 6.2 Memory-Mapped I/O

```pseudocode
CLASS MmapStream IMPLEMENTS ReadSeekStream:
    PURPOSE: Memory-mapped file I/O for performance
    
    FIELDS:
        mmap_region: MemoryMappedRegion
        length: Integer
        current_position: Integer

    FUNCTION new(file_path: String) -> MmapStream:
        // SEC-004: Validate file before mmapping
        validate_file_for_mmap(file_path)
        
        mmap_region = create_mmap(file_path, READ_ONLY)
        length = mmap_region.length()
        
        RETURN MmapStream(mmap_region, length, 0)

    FUNCTION validate_file_for_mmap(file_path: String) -> Void:
        file_info = get_file_info(file_path)
        
        // Only allow regular files and block devices
        IF file_info.type NOT IN [REGULAR_FILE, BLOCK_DEVICE]:
            THROW SecurityViolation("Cannot mmap special files")
        
        // Enforce size limit
        CONST MAX_MMAP_SIZE = 16 * 1024 * 1024 * 1024  // 16 GB
        IF file_info.size > MAX_MMAP_SIZE:
            THROW SecurityViolation("File too large for mmap: " + file_info.size)

    FUNCTION read(buffer: ByteArray, length: Integer) -> Integer:
        bytes_available = this.length - current_position
        bytes_to_read = min(length, bytes_available)
        
        IF bytes_to_read <= 0:
            RETURN 0
        
        // Direct memory copy from mmap region
        memory_copy(mmap_region.data + current_position, buffer, bytes_to_read)
        current_position += bytes_to_read
        
        RETURN bytes_to_read
```

---

## 7. Security Requirements

### 7.1 Input Validation

```pseudocode
MODULE Security:

    CONSTANTS:
        MAX_SECTOR_SIZE = 4096
        MAX_ALLOCATION_SIZE = 256 * 1024 * 1024      // 256 MB
        MAX_FAT_ALLOCATION = 100 * 1024 * 1024       // 100 MB
        MAX_EXTRACTION_SIZE = 1024 * 1024 * 1024     // 1 GB
        MAX_PATH_LENGTH = 4096
        MAX_FILENAME_LENGTH = 255

    FUNCTION validate_sector_size(size: Integer) -> Void:
        IF size NOT IN [512, 1024, 2048, 4096]:
            THROW SecurityViolation("Invalid sector size: " + size)

    FUNCTION validate_allocation_size(size: Integer) -> Void:
        IF size > MAX_ALLOCATION_SIZE:
            THROW SecurityViolation("Allocation too large: " + size)
        
        IF size < 0:
            THROW SecurityViolation("Negative allocation size")

    FUNCTION validate_file_path(path: String) -> Void:
        IF path.length > MAX_PATH_LENGTH:
            THROW SecurityViolation("Path too long")
        
        // Prevent directory traversal
        IF path.contains(".."):
            THROW SecurityViolation("Path traversal detected")
        
        // Prevent absolute paths
        IF path.starts_with("/") OR path.starts_with("\\"):
            THROW SecurityViolation("Absolute paths not allowed")

    FUNCTION validate_extraction_size(size: Integer) -> Void:
        IF size > MAX_EXTRACTION_SIZE:
            THROW SecurityViolation("File too large to extract: " + size)
```

### 7.2 Checked Arithmetic

```pseudocode
FUNCTION checked_add(a: Integer, b: Integer) -> Integer:
    // SEC-008: Prevent integer overflow
    IF a > 0 AND b > INTEGER_MAX - a:
        THROW ArithmeticOverflow("Addition overflow")
    IF a < 0 AND b < INTEGER_MIN - a:
        THROW ArithmeticOverflow("Addition underflow")
    
    RETURN a + b

FUNCTION checked_mul(a: Integer, b: Integer) -> Integer:
    // SEC-008: Prevent integer overflow
    IF a == 0 OR b == 0:
        RETURN 0
    
    IF a > INTEGER_MAX / b:
        THROW ArithmeticOverflow("Multiplication overflow")
    
    RETURN a * b

FUNCTION saturating_add(a: Integer, b: Integer) -> Integer:
    // Returns INTEGER_MAX/MIN on overflow instead of throwing
    result = a + b
    IF a > 0 AND b > 0 AND result < a:
        RETURN INTEGER_MAX
    IF a < 0 AND b < 0 AND result > a:
        RETURN INTEGER_MIN
    RETURN result
```

### 7.3 Timeout & Iteration Limits

```pseudocode
FUNCTION read_vhd_dynamic_block_with_timeout(block_num: Integer) -> ByteArray:
    // SEC-011: Prevent infinite loops in VHD chain traversal
    CONST MAX_VHD_CHAIN_DEPTH = 100
    
    chain_depth = 0
    current_vhd = this
    
    WHILE current_vhd.has_parent():
        chain_depth += 1
        
        IF chain_depth > MAX_VHD_CHAIN_DEPTH:
            THROW SecurityViolation("VHD chain too deep: " + chain_depth)
        
        // Check if block exists in current differencing disk
        IF current_vhd.has_block(block_num):
            RETURN current_vhd.read_block(block_num)
        
        current_vhd = current_vhd.parent
    
    THROW IOError("Block not found in VHD chain")
```

---

## 8. Algorithms & Utilities

### 8.1 CRC32 Calculation

```pseudocode
FUNCTION crc32(data: ByteArray) -> Integer:
    // IEEE CRC32 polynomial: 0xEDB88320
    CONST CRC32_POLYNOMIAL = 0xEDB88320
    
    crc = 0xFFFFFFFF
    
    FOR EACH byte IN data:
        crc = crc XOR byte
        
        FOR bit FROM 0 TO 7:
            IF (crc AND 1) != 0:
                crc = (crc >> 1) XOR CRC32_POLYNOMIAL
            ELSE:
                crc = crc >> 1
    
    RETURN NOT crc
```

### 8.2 Hash Calculation (MD5, SHA1, SHA256)

```pseudocode
FUNCTION hash_file(file_path: String, algorithms: Array<HashAlgorithm>) -> Array<HashResult>:
    file = open_file(file_path, READ_ONLY)
    file_size = get_file_size(file)
    
    // Initialize hashers
    hashers = []
    FOR EACH algorithm IN algorithms:
        IF algorithm == MD5:
            hashers.append(Md5Hasher())
        ELSE IF algorithm == SHA1:
            hashers.append(Sha1Hasher())
        ELSE IF algorithm == SHA256:
            hashers.append(Sha256Hasher())
    
    // Read file in chunks and update hashers
    CONST CHUNK_SIZE = 1024 * 1024  // 1 MB
    bytes_processed = 0
    
    WHILE bytes_processed < file_size:
        chunk = read_bytes(file, CHUNK_SIZE)
        
        FOR EACH hasher IN hashers:
            hasher.update(chunk)
        
        bytes_processed += chunk.length
        report_progress(bytes_processed, file_size)
    
    // Finalize and return results
    results = []
    FOR i FROM 0 TO hashers.length - 1:
        digest = hashers[i].finalize()
        results.append(HashResult(algorithms[i], digest))
    
    RETURN results
```

### 8.3 Compression/Decompression

```pseudocode
FUNCTION zlib_decompress(compressed_data: ByteArray) -> ByteArray:
    // zlib/deflate decompression (RFC 1950/1951)
    decompressor = create_zlib_decompressor()
    uncompressed = decompressor.decompress(compressed_data)
    RETURN uncompressed

FUNCTION snappy_decompress(compressed_data: ByteArray) -> ByteArray:
    // Snappy compression (used in AFF4)
    decompressor = create_snappy_decompressor()
    uncompressed = decompressor.decompress(compressed_data)
    RETURN uncompressed

FUNCTION lz4_decompress(compressed_data: ByteArray) -> ByteArray:
    // LZ4 compression (used in AFF4)
    decompressor = create_lz4_decompressor()
    uncompressed = decompressor.decompress(compressed_data)
    RETURN uncompressed
```

---

## 9. Command-Line Interface

### 9.1 CLI Commands

```pseudocode
PROGRAM TotalImageCLI:

    COMMAND info <image_file>:
        PURPOSE: Display vault information
        
        vault = open_vault(image_file)
        PRINT "Vault Type: " + vault.name()
        PRINT "Total Size: " + format_bytes(vault.total_size())
        PRINT "Sector Size: " + vault.sector_size()

    COMMAND zones <image_file>:
        PURPOSE: List partitions
        
        vault = open_vault(image_file)
        zone_table = parse_zone_table(vault)
        
        PRINT "Partition Table: " + zone_table.identify()
        PRINT ""
        PRINT "Index  Offset         Length         Type"
        PRINT "-----  -------------  -------------  ----"
        
        FOR EACH zone IN zone_table.enumerate_zones():
            PRINT format("{:5}  {:13}  {:13}  {}",
                        zone.index,
                        format_bytes(zone.offset),
                        format_bytes(zone.length),
                        zone.zone_type)

    COMMAND list <image_file> [--zone INDEX]:
        PURPOSE: List files in partition
        
        vault = open_vault(image_file)
        zone_table = parse_zone_table(vault)
        zone = zone_table.enumerate_zones()[zone_index]
        
        territory = parse_territory(vault, zone)
        files = territory.list_files("/")
        
        PRINT "Files in " + zone.zone_type + ":"
        FOR EACH file IN files:
            PRINT format("{:10}  {}  {}",
                        format_bytes(file.size),
                        file.modified,
                        file.name)

    COMMAND extract <image_file> <file_path> [--zone INDEX] [--output PATH]:
        PURPOSE: Extract file from image
        
        vault = open_vault(image_file)
        zone_table = parse_zone_table(vault)
        zone = zone_table.enumerate_zones()[zone_index]
        
        territory = parse_territory(vault, zone)
        territory.extract_file(file_path, output_path)
        
        PRINT "Extracted: " + file_path + " -> " + output_path

    COMMAND hash <file> [--algorithm <md5|sha1|sha256>] [--format <hex|base64>]:
        PURPOSE: Calculate file hash
        
        algorithm = parse_algorithm_arg()  // Default: SHA256
        results = hash_file(file, [algorithm])
        
        PRINT "Algorithm: " + algorithm
        PRINT "Hash: " + format_hash(results[0].digest, format)

    COMMAND create-winpe-usb <device> [--winpe-source PATH]:
        PURPOSE: Create bootable WinPE USB drive
        
        // Detect USB drives
        usb_drives = detect_usb_drives()
        target_drive = find_drive_by_path(usb_drives, device)
        
        // Validate WinPE source
        winpe_source = IF winpe_source_arg THEN validate_winpe_source(winpe_source_arg)
                      ELSE find_winpe_source()
        
        // Create partition table (GPT for UEFI)
        partition_builder = PartitionTableBuilder(GPT, 512)
        (partition_offset, partition_length) = partition_builder.create_gpt(target_drive, size)
        
        // Format as FAT32
        formatter = Fat32Formatter(512, 8, "WINPE")
        formatter.format(target_drive, partition_offset, partition_length)
        
        // Extract WinPE (requires external WIM tools)
        extract_wim_to_usb(winpe_source.boot_wim_path, mount_point)
        
        PRINT "WinPE USB created successfully"
```

---

## 10. Web API

### 10.1 REST API Endpoints

```pseudocode
WEB_API TotalImageWeb:
    BASE_URL: http://localhost:8080/api/v1

    ENDPOINT GET /vaults:
        PURPOSE: List available disk images
        RETURNS: Array<VaultSummary>

    ENDPOINT POST /vaults:
        PURPOSE: Open new disk image
        BODY: { "path": "/path/to/image.vhd" }
        RETURNS: VaultHandle

    ENDPOINT GET /vaults/{id}/zones:
        PURPOSE: List partitions in vault
        RETURNS: Array<Zone>

    ENDPOINT GET /vaults/{id}/zones/{zone_id}/files:
        PURPOSE: List files in partition
        QUERY_PARAMS: path=/path/to/dir
        RETURNS: Array<FileEntry>

    ENDPOINT GET /vaults/{id}/zones/{zone_id}/files/download:
        PURPOSE: Extract and download file
        QUERY_PARAMS: path=/path/to/file.txt
        RETURNS: Binary file data

    ENDPOINT POST /vaults/{id}/hash:
        PURPOSE: Calculate hash of vault
        BODY: { "algorithms": ["sha256"] }
        RETURNS: { "hashes": { "sha256": "abc123..." } }

IMPLEMENTATION:
    CLASS WebServer:
        FUNCTION handle_list_files(vault_id, zone_id, path):
            vault = get_vault_from_cache(vault_id)
            zone = get_zone(vault, zone_id)
            territory = parse_territory(vault, zone)
            
            // SEC-007: Rate limiting
            check_rate_limit(client_ip)
            
            files = territory.list_files(path)
            RETURN json_response(files)

        FUNCTION handle_download_file(vault_id, zone_id, file_path):
            // SEC-012: Path validation
            validate_file_path(file_path)
            
            vault = get_vault_from_cache(vault_id)
            zone = get_zone(vault, zone_id)
            territory = parse_territory(vault, zone)
            
            file_data = territory.read_file(file_path)
            
            // SEC-013: Size limit
            IF file_data.length > MAX_EXTRACTION_SIZE:
                RETURN error_response("File too large")
            
            RETURN binary_response(file_data)
```

---

## 11. MCP Server (Model Context Protocol)

### 11.1 MCP Tool Definitions

```pseudocode
MCP_SERVER TotalImageMCP:

    TOOL analyze_disk_image:
        PURPOSE: Analyze disk image and return metadata
        PARAMETERS:
            image_path: String
        RETURNS: JSON with vault info, partition table, filesystem types

    TOOL list_partitions:
        PURPOSE: List all partitions in disk image
        PARAMETERS:
            image_path: String
        RETURNS: JSON array of partitions

    TOOL list_files:
        PURPOSE: List files in specific partition
        PARAMETERS:
            image_path: String
            partition_index: Integer
            directory_path: String
        RETURNS: JSON array of files

    TOOL extract_file:
        PURPOSE: Extract file from disk image
        PARAMETERS:
            image_path: String
            partition_index: Integer
            file_path: String
            output_path: String
        RETURNS: Success/failure status

    TOOL validate_integrity:
        PURPOSE: Validate disk image integrity (checksums)
        PARAMETERS:
            image_path: String
        RETURNS: Validation results

IMPLEMENTATION:
    FUNCTION handle_mcp_request(tool_name, parameters):
        // Authenticate request
        verify_api_key(request.headers["X-API-Key"])
        
        IF tool_name == "analyze_disk_image":
            RETURN analyze_disk_image_impl(parameters.image_path)
        
        ELSE IF tool_name == "list_partitions":
            RETURN list_partitions_impl(parameters.image_path)
        
        ELSE IF tool_name == "list_files":
            RETURN list_files_impl(parameters.image_path,
                                  parameters.partition_index,
                                  parameters.directory_path)
        
        ELSE IF tool_name == "extract_file":
            RETURN extract_file_impl(parameters.image_path,
                                    parameters.partition_index,
                                    parameters.file_path,
                                    parameters.output_path)
        
        ELSE IF tool_name == "validate_integrity":
            RETURN validate_integrity_impl(parameters.image_path)
```

---

## 12. PYRO Platform Integration

### 12.1 Fire Marshal (Tool Orchestration)

```pseudocode
MODULE FireMarshal:
    PURPOSE: Tool registry and orchestration for PYRO platform

    CLASS ToolRegistry:
        FIELDS:
            tools: Map<String, ToolDefinition>

        FUNCTION register_tool(name: String, definition: ToolDefinition) -> Void:
            tools[name] = definition

        FUNCTION execute_tool(name: String, parameters: Map<String, Any>) -> ToolResult:
            IF name NOT IN tools:
                THROW ToolNotFound(name)
            
            tool = tools[name]
            result = tool.execute(parameters)
            
            RETURN result

    STRUCTURE ToolDefinition:
        name: String
        description: String
        parameters: Array<ParameterDefinition>
        executor: Function

    FUNCTION register_totalimage_tools() -> Void:
        registry = ToolRegistry()
        
        registry.register_tool("analyze_disk_image", ToolDefinition(
            name: "analyze_disk_image",
            description: "Analyze forensic disk image",
            parameters: [
                Parameter("image_path", STRING, required: true)
            ],
            executor: analyze_disk_image_handler
        ))
        
        // Register all 5 MCP tools...
```

### 12.2 PYRO Worker Integration

```pseudocode
CLASS PyroWorker:
    PURPOSE: BullMQ worker for async disk image processing

    FUNCTION process_job(job: Job) -> JobResult:
        job_type = job.type
        
        IF job_type == "analyze_image":
            image_path = job.data.image_path
            result = analyze_disk_image(image_path)
            RETURN JobResult(status: "completed", data: result)
        
        ELSE IF job_type == "extract_files":
            image_path = job.data.image_path
            file_list = job.data.files
            output_dir = job.data.output_dir
            
            FOR EACH file_path IN file_list:
                extract_file_from_image(image_path, file_path, output_dir)
            
            RETURN JobResult(status: "completed", files_extracted: file_list.length)

    FUNCTION analyze_disk_image(image_path: String) -> AnalysisResult:
        vault = open_vault(image_path)
        zone_table = parse_zone_table(vault)
        
        result = AnalysisResult()
        result.vault_type = vault.name()
        result.total_size = vault.total_size()
        result.partitions = []
        
        FOR EACH zone IN zone_table.enumerate_zones():
            partition = PartitionInfo()
            partition.index = zone.index
            partition.type = zone.zone_type
            partition.size = zone.length
            
            // Parse filesystem
            TRY:
                territory = parse_territory(vault, zone)
                partition.filesystem = territory.identify()
                partition.files_count = count_files(territory)
            CATCH error:
                partition.filesystem = "Unknown"
                partition.error = error.message
            
            result.partitions.append(partition)
        
        RETURN result
```

---

## 13. Testing & Validation

### 13.1 Property-Based Testing

```pseudocode
PROPERTY_TEST gpt_header_roundtrip:
    GENERATE random_gpt_header:
        partition_count = random_integer(0, 128)
        disk_size = random_integer(1_000_000_000, 1_000_000_000_000)
        sector_size = random_choice([512, 1024, 2048, 4096])
        
        header = create_gpt_header(partition_count, disk_size, sector_size)
    
    TEST:
        // Serialize to bytes
        bytes = header.serialize()
        
        // Parse back from bytes
        parsed = GptHeader.parse(bytes)
        
        // Verify all fields match
        ASSERT parsed.num_partition_entries == partition_count
        ASSERT parsed.disk_guid == header.disk_guid
        ASSERT parsed.verify_header_crc32(bytes)

PROPERTY_TEST fat_bpb_roundtrip:
    GENERATE random_fat_bpb:
        bytes_per_sector = random_choice([512, 1024, 2048, 4096])
        sectors_per_cluster = random_power_of_2(1, 128)
        
        bpb = create_fat_bpb(bytes_per_sector, sectors_per_cluster)
    
    TEST:
        bytes = bpb.serialize()
        parsed = BiosParameterBlock.parse(bytes)
        
        ASSERT parsed.bytes_per_sector == bytes_per_sector
        ASSERT parsed.sectors_per_cluster == sectors_per_cluster
```

### 13.2 Integration Tests

```pseudocode
INTEGRATION_TEST test_vhd_to_fat32_pipeline:
    // Generate synthetic VHD with FAT32
    vhd_data = create_vhd_with_mbr_fat32(100_MB)
    write_to_file(vhd_data, "/tmp/test.vhd")
    
    // Open vault
    vault = open_vault("/tmp/test.vhd")
    ASSERT vault.name() == "Microsoft VHD"
    
    // Parse partition table
    zone_table = parse_zone_table(vault)
    ASSERT zone_table.identify() == "Master Boot Record"
    ASSERT zone_table.enumerate_zones().length == 1
    
    zone = zone_table.enumerate_zones()[0]
    ASSERT zone.zone_type.contains("FAT32")
    
    // Parse filesystem
    territory = parse_territory(vault, zone)
    ASSERT territory.identify() == "FAT32"
    
    // List root directory
    files = territory.list_files("/")
    ASSERT files.length >= 0
```

---

## 14. Performance Considerations

### 14.1 Caching Strategy

```pseudocode
CLASS CacheManager:
    FIELDS:
        vault_cache: LRU_Cache<String, Vault>
        zone_cache: LRU_Cache<String, ZoneTable>
        territory_cache: LRU_Cache<String, Territory>

    CONSTANTS:
        MAX_VAULT_CACHE_SIZE = 10
        MAX_ZONE_CACHE_SIZE = 50
        MAX_TERRITORY_CACHE_SIZE = 20

    FUNCTION get_or_open_vault(path: String) -> Vault:
        IF path IN vault_cache:
            RETURN vault_cache[path]
        
        vault = open_vault(path)
        vault_cache.put(path, vault)
        RETURN vault

    FUNCTION evict_on_memory_pressure() -> Void:
        // SEC-008: Use saturating arithmetic for cache size
        current_memory = get_current_memory_usage()
        CONST MAX_CACHE_MEMORY = 256 * 1024 * 1024  // 256 MB
        
        IF current_memory > MAX_CACHE_MEMORY:
            vault_cache.evict_lru()
            territory_cache.evict_lru()
```

### 14.2 Streaming vs Buffering

```pseudocode
GUIDELINE: When to use streaming vs buffering

USE STREAMING WHEN:
    - File size > 100 MB
    - Processing sequential data
    - Limited memory available

USE BUFFERING WHEN:
    - File size < 10 MB
    - Random access required
    - Sufficient memory available

EXAMPLE streaming implementation:
    FUNCTION extract_large_file_streaming(territory, file_path, output_path):
        file_entry = territory.find_file(file_path)
        output_file = create_file(output_path)
        
        CONST BUFFER_SIZE = 1024 * 1024  // 1 MB chunks
        offset = 0
        
        WHILE offset < file_entry.size:
            chunk_size = min(BUFFER_SIZE, file_entry.size - offset)
            chunk = territory.read_file_chunk(file_path, offset, chunk_size)
            output_file.write(chunk)
            offset += chunk_size
        
        output_file.close()
```

---

## 15. Deployment Architecture

### 15.1 Kubernetes Deployment

```pseudocode
KUBERNETES_DEPLOYMENT TotalImageWeb:
    REPLICAS: 2-10 (with HPA)
    RESOURCES:
        CPU: 500m-2000m
        MEMORY: 1Gi-4Gi
    
    ENVIRONMENT:
        RUST_LOG: info
        TOTALIMAGE_CACHE_DIR: /cache
        MAX_UPLOAD_SIZE: 10GB
    
    VOLUMES:
        - name: cache-volume
          emptyDir:
            sizeLimit: 10Gi
        - name: image-storage
          persistentVolumeClaim:
            claimName: disk-images-pvc

    PROBES:
        LIVENESS: GET /health
        READINESS: GET /ready

KUBERNETES_DEPLOYMENT TotalImageMCP:
    REPLICAS: 2-8 (with HPA)
    RESOURCES:
        CPU: 250m-1000m
        MEMORY: 512Mi-2Gi

SERVICE_MESH:
    INGRESS: nginx
    TLS: cert-manager
    RATE_LIMITING: 100 requests/minute per IP
```

---

## 16. Future Extensions

### 16.1 Planned Features

```pseudocode
FUTURE_FEATURE exFAT_write_support:
    PURPOSE: Create exFAT filesystems
    COMPLEXITY: Medium
    DEPENDENCIES: exFAT specification (Microsoft)

FUTURE_FEATURE e01_multithreading:
    PURPOSE: Parallel chunk decompression
    COMPLEXITY: High
    PERFORMANCE_GAIN: 3-5x speedup

FUTURE_FEATURE aff4_encryption:
    PURPOSE: Support encrypted AFF4 images
    COMPLEXITY: High
    DEPENDENCIES: Cryptography library

FUTURE_FEATURE wim_extraction:
    PURPOSE: Extract WinPE WIM files natively
    COMPLEXITY: Very High
    NOTES: Currently requires external wimlib
    ALGORITHM:
        - Parse WIM header
        - Decompress LZX/XPRESS streams
        - Extract file metadata
        - Preserve security descriptors
```

---

## Appendix A: Data Structures Reference

### A.1 VHD Footer Structure (512 bytes)

```
Offset  Size  Field
------  ----  -----
0       8     Cookie ("conectix")
8       4     Features
12      4     File Format Version
16      8     Data Offset
24      4     Timestamp
28      4     Creator Application
32      4     Creator Version
36      4     Creator Host OS
40      8     Original Size
48      8     Current Size
56      4     Disk Geometry (CHS)
60      4     Disk Type
64      4     Checksum
68      16    Unique ID (UUID)
84      1     Saved State
85      427   Reserved
```

### A.2 GPT Header Structure (92 bytes)

```
Offset  Size  Field
------  ----  -----
0       8     Signature ("EFI PART")
8       4     Revision
12      4     Header Size (92)
16      4     Header CRC32
20      4     Reserved (0)
24      8     Current LBA
32      8     Backup LBA
40      8     First Usable LBA
48      8     Last Usable LBA
56      16    Disk GUID
72      8     Partition Entry Array LBA
80      4     Number of Partition Entries
84      4     Size of Partition Entry
88      4     Partition Entry Array CRC32
```

---

## Appendix B: File Format Specifications

- VHD: Microsoft Virtual Hard Disk Format Specification v1.0
- E01: EnCase Evidence File Format (proprietary, reverse-engineered)
- AFF4: Advanced Forensic Format 4 (https://github.com/aff4/Standard)
- ISO-9660: ECMA-119 Optical Disc Format
- FAT: Microsoft FAT Specification (FAT12/16/32)
- NTFS: New Technology File System (partial, based on observation)
- GPT: UEFI Specification (Partition Table)

---

**END OF TOTALIMAGE PSEUDOCODE SPECIFICATION v1.0**

This specification enables complete reconstruction of the TotalImage system
in any programming language for deployment on the PYRO platform.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Sonnet 4.5 (1M context) <noreply@anthropic.com>
