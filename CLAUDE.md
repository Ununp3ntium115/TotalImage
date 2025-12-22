# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

TotalImage is a forensic disk image analysis tool written in Rust. It parses disk images, partition tables, and filesystems with memory-safe, zero-copy operations. The project uses anarchist-themed terminology: Vault (container format), Territory (filesystem), Zone (partition).

## Build Commands

```bash
# Build all crates (debug)
cargo build

# Build specific binaries (release, optimized)
cargo build --release -p totalimage-cli
cargo build --release -p totalimage-web
cargo build --release -p totalimage-mcp
cargo build --release -p fire-marshal

# Check without building
cargo check --all-targets

# Linting
cargo clippy -- -D warnings
```

## Testing

```bash
# Run all tests
cargo test --all-targets

# Run tests for a specific crate
cargo test -p totalimage-zones
cargo test -p totalimage-mcp

# Run tests with output
cargo test -- --nocapture
```

## Fuzzing (requires nightly Rust)

```bash
cargo install cargo-fuzz
cargo +nightly fuzz run fuzz_mbr_parser -- -max_total_time=60
```

Fuzz targets: `fuzz_mbr_parser`, `fuzz_gpt_parser`, `fuzz_fat_bpb`, `fuzz_vhd_footer`, `fuzz_e01_header`

## Web UI (Svelte + Vite)

```bash
cd web-ui
npm install
npm run dev        # Development server at localhost:5173
npm run build      # Production build
npm run check      # Type checking
```

## Architecture

### Rust Workspace Crates (`crates/`)

- **totalimage-core**: Traits, error types, security validation utilities (checked arithmetic, allocation limits)
- **totalimage-pipeline**: I/O abstractions (memory-mapped files, streaming)
- **totalimage-vaults**: Container format parsers (Raw, VHD, E01, AFF4)
- **totalimage-zones**: Partition table parsers (MBR, GPT)
- **totalimage-territories**: Filesystem parsers (FAT12/16/32, exFAT, NTFS, ISO-9660)
- **totalimage-acquire**: Image acquisition utilities
- **totalimage-cli**: Command-line interface
- **totalimage-web**: REST API server (Axum)
- **totalimage-mcp**: Model Context Protocol server for Claude Desktop integration
- **fire-marshal**: Tool orchestration framework for PYRO Platform

### Node/TypeScript Packages

- **web-ui/**: Svelte 5 frontend for the web interface
- **node-red-contrib-totalimage/**: Node-RED nodes for disk image analysis
- **packages/pyro-worker-totalimage/**: BullMQ worker for PYRO Platform

### Data Flow

1. **Vault** opens container format (VHD, raw, E01) → provides sector-level access
2. **Zone** parser reads partition table (MBR/GPT) → enumerates partitions
3. **Territory** parser accesses filesystem (FAT, NTFS, ISO) → provides file operations

## Security Requirements

TotalImage parses untrusted binary data. All parsers must:

- Use checked arithmetic (`checked_add`, `checked_mul`) to prevent integer overflow
- Validate allocation sizes against limits (256 MB max buffer, 100 MB FAT table, 1 GB extraction)
- Use `validate_file_path()` from `totalimage_core::security` for path validation
- Never expose internal error details in web API responses

Key constants in `totalimage-core/src/security.rs`:
- `MAX_SECTOR_SIZE`: 4KB
- `MAX_ALLOCATION_SIZE`: 256 MB
- `MAX_FAT_ALLOCATION`: 100 MB
- `MAX_EXTRACTION_SIZE`: 1 GB

## Environment Variables

- `RUST_LOG`: Logging level (`trace`, `debug`, `info`, `warn`, `error`)
- `TOTALIMAGE_CACHE_DIR`: Cache directory (default: `~/.cache/totalimage`)

## CLI Usage

```bash
totalimage-cli info <image>           # Display vault information
totalimage-cli zones <image>          # List partitions
totalimage-cli list <image> --zone N  # List files in partition
totalimage-cli extract <image> <file> --zone N --output out.txt
```

## MCP Server

The MCP server (`totalimage-mcp`) provides 5 tools for Claude Desktop: `analyze_disk_image`, `list_partitions`, `list_files`, `extract_file`, `validate_integrity`.

Configure in Claude Desktop:
```json
{
  "mcpServers": {
    "totalimage": {
      "command": "/path/to/totalimage-mcp",
      "args": ["standalone"]
    }
  }
}
```

## Kubernetes Deployment

See `k8s/README.md` for production deployment. Key components:
- `totalimage-web`: REST API (2-10 replicas with HPA)
- `totalimage-mcp`: MCP server (2-8 replicas)
- `fire-marshal`: Tool registry (single replica)
