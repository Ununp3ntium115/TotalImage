# node-red-contrib-totalimage

Node-RED nodes for TotalImage disk image analysis - an open-source alternative to FTK Imager.

## Overview

This package provides Node-RED nodes for analyzing and extracting files from disk images using the TotalImage MCP server or Fire Marshal tool orchestration framework.

## Installation

```bash
cd ~/.node-red
npm install /path/to/TotalImage/node-red-contrib-totalimage
```

Or from npm (when published):
```bash
npm install node-red-contrib-totalimage
```

## Prerequisites

Start the TotalImage MCP server:
```bash
# Standalone mode (stdio - not for Node-RED)
./totalimage-mcp standalone

# Integrated mode (HTTP - required for Node-RED)
./totalimage-mcp integrated --port 3002

# Or use Fire Marshal for multi-tool orchestration
./fire-marshal start --port 3001
```

## Nodes

### totalimage-config

Configuration node for TotalImage server connection.

- **Host**: Server hostname (default: localhost)
- **Port**: Server port (default: 3002)
- **Protocol**: HTTP or HTTPS
- **Timeout**: Request timeout in milliseconds
- **API Key**: Optional authentication

### totalimage-analyze

Analyzes a disk image file and returns comprehensive information.

**Inputs:**
- `msg.payload.path` - Path to disk image file
- `msg.payload.deepScan` - Enable deep filesystem scan
- `msg.payload.useCache` - Use cached results

**Outputs:**
- `msg.payload.analysis` - Analysis results (vault type, partitions, filesystems)

### totalimage-list-partitions

Lists all partitions/zones in a disk image.

**Inputs:**
- `msg.payload.path` - Path to disk image file
- `msg.payload.useCache` - Use cached results

**Outputs:**
- `msg.payload.partitions` - Array of partition objects (index, offset, length, type)
- `msg.payload.partitionTable` - Partition table type (MBR or GPT)
- `msg.payload.count` - Number of partitions found

### totalimage-list-files

Lists files in a disk image filesystem.

**Inputs:**
- `msg.payload.path` - Path to disk image file
- `msg.payload.zoneIndex` - Partition index (0 = first)
- `msg.payload.directory` - Directory path within filesystem

**Outputs:**
- `msg.payload.files` - Array of file objects

### totalimage-extract

Extracts a file from a disk image.

**Inputs:**
- `msg.payload.imagePath` - Path to disk image
- `msg.payload.filePath` - Path to file within image
- `msg.payload.outputPath` - Destination for extracted file
- `msg.payload.zoneIndex` - Partition containing the file

**Outputs:**
- `msg.payload.success` - Extraction result
- `msg.payload.outputPath` - Where file was saved

### totalimage-validate-integrity

Validates the integrity of a disk image file.

**Inputs:**
- `msg.payload.path` - Path to disk image file
- `msg.payload.checkChecksums` - Verify embedded checksums
- `msg.payload.checkBootSectors` - Validate boot sector signatures

**Outputs:**
- `msg.payload.valid` - Overall validation result (boolean)
- `msg.payload.issues` - Array of validation issues found
- `msg.payload.checksums` - Computed checksums (MD5, SHA1, SHA256)

## Supported Formats

### Container Formats (Vaults)
- Raw sector images (.img, .dd, .raw, .iso)
- VHD Fixed, Dynamic, and Differencing
- E01 (EnCase) forensic format
- AFF4 (Advanced Forensic Format 4)

### Partition Tables (Zones)
- MBR (Master Boot Record)
- GPT (GUID Partition Table)

### Filesystems (Territories)
- FAT12, FAT16, FAT32 (with Long File Name support)
- exFAT
- ISO-9660

## Example Flow

```json
[
    {
        "id": "analyze-node",
        "type": "totalimage-analyze",
        "server": "config-node",
        "imagePath": "/images/disk.img"
    },
    {
        "id": "list-node",
        "type": "totalimage-list-files",
        "server": "config-node",
        "imagePath": "/images/disk.img",
        "directory": "WINDOWS"
    },
    {
        "id": "extract-node",
        "type": "totalimage-extract",
        "server": "config-node",
        "imagePath": "/images/disk.img",
        "filePath": "AUTOEXEC.BAT",
        "outputPath": "/tmp/AUTOEXEC.BAT"
    }
]
```

## Integration with PYRO Platform

These nodes integrate with the PYRO Platform Ignition system through Fire Marshal:

1. Fire Marshal provides tool orchestration and rate limiting
2. TotalImage MCP server registers with Fire Marshal
3. Node-RED flows can use these nodes for visual workflow automation
4. Multiple tools can be coordinated in a single flow

## License

GPL-3.0 - see LICENSE file for details.

## Links

- [TotalImage GitHub](https://github.com/Ununp3ntium115/TotalImage)
- [Node-RED](https://nodered.org/)
- [PYRO Platform](https://github.com/Ununp3ntium115/TotalImage/blob/main/steering/PYRO-INTEGRATION-DESIGN.md)
