# Acquire Module - Pseudocode Documentation

**Component:** `totalimage-acquire`  
**Location:** `crates/totalimage-acquire/src/`  
**Purpose:** Disk image acquisition and creation (E01 writer, WinPE USB)

---

## Table of Contents

1. [Overview](#overview)
2. [E01 Writer](#e01-writer)
3. [WinPE USB Creation](#winpe-usb-creation)
4. [Code References](#code-references)

---

## Overview

The acquire module provides disk image creation capabilities:
- **E01 Writer**: Create EnCase forensic images
- **WinPE USB**: Create bootable Windows PE USB drives
- **VHD Creation**: Create VHD disk images

**Code Reference:** `crates/totalimage-acquire/src/lib.rs`

---

## E01 Writer

EnCase forensic format writer.

**Code Reference:** `crates/totalimage-acquire/src/e01_writer.rs:78-697`

### E01 Writer Structure

```pseudocode
STRUCTURE E01Writer:
    output_path: PathBuf            // Base path for segments
    current_file: OPTIONAL<File>    // Current segment file
    current_segment: UINT16         // Segment number (1-based)
    config: E01WriterConfig        // Writer configuration
    chunk_table: ARRAY<(offset, size, compressed_size)>  // Chunk table
    current_chunk: BYTE_ARRAY       // Current chunk buffer
    current_chunk_index: UINT       // Current chunk index
    sectors_written: UINT64         // Total sectors written
    md5_hasher: Md5                 // MD5 hasher for uncompressed data
    file_position: UINT64           // Current file position
    volume_written: BOOLEAN         // Volume section written flag
END STRUCTURE

STRUCTURE E01WriterConfig:
    media_type: E01MediaType       // Media type
    bytes_per_sector: UINT32        // Sector size (usually 512)
    sectors_per_chunk: UINT32       // Sectors per chunk (usually 64)
    compression: UINT8              // Compression method (0=none, 1=deflate)
    max_segment_size: UINT64        // Max segment size (default: 2GB)
    case_info: OPTIONAL<STRING>     // Case metadata
    examiner: OPTIONAL<STRING>      // Examiner name
END STRUCTURE
```

### E01 Writing Process

**Code Reference:** `crates/totalimage-acquire/src/e01_writer.rs:103-697`

```pseudocode
FUNCTION E01Writer.new(output_path: Path, config: E01WriterConfig) -> Result<E01Writer>:
    writer = E01Writer {
        output_path: output_path,
        current_file: NULL,
        current_segment: 1,
        config: config,
        chunk_table: [],
        current_chunk: [],
        current_chunk_index: 0,
        sectors_written: 0,
        md5_hasher: Md5::new(),
        file_position: 0,
        volume_written: false
    }
    
    // Start first segment
    writer.start_segment()
    
    RETURN writer
END FUNCTION

FUNCTION E01Writer.start_segment() -> Result<VOID>:
    // Close previous segment if any
    IF this.current_file IS NOT NULL:
        this.finalize_segment(this.current_file)
    END IF
    
    // Create segment filename
    segment_path = IF this.current_segment == 1:
        this.output_path.with_extension("E01")
    ELSE:
        this.output_path.with_extension(format("E{:02}", this.current_segment))
    
    // Create file
    file = File::create(segment_path)
    
    // Write file header
    file_header = E01FileHeader {
        signature: EVF_SIGNATURE,
        segment_number: this.current_segment,
        fields_start: 13
    }
    write_bytes(file, file_header.serialize())
    
    // Reset chunk table for new segment
    this.chunk_table = []
    this.current_chunk_index = 0
    this.file_position = 13
    
    // Write header section (if first segment)
    IF this.current_segment == 1:
        this.write_header_section(file)
    END IF
    
    // Write volume section (if first segment)
    IF this.current_segment == 1:
        this.write_volume_section(file)
        this.volume_written = true
    END IF
    
    this.current_file = file
    RETURN success
END FUNCTION

FUNCTION E01Writer.write_data(source: ReadSeek, output: Write) -> Result<VOID>:
    buffer = allocate_buffer(this.config.sectors_per_chunk * this.config.bytes_per_sector)
    
    WHILE true:
        // Read chunk from source
        bytes_read = source.read(buffer)
        IF bytes_read == 0:
            BREAK  // EOF
        
        // Update MD5 hash
        this.md5_hasher.update(buffer[0:bytes_read])
        
        // Write chunk
        this.write_chunk(output, buffer[0:bytes_read])
        
        this.sectors_written = this.sectors_written + (bytes_read / this.config.bytes_per_sector)
    END WHILE
    
    RETURN success
END FUNCTION

FUNCTION E01Writer.write_chunk(output: Write, data: BYTE_ARRAY) -> Result<VOID>:
    // Check if we need a new segment
    IF this.file_position >= this.config.max_segment_size:
        this.current_segment = this.current_segment + 1
        this.start_segment()
        output = this.current_file
    END IF
    
    chunk_offset = this.file_position
    
    // Compress if enabled
    IF this.config.compression == 1:
        compressed_data = zlib_compress(data)
    ELSE:
        compressed_data = data
    END IF
    
    // Write compressed data
    write_bytes(output, compressed_data)
    
    // Record in chunk table
    this.chunk_table.append((
        chunk_offset,
        data.length,
        compressed_data.length
    ))
    
    this.file_position = this.file_position + compressed_data.length
    this.current_chunk_index = this.current_chunk_index + 1
    
    RETURN success
END FUNCTION

FUNCTION E01Writer.finalize(output: Write) -> Result<VOID>:
    // Flush any remaining chunk
    IF this.current_chunk IS NOT EMPTY:
        this.write_chunk(output, this.current_chunk)
    END IF
    
    // Write sectors section
    this.write_sectors_section(output)
    
    // Write table section (chunk offset table)
    this.write_table_section(output)
    
    // Write hash section
    md5_hash = this.md5_hasher.finalize()
    this.write_hash_section(output, md5_hash)
    
    // Write done section
    this.write_done_section(output)
    
    RETURN success
END FUNCTION
```

---

## WinPE USB Creation

Bootable Windows PE USB drive creation.

**Code Reference:** `crates/totalimage-acquire/src/winpe.rs`

```pseudocode
FUNCTION create_winpe_usb(
    usb_device: Path,
    winpe_source: Path,
    config: WinPeConfig
) -> Result<VOID>:
    // Step 1: Detect USB device
    IF NOT is_usb_device(usb_device):
        RETURN Error::InvalidOperation("Path is not a USB device")
    
    // Step 2: Create partition table
    IF config.partition_table == "MBR":
        create_mbr_partition_table(usb_device, config)
    ELSE IF config.partition_table == "GPT":
        create_gpt_partition_table(usb_device, config)
    END IF
    
    // Step 3: Format partition as FAT32
    format_fat32(usb_device, config)
    
    // Step 4: Detect WinPE source
    winpe_path = detect_winpe_source(winpe_source)
    IF winpe_path IS NULL:
        RETURN Error::NotFound("WinPE source not found")
    
    // Step 5: Extract WIM file
    wim_file = extract_wim_file(winpe_path)
    
    // Step 6: Copy boot files
    copy_boot_files(usb_device, winpe_path)
    
    // Step 7: Create boot configuration
    create_boot_config(usb_device, config)
    
    // Step 8: Inject drivers (if specified)
    IF config.drivers IS NOT EMPTY:
        inject_drivers(usb_device, config.drivers)
    END IF
    
    RETURN success
END FUNCTION
```

---

## Code References

### File Structure

```
crates/totalimage-acquire/src/
├── lib.rs              # Module exports
├── e01_writer.rs       # E01 writer (lines 1-697)
├── winpe.rs            # WinPE USB creation
└── error.rs            # Acquire-specific errors
```

### Key Functions

#### `e01_writer.rs`
- `E01Writer::new`: `crates/totalimage-acquire/src/e01_writer.rs:114-132`
- `E01Writer::start_segment`: Segment file creation
- `E01Writer::write_data`: Data writing with chunking
- `E01Writer::write_chunk`: Chunk compression and writing
- `E01Writer::finalize`: Finalize E01 file

---

## Cross-References

- **Vault Creation:** See [02-vaults.md](02-vaults.md) (E01 vault reading)
- **Zone Creation:** See [03-zones.md](03-zones.md) (MBR/GPT creation)
- **Territory Formatting:** See [04-territories.md](04-territories.md) (FAT32 formatting)

---

**Document Version:** 1.0.0  
**Last Updated:** December 21, 2025  
**Next:** [07-mcp-server.md](07-mcp-server.md) - MCP Server Implementation
