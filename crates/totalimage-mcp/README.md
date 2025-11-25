# TotalImage MCP Server

Model Context Protocol (MCP) server for disk image analysis with Claude Desktop.

## Overview

TotalImage MCP Server provides Claude with forensic analysis capabilities for disk images. It supports:

- **Raw disk images** (.img, .dsk, .iso)
- **VHD containers** (Fixed and Dynamic)
- **Partition tables** (MBR and GPT)
- **Filesystems** (FAT12/16/32, ISO-9660)

## Installation

### Build from Source

```bash
cargo build --release -p totalimage-mcp
```

The binary will be at `target/release/totalimage-mcp`.

### Claude Desktop Configuration

Add to your Claude Desktop configuration file:

**macOS:** `~/Library/Application Support/Claude/claude_desktop_config.json`
**Linux:** `~/.config/claude/claude_desktop_config.json`
**Windows:** `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "totalimage": {
      "command": "/path/to/totalimage-mcp",
      "args": ["standalone"],
      "env": {
        "RUST_LOG": "info"
      }
    }
  }
}
```

## Usage Modes

### Standalone Mode (Claude Desktop)

```bash
totalimage-mcp standalone [--cache-dir DIR]
```

Uses stdio transport for direct Claude Desktop integration.

### Integrated Mode (Fire Marshal)

```bash
totalimage-mcp integrated --marshal-url http://localhost:3001 --port 3002
```

Uses HTTP transport and registers with Fire Marshal framework.

### Auto-Detect Mode

```bash
totalimage-mcp auto
```

Automatically selects mode based on `FIRE_MARSHAL_URL` environment variable.

## Tools

### 1. analyze_disk_image

Comprehensive disk image analysis including vault type, partitions, filesystems, and security validation.

**Input:**
```json
{
  "path": "/path/to/disk.img",
  "cache": true,
  "deep_scan": false
}
```

**Output:**
- Vault type and size
- Partition table type (MBR/GPT)
- Filesystem information
- Security validation results

### 2. list_partitions

Enumerate all partitions (zones) in a disk image.

**Input:**
```json
{
  "path": "/path/to/disk.img",
  "cache": true
}
```

**Output:**
- Partition count
- Each partition's type, offset, size
- Filesystem type detection

### 3. list_files

List files in a disk image filesystem.

**Input:**
```json
{
  "path": "/path/to/disk.img",
  "zone_index": 0,
  "cache": true
}
```

**Output:**
- File names
- File sizes
- File types (file/directory)
- File attributes

### 4. extract_file

Extract a file from a disk image to the local filesystem.

**Input:**
```json
{
  "image_path": "/path/to/disk.img",
  "file_path": "AUTOEXEC.BAT",
  "zone_index": 0,
  "output_path": "/tmp/extracted.bat"
}
```

**Output:**
- Success/failure status
- Bytes extracted
- Output path

### 5. validate_integrity

Validate disk image structure and checksums.

**Input:**
```json
{
  "path": "/path/to/disk.img",
  "check_checksums": true,
  "check_boot_sectors": true
}
```

**Output:**
- Overall validity status
- Checksum validation results
- Boot sector validation
- Any issues found

## Example Claude Conversations

### Analyze a Disk Image

> **User:** Analyze the disk image at /home/user/floppy.img

Claude will use the `analyze_disk_image` tool to examine the image and report:
- Container type (Raw, VHD)
- Partition layout
- Filesystem type and details
- Any security concerns

### Extract a File

> **User:** Extract CONFIG.SYS from /home/user/dos.img to my desktop

Claude will:
1. Use `list_files` to find CONFIG.SYS
2. Use `extract_file` to save it to the specified location

### Verify Integrity

> **User:** Check if this VHD file is corrupted: /home/user/backup.vhd

Claude will use `validate_integrity` to check:
- VHD footer checksum
- Dynamic header checksum (if applicable)
- Boot sector signatures
- Partition table integrity

## Command Line Options

```
TotalImage MCP Server - Disk Image Analysis for Claude

Usage: totalimage-mcp [OPTIONS] [COMMAND]

Commands:
  standalone  Standalone mode (stdio transport for Claude Desktop)
  integrated  Integrated mode (HTTP transport + Fire Marshal registration)
  auto        Auto-detect mode based on environment variables

Options:
      --cache-dir <CACHE_DIR>  Cache directory for results
      --log-level <LOG_LEVEL>  Log level (trace, debug, info, warn, error) [default: info]
  -h, --help                   Print help
  -V, --version                Print version
```

## Protocol Details

- **Protocol Version:** 2024-11-05
- **Transport:** stdio (standalone) or HTTP (integrated)
- **Message Format:** JSON-RPC 2.0

## Security

- Path validation prevents directory traversal attacks
- Allocation limits prevent memory exhaustion
- Checked arithmetic prevents integer overflow

See `/SECURITY.md` for full security documentation.

## Supported Formats

| Format | Read | Write | Notes |
|--------|------|-------|-------|
| Raw (.img, .dsk) | Yes | No | Direct sector access |
| ISO-9660 (.iso) | Yes | No | CD-ROM images |
| VHD Fixed | Yes | No | Microsoft Virtual PC |
| VHD Dynamic | Yes | No | Sparse block allocation |
| FAT12/16/32 | Yes | No | Floppy and hard disk |
| MBR | Yes | No | Legacy partitions |
| GPT | Yes | No | Modern partitions |

## Caching

Results are cached in a redb database for performance:

- Default location: `~/.cache/totalimage/mcp-cache.redb`
- TTL: 30 days
- Configure with `--cache-dir` option

## Logging

Set log level via `RUST_LOG` environment variable:

```bash
RUST_LOG=debug totalimage-mcp standalone
```

Levels: `trace`, `debug`, `info`, `warn`, `error`

## Development

### Run Tests

```bash
cargo test -p totalimage-mcp
```

### Build Debug

```bash
cargo build -p totalimage-mcp
```

### Test MCP Protocol

```bash
echo '{"jsonrpc":"2.0","id":"1","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}' | ./target/release/totalimage-mcp standalone
```

## License

GPL-3.0-or-later

## Related Documentation

- [PYRO Integration Design](/steering/PYRO-INTEGRATION-DESIGN.md)
- [Implementation Status](/steering/IMPLEMENTATION-STATUS.md)
- [Security Policy](/SECURITY.md)
