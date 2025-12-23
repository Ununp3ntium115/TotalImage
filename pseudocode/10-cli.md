# CLI Module - Pseudocode Documentation

**Component:** `totalimage-cli`  
**Location:** `crates/totalimage-cli/src/`  
**Purpose:** Command-line interface for disk image analysis

---

## Table of Contents

1. [Overview](#overview)
2. [Commands](#commands)
3. [Code References](#code-references)

---

## Overview

The CLI provides command-line tools:
- `info`: Display vault information
- `zones`: List partitions
- `list`: List files
- `extract`: Extract files
- `hash`: Calculate hashes

**Code Reference:** `crates/totalimage-cli/src/main.rs`

---

## Commands

### Info Command

```pseudocode
COMMAND: totalimage info <image>

FUNCTION info_command(image_path: Path) -> Result<VOID>:
    vault = open_vault(image_path, VaultConfig::default())
    
    PRINT "Vault Type: {}", vault.identify()
    PRINT "Vault Size: {} bytes", vault.length()
    
    RETURN success
END FUNCTION
```

### Zones Command

```pseudocode
COMMAND: totalimage zones <image>

FUNCTION zones_command(image_path: Path) -> Result<VOID>:
    vault = open_vault(image_path, VaultConfig::default())
    vault_content = vault.content()
    zone_table = detect_zone_table(vault_content)
    
    IF zone_table IS NULL:
        PRINT "No partition table found"
        RETURN success
    END IF
    
    PRINT "Partition Table: {}", zone_table.identify()
    PRINT ""
    
    FOR EACH zone IN zone_table.enumerate_zones():
        PRINT "Zone {}: {} [{} @ 0x{:08X}, {} bytes]",
            zone.index, zone.zone_type, zone.offset, zone.length
    END FOR
    
    RETURN success
END FUNCTION
```

### List Command

```pseudocode
COMMAND: totalimage list <image> --zone <index> [--directory <path>]

FUNCTION list_command(image_path: Path, zone_index: UINT, directory: OPTIONAL<STRING>) -> Result<VOID>:
    vault = open_vault(image_path, VaultConfig::default())
    vault_content = vault.content()
    zone_table = detect_zone_table(vault_content)
    zone = zone_table.get_zone(zone_index)
    
    zone_stream = PartialPipeline::new(vault_content, zone.offset, zone.length)
    territory = detect_and_parse_territory(zone_stream)
    
    dir = IF directory IS NOT NULL:
        territory.navigate_to(directory)
    ELSE:
        territory.headquarters()
    END IF
    
    occupants = dir.list_occupants()
    
    FOR EACH occupant IN occupants:
        PRINT "{} {:>12} {}",
            IF occupant.is_directory THEN "d" ELSE "f",
            IF occupant.is_directory THEN "<DIR>" ELSE format_size(occupant.size),
            occupant.name
    END FOR
    
    RETURN success
END FUNCTION
```

### Extract Command

```pseudocode
COMMAND: totalimage extract <image> <file> --zone <index> --output <path>

FUNCTION extract_command(
    image_path: Path,
    file_path: STRING,
    zone_index: UINT,
    output_path: Path
) -> Result<VOID>:
    vault = open_vault(image_path, VaultConfig::default())
    vault_content = vault.content()
    zone_table = detect_zone_table(vault_content)
    zone = zone_table.get_zone(zone_index)
    
    zone_stream = PartialPipeline::new(vault_content, zone.offset, zone.length)
    territory = detect_and_parse_territory(zone_stream)
    
    file_data = territory.extract_file(file_path)
    write_file(output_path, file_data)
    
    PRINT "Extracted {} bytes to {}", file_data.length, output_path
    
    RETURN success
END FUNCTION
```

---

## Code References

### File Structure

```
crates/totalimage-cli/src/
└── main.rs             # CLI implementation
```

---

## Cross-References

- **All Components:** Uses all other modules
- **MCP Server:** See [07-mcp-server.md](07-mcp-server.md) (similar operations)

---

**Document Version:** 1.0.0  
**Last Updated:** December 21, 2025  
**Back to:** [00-INDEX.md](00-INDEX.md)
