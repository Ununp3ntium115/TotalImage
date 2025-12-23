# Core Module - Pseudocode Documentation

**Component:** `totalimage-core`  
**Location:** `crates/totalimage-core/src/`  
**Purpose:** Core traits, types, error handling, and security utilities

---

## Table of Contents

1. [Overview](#overview)
2. [Data Structures](#data-structures)
3. [Core Traits](#core-traits)
4. [Error Handling](#error-handling)
5. [Security Utilities](#security-utilities)
6. [Type Definitions](#type-definitions)
7. [Code References](#code-references)

---

## Overview

The core module provides the foundational abstractions for the TotalImage project:
- **Traits**: Vault, Territory, ZoneTable, DirectoryCell
- **Error Types**: Comprehensive error handling
- **Security**: Validation functions and limits
- **Types**: OccupantInfo, Zone

**Code Reference:** `crates/totalimage-core/src/lib.rs:1-51`

---

## Data Structures

### OccupantInfo

Represents a file or directory entry in a filesystem.

**Code Reference:** `crates/totalimage-core/src/types.rs:9-82`

```pseudocode
STRUCTURE OccupantInfo:
    name: STRING                    // File or directory name
    is_directory: BOOLEAN           // True if directory, false if file
    size: UINT64                    // Size in bytes (0 for directories)
    created: OPTIONAL<DATETIME>     // Creation timestamp
    modified: OPTIONAL<DATETIME>    // Last modified timestamp
    accessed: OPTIONAL<DATETIME>    // Last accessed timestamp
    attributes: UINT32              // File attributes (platform-specific)
END STRUCTURE

FUNCTION OccupantInfo.file(name: STRING, size: UINT64) -> OccupantInfo:
    RETURN OccupantInfo {
        name: name,
        is_directory: false,
        size: size,
        created: NULL,
        modified: NULL,
        accessed: NULL,
        attributes: 0
    }
END FUNCTION

FUNCTION OccupantInfo.directory(name: STRING) -> OccupantInfo:
    RETURN OccupantInfo {
        name: name,
        is_directory: true,
        size: 0,
        created: NULL,
        modified: NULL,
        accessed: NULL,
        attributes: 0
    }
END FUNCTION
```

### Zone

Represents a partition (zone) within a disk image.

**Code Reference:** `crates/totalimage-core/src/types.rs:121-169`

```pseudocode
STRUCTURE Zone:
    index: UINT                    // Zone index (0-based)
    offset: UINT64                 // Offset from vault start (bytes)
    length: UINT64                  // Zone length (bytes)
    zone_type: STRING              // Partition type (e.g., "FAT32", "NTFS")
    territory_type: OPTIONAL<STRING> // Detected filesystem type
END STRUCTURE

FUNCTION Zone.new(index: UINT, offset: UINT64, length: UINT64, zone_type: STRING) -> Zone:
    RETURN Zone {
        index: index,
        offset: offset,
        length: length,
        zone_type: zone_type,
        territory_type: NULL
    }
END FUNCTION

FUNCTION Zone.with_territory_type(territory_type: STRING) -> Zone:
    this.territory_type = territory_type
    RETURN this
END FUNCTION
```

---

## Core Traits

### Vault Trait

Trait for container formats (Raw, VHD, E01, AFF4).

**Code Reference:** `crates/totalimage-core/src/traits.rs:10-19`

```pseudocode
TRAIT Vault:
    // Get human-readable identifier
    FUNCTION identify() -> STRING
    
    // Get total size in bytes
    FUNCTION length() -> UINT64
    
    // Get readable/seekable stream to vault content
    FUNCTION content() -> MUTABLE<ReadSeek>
END TRAIT

// Example implementation pseudocode
FUNCTION RawVault.identify() -> STRING:
    RETURN "Raw Sector Image"
END FUNCTION

FUNCTION RawVault.length() -> UINT64:
    RETURN this.file_size
END FUNCTION

FUNCTION RawVault.content() -> MUTABLE<ReadSeek>:
    RETURN this.file_handle
END FUNCTION
```

### Territory Trait

Trait for filesystem implementations (FAT, exFAT, ISO, NTFS).

**Code Reference:** `crates/totalimage-core/src/traits.rs:36-70`

```pseudocode
TRAIT Territory:
    // Get human-readable identifier
    FUNCTION identify() -> STRING
    
    // Get volume label (banner)
    FUNCTION banner() -> Result<STRING>
    
    // Set volume label (may be unsupported for read-only)
    FUNCTION set_banner(label: STRING) -> Result<VOID>
        DEFAULT: RETURN Error::Unsupported("Setting banner not supported")
    
    // Get root directory
    FUNCTION headquarters() -> Result<DirectoryCell>
    
    // Get total size in bytes
    FUNCTION domain_size() -> UINT64
    
    // Get free space in bytes
    FUNCTION liberated_space() -> UINT64
    
    // Get allocation unit size (cluster/block)
    FUNCTION block_size() -> UINT64
    
    // Does this territory support subdirectories?
    FUNCTION hierarchical() -> BOOLEAN
    
    // Navigate to directory by path
    FUNCTION navigate_to(path: STRING) -> Result<DirectoryCell>
    
    // Extract file by path
    FUNCTION extract_file(path: STRING) -> Result<BYTE_ARRAY>
END TRAIT
```

### ZoneTable Trait

Trait for partition table parsers (MBR, GPT).

**Code Reference:** `crates/totalimage-core/src/traits.rs:22-33`

```pseudocode
TRAIT ZoneTable:
    // Get human-readable identifier
    FUNCTION identify() -> STRING
    
    // Get all zones (partitions)
    FUNCTION enumerate_zones() -> ARRAY<Zone>
    
    // Get specific zone by index
    FUNCTION get_zone(index: UINT) -> OPTIONAL<Zone>
        DEFAULT:
            zones = this.enumerate_zones()
            IF index < zones.length:
                RETURN zones[index]
            ELSE:
                RETURN NULL
END TRAIT
```

### DirectoryCell Trait

Trait for directory navigation.

**Code Reference:** `crates/totalimage-core/src/traits.rs:73-92`

```pseudocode
TRAIT DirectoryCell:
    // Get directory name
    FUNCTION name() -> STRING
    
    // List all occupants (files/subdirectories)
    FUNCTION list_occupants() -> Result<ARRAY<OccupantInfo>>
    
    // Enter subdirectory by name
    FUNCTION enter(name: STRING) -> Result<DirectoryCell>
    
    // Check if file/directory exists
    FUNCTION exists(name: STRING) -> Result<BOOLEAN>
        DEFAULT:
            occupants = this.list_occupants()
            RETURN occupants.contains(o => o.name == name)
    
    // Get info about specific occupant
    FUNCTION get_occupant(name: STRING) -> Result<OPTIONAL<OccupantInfo>>
        DEFAULT:
            occupants = this.list_occupants()
            RETURN occupants.find(o => o.name == name)
END TRAIT
```

---

## Error Handling

### Error Enum

Comprehensive error types for all operations.

**Code Reference:** `crates/totalimage-core/src/error.rs:7-113`

```pseudocode
ENUM Error:
    Io(io::Error)                          // I/O errors
    InvalidVault(STRING)                   // Invalid container format
    InvalidZoneTable(STRING)               // Invalid partition table
    InvalidTerritory(STRING)               // Invalid filesystem
    SignatureVerification(STRING)          // Signature check failed
    ChecksumVerification(STRING)            // Checksum check failed
    Unsupported(STRING)                    // Unsupported feature
    NotFound(STRING)                       // File/directory not found
    InvalidPath(STRING)                    // Invalid path
    AlreadyExists(STRING)                  // Resource already exists
    PermissionDenied(STRING)               // Permission denied
    InvalidOperation(STRING)               // Invalid operation
    Encoding(STRING)                       // Encoding error
    Custom(STRING)                         // Generic error
END ENUM

TYPE Result<T> = Result<T, Error>

// Helper functions
FUNCTION Error.invalid_vault(message: STRING) -> Error:
    RETURN Error::InvalidVault(message)
END FUNCTION

FUNCTION Error.invalid_zone_table(message: STRING) -> Error:
    RETURN Error::InvalidZoneTable(message)
END FUNCTION

FUNCTION Error.invalid_territory(message: STRING) -> Error:
    RETURN Error::InvalidTerritory(message)
END FUNCTION

FUNCTION Error.not_found(message: STRING) -> Error:
    RETURN Error::NotFound(message)
END FUNCTION
```

---

## Security Utilities

### Security Constants

**Code Reference:** `crates/totalimage-core/src/security.rs:11-35`

```pseudocode
CONSTANTS:
    MAX_SECTOR_SIZE: UINT32 = 4096                    // 4KB max sector
    MAX_ALLOCATION_SIZE: UINT = 256 * 1024 * 1024     // 256 MB max buffer
    MAX_FAT_TABLE_SIZE: UINT = 100 * 1024 * 1024      // 100 MB max FAT
    MAX_PARTITION_COUNT: UINT = 256                   // Max partitions
    MAX_DIRECTORY_ENTRIES: UINT = 10000                // Max dir entries
    MAX_FILE_EXTRACT_SIZE: UINT64 = 1 GB               // Max file size
    MAX_CLUSTER_CHAIN_LENGTH: UINT = 1000000           // Max cluster chain
    MAX_MMAP_SIZE: UINT64 = 16 GB                     // Max memory map
    DEFAULT_ALLOWED_ROOT_ENV: STRING = "TOTALIMAGE_ALLOWED_ROOT"
END CONSTANTS
```

### Path Validation

**Code Reference:** `crates/totalimage-core/src/security.rs:156-239`

```pseudocode
FUNCTION validate_file_path(path: STRING, allowed_roots: ARRAY<PathBuf>) -> Result<PathBuf>:
    // Normalize path
    normalized = normalize_path(path)
    
    // Check for path traversal
    IF normalized.contains(".."):
        RETURN Error::InvalidPath("Path traversal detected")
    
    // Resolve to absolute path
    absolute = resolve_absolute(normalized)
    
    // Check against allowed roots
    is_allowed = false
    FOR EACH root IN allowed_roots:
        IF absolute.starts_with(root):
            is_allowed = true
            BREAK
    
    IF NOT is_allowed:
        RETURN Error::InvalidPath("Path not in allowed roots")
    
    // Validate path exists and is accessible
    IF NOT file_exists(absolute):
        RETURN Error::NotFound("File does not exist")
    
    RETURN success(absolute)
END FUNCTION
```

### Allocation Size Validation

**Code Reference:** `crates/totalimage-core/src/security.rs:88-98`

```pseudocode
FUNCTION validate_allocation_size(size: UINT64, limit: UINT, context: STRING) -> Result<UINT>:
    // Check against limit
    IF size > limit:
        RETURN Error::InvalidVault(
            format("{} size {} exceeds limit {}", context, size, limit)
        )
    
    // Convert to usize safely
    TRY:
        size_usize = size as usize
        RETURN success(size_usize)
    CATCH:
        RETURN Error::InvalidVault(
            format("{} size exceeds platform limits", context)
        )
END FUNCTION
```

### Checked Arithmetic

**Code Reference:** `crates/totalimage-core/src/security.rs:104-119`

```pseudocode
FUNCTION checked_multiply_u64(a: UINT64, b: UINT64, context: STRING) -> Result<UINT64>:
    result = a.checked_mul(b)
    IF result IS NULL:
        RETURN Error::InvalidVault(
            format("{} multiplication overflow: {} * {}", context, a, b)
        )
    RETURN success(result)
END FUNCTION

FUNCTION checked_multiply_u32_to_u64(a: UINT32, b: UINT32, context: STRING) -> Result<UINT64>:
    a64 = a as UINT64
    b64 = b as UINT64
    RETURN checked_multiply_u64(a64, b64, context)
END FUNCTION
```

### Allowed Roots from Environment

**Code Reference:** `crates/totalimage-core/src/security.rs:38-82`

```pseudocode
FUNCTION allowed_roots_from_env() -> Result<ARRAY<PathBuf>>:
    RETURN allowed_roots_from_env_var(DEFAULT_ALLOWED_ROOT_ENV)
END FUNCTION

FUNCTION allowed_roots_from_env_var(var_name: STRING) -> Result<ARRAY<PathBuf>>:
    // Get environment variable
    value = get_env(var_name)
    IF value IS NULL:
        RETURN Error::InvalidVault(
            format("{} must be set to directories TotalImage can access", var_name)
        )
    
    // Parse path list (platform-specific separator)
    roots = []
    FOR EACH raw_path IN split_paths(value):
        IF raw_path IS EMPTY:
            CONTINUE
        
        // Canonicalize path
        canonical = canonicalize(raw_path)
        IF canonical FAILED:
            RETURN Error::InvalidVault(
                format("Allowed root {} is invalid", raw_path)
            )
        
        // Verify is directory
        IF NOT is_directory(canonical):
            RETURN Error::InvalidVault(
                format("Allowed root {} is not a directory", canonical)
            )
        
        roots.append(canonical)
    
    // Must have at least one root
    IF roots IS EMPTY:
        RETURN Error::InvalidVault(
            format("{} must contain at least one directory", var_name)
        )
    
    RETURN success(roots)
END FUNCTION
```

---

## Type Definitions

### ReadSeek Trait

Combined Read + Seek trait.

**Code Reference:** `crates/totalimage-core/src/traits.rs:95-98`

```pseudocode
TRAIT ReadSeek: Read + Seek + Send + Sync
    // Blanket implementation for any type implementing Read + Seek + Send + Sync
END TRAIT
```

### ReadWriteSeek Trait

Combined Read + Write + Seek trait.

**Code Reference:** `crates/totalimage-core/src/traits.rs:101-104`

```pseudocode
TRAIT ReadWriteSeek: Read + Write + Seek + Send + Sync
    // Blanket implementation for any type implementing Read + Write + Seek + Send + Sync
END TRAIT
```

---

## Code References

### File Structure

```
crates/totalimage-core/src/
├── lib.rs              # Module exports (lines 1-51)
├── error.rs            # Error types (lines 1-113)
├── security.rs         # Security utilities (lines 1-624)
├── traits.rs           # Core traits (lines 1-105)
├── types.rs            # Type definitions (lines 1-209)
└── proptest.rs         # Property testing utilities (lines 1-53)
```

### Key Functions by File

#### `lib.rs`
- Module organization: `crates/totalimage-core/src/lib.rs:38-44`
- Public exports: `crates/totalimage-core/src/lib.rs:47-50`

#### `error.rs`
- Error enum: `crates/totalimage-core/src/error.rs:7-63`
- Result type: `crates/totalimage-core/src/error.rs:66`
- Helper functions: `crates/totalimage-core/src/error.rs:68-113`

#### `traits.rs`
- Vault trait: `crates/totalimage-core/src/traits.rs:10-19`
- ZoneTable trait: `crates/totalimage-core/src/traits.rs:22-33`
- Territory trait: `crates/totalimage-core/src/traits.rs:36-70`
- DirectoryCell trait: `crates/totalimage-core/src/traits.rs:73-92`
- ReadSeek trait: `crates/totalimage-core/src/traits.rs:95-98`
- ReadWriteSeek trait: `crates/totalimage-core/src/traits.rs:101-104`

#### `types.rs`
- OccupantInfo struct: `crates/totalimage-core/src/types.rs:9-82`
- Zone struct: `crates/totalimage-core/src/types.rs:121-169`
- Format size function: `crates/totalimage-core/src/types.rs:102-117`

#### `security.rs`
- Constants: `crates/totalimage-core/src/security.rs:11-35`
- Allowed roots: `crates/totalimage-core/src/security.rs:38-82`
- Allocation validation: `crates/totalimage-core/src/security.rs:88-98`
- Checked arithmetic: `crates/totalimage-core/src/security.rs:104-119`
- Path validation: `crates/totalimage-core/src/security.rs:156-239`
- Filename sanitization: `crates/totalimage-core/src/security.rs:240-256`

---

## Cross-References

- **Vault Implementations:** See [02-vaults.md](02-vaults.md)
- **Territory Implementations:** See [04-territories.md](04-territories.md)
- **Zone Implementations:** See [03-zones.md](03-zones.md)
- **Pipeline Usage:** See [05-pipeline.md](05-pipeline.md)

---

**Document Version:** 1.0.0  
**Last Updated:** December 21, 2025  
**Next:** [02-vaults.md](02-vaults.md) - Container Format Implementations
