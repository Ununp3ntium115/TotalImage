# Pipeline Module - Pseudocode Documentation

**Component:** `totalimage-pipeline`  
**Location:** `crates/totalimage-pipeline/src/`  
**Purpose:** I/O abstractions for efficient data access

---

## Table of Contents

1. [Overview](#overview)
2. [PartialPipeline](#partialpipeline)
3. [MmapPipeline](#mmappipeline)
4. [Code References](#code-references)

---

## Overview

The pipeline module provides efficient I/O abstractions:
- **PartialPipeline**: Window into a subset of a stream (for partitions)
- **MmapPipeline**: Memory-mapped file access for direct action

**Code Reference:** `crates/totalimage-pipeline/src/lib.rs`

---

## PartialPipeline

Window into a subset of a stream.

**Code Reference:** `crates/totalimage-pipeline/src/partial.rs:22-232`

```pseudocode
STRUCTURE PartialPipeline<R: Read + Seek>:
    inner: R                        // Underlying stream
    start: UINT64                   // Start offset in stream
    length: UINT64                  // Length of window
    position: UINT64                // Current position in window
END STRUCTURE

FUNCTION PartialPipeline.new(inner: R, start: UINT64, length: UINT64) -> Result<PartialPipeline>:
    // Verify we can seek to start position
    inner.seek(SeekFrom::Start(start))
    IF FAILED:
        RETURN error
    
    RETURN PartialPipeline {
        inner: inner,
        start: start,
        length: length,
        position: 0
    }
END FUNCTION

FUNCTION PartialPipeline.read(buffer: MUTABLE<BYTE_ARRAY>) -> Result<UINT>:
    // Calculate remaining bytes
    remaining = this.length - this.position
    IF remaining == 0:
        RETURN 0  // EOF
    
    // Limit read to remaining bytes
    to_read = MIN(buffer.length, remaining)
    
    // Calculate absolute position in inner stream
    absolute_pos = this.start + this.position
    this.inner.seek(SeekFrom::Start(absolute_pos))
    
    // Read from inner stream
    bytes_read = this.inner.read(buffer[0:to_read])
    
    // Update position
    this.position = this.position + bytes_read
    
    RETURN bytes_read
END FUNCTION

FUNCTION PartialPipeline.seek(pos: SeekFrom) -> Result<UINT64>:
    new_pos = SWITCH pos:
        CASE SeekFrom::Start(offset):
            offset
        CASE SeekFrom::End(offset):
            this.length + offset
        CASE SeekFrom::Current(offset):
            this.position + offset
    END SWITCH
    
    // Validate bounds
    IF new_pos < 0:
        RETURN error("Seek before beginning")
    IF new_pos > this.length:
        RETURN error("Seek beyond end")
    
    this.position = new_pos
    RETURN this.position
END FUNCTION

FUNCTION PartialPipeline.start() -> UINT64:
    RETURN this.start
END FUNCTION

FUNCTION PartialPipeline.length() -> UINT64:
    RETURN this.length
END FUNCTION

FUNCTION PartialPipeline.position() -> UINT64:
    RETURN this.position
END FUNCTION

FUNCTION PartialPipeline.remaining() -> UINT64:
    RETURN this.length - this.position
END FUNCTION
```

---

## MmapPipeline

Memory-mapped file access.

**Code Reference:** `crates/totalimage-pipeline/src/mmap.rs`

```pseudocode
STRUCTURE MmapPipeline:
    mmap: Mmap                      // Memory-mapped file
    position: UINT64                // Current position
END STRUCTURE

FUNCTION MmapPipeline.from_file(file: File) -> Result<MmapPipeline>:
    mmap = memory_map_file(file)
    IF mmap FAILED:
        RETURN error
    
    RETURN MmapPipeline {
        mmap: mmap,
        position: 0
    }
END FUNCTION

FUNCTION MmapPipeline.read(buffer: MUTABLE<BYTE_ARRAY>) -> Result<UINT>:
    // Calculate remaining bytes
    remaining = this.mmap.length - this.position
    IF remaining == 0:
        RETURN 0  // EOF
    
    // Limit read to remaining bytes
    to_read = MIN(buffer.length, remaining)
    
    // Copy from memory map
    buffer[0:to_read] = this.mmap[this.position:this.position + to_read]
    
    // Update position
    this.position = this.position + to_read
    
    RETURN to_read
END FUNCTION

FUNCTION MmapPipeline.seek(pos: SeekFrom) -> Result<UINT64>:
    new_pos = SWITCH pos:
        CASE SeekFrom::Start(offset):
            offset
        CASE SeekFrom::End(offset):
            this.mmap.length + offset
        CASE SeekFrom::Current(offset):
            this.position + offset
    END SWITCH
    
    // Validate bounds
    IF new_pos < 0 OR new_pos > this.mmap.length:
        RETURN error("Seek out of bounds")
    
    this.position = new_pos
    RETURN this.position
END FUNCTION
```

---

## Code References

### File Structure

```
crates/totalimage-pipeline/src/
├── lib.rs              # Module exports
├── partial.rs          # PartialPipeline (lines 1-232)
└── mmap.rs             # MmapPipeline
```

### Key Functions

#### `partial.rs`
- `PartialPipeline::new`: `crates/totalimage-pipeline/src/partial.rs:41-51`
- `PartialPipeline::read`: `crates/totalimage-pipeline/src/partial.rs:75-96`
- `PartialPipeline::seek`: `crates/totalimage-pipeline/src/partial.rs:100-125`
- `PartialPipeline::start`: `crates/totalimage-pipeline/src/partial.rs:54-56`
- `PartialPipeline::length`: `crates/totalimage-pipeline/src/partial.rs:59-61`
- `PartialPipeline::position`: `crates/totalimage-pipeline/src/partial.rs:64-66`
- `PartialPipeline::remaining`: `crates/totalimage-pipeline/src/partial.rs:69-71`

---

## Cross-References

- **Core Traits:** See [01-core.md](01-core.md) (ReadSeek trait)
- **Vault Usage:** See [02-vaults.md](02-vaults.md) (vaults use pipelines)
- **Zone Usage:** See [03-zones.md](03-zones.md) (zones use PartialPipeline)

---

**Document Version:** 1.0.0  
**Last Updated:** December 21, 2025  
**Next:** [06-acquire.md](06-acquire.md) - Image Acquisition
