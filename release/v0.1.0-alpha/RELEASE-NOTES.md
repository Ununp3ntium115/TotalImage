# TotalImage v0.1.0-alpha Release Notes

**Release Date:** 2025-11-26
**Platform:** Linux x86_64 (glibc)

## Overview

TotalImage is a forensic disk image analysis toolkit designed to replace FTK Imager, providing comprehensive support for forensic image formats, filesystems, and integration with the PYRO Platform.

## Binaries Included

| Binary | Description | Size |
|--------|-------------|------|
| `totalimage` | CLI for disk image analysis | 934 KB |
| `totalimage-mcp` | MCP server for AI tool integration | 7.5 MB |
| `totalimage-web` | Web interface server | 3.1 MB |
| `fire-marshal` | Tool orchestration framework | 5.7 MB |

## Features

### Disk Image Formats (Vaults)
- **AFF4** - Advanced Forensic Format v4 with LRU caching
- **E01** - EnCase Expert Witness Format with single-chunk cache
- **VHD/VHDX** - Microsoft Virtual Hard Disk (including differencing chains)
- **Raw** - DD/raw image format

### Partition Tables (Zones)
- **MBR** - Master Boot Record
- **GPT** - GUID Partition Table
- **APM** - Apple Partition Map

### Filesystems (Territories)
- **NTFS** - Read-only support (NEW in this release)
- **FAT12/FAT16/FAT32** - Full read support
- **exFAT** - Extended FAT support
- **ISO9660/UDF** - Optical disc formats

### MCP Server Features
- JWT authentication support
- WebSocket progress notifications
- Tool caching with redb database
- Fire Marshal integration

## Test Coverage

| Component | Tests |
|-----------|-------|
| totalimage-mcp | 50 |
| fire-marshal | 21 |
| totalimage-vaults | 59 |
| totalimage-territories | 36 |
| totalimage-zones | 20 |
| **Total** | **190+** |

## Known Issues

- AFF4 Snappy/LZ4 compression not yet implemented (GAP-011)
- AFF4 chunk offset calculation edge case (GAP-003)
- Path traversal protection incomplete (GAP-006)

## Requirements

- Linux x86_64 (glibc 2.17+)
- OpenSSL 1.1.1+

## Usage

```bash
# Basic disk image analysis
./totalimage analyze /path/to/image.vhd

# Start MCP server (for AI integration)
./totalimage-mcp --port 3000

# Start web interface
./totalimage-web --port 8080

# Start Fire Marshal orchestrator
./fire-marshal --port 3001
```

## PYRO Platform Integration

This release includes a TypeScript worker package for PYRO Platform integration:
- Located in `packages/pyro-worker-totalimage/`
- Supports Redis job queues
- WebSocket progress notifications
- JWT authentication

## License

GPL-3.0-or-later

## Links

- Repository: https://github.com/Ununp3ntium115/TotalImage
- SDLC Roadmap: See `steering/SDLC-ROADMAP.md`
