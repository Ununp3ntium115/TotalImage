# TotalImage (Rust Edition)

**TotalImage** is a fast, secure disk image analysis tool written in Rust. Parse and analyze disk images, partition tables, and filesystems with memory-safe, zero-copy operations.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Tests](https://img.shields.io/badge/tests-94%20passing-brightgreen)](tests)
[![Security](https://img.shields.io/badge/security-hardened-blue)](SECURITY.md)

## Features

### ✅ Implemented (v0.1.0)

**Container Formats (Vaults)**
- ✅ Raw sector images (.img, .ima, .bin)
- ✅ VHD (Virtual Hard Disk) - Fixed and Dynamic
  - Footer checksum validation
  - Block Allocation Table (BAT) parsing
  - Sparse block support

**Partition Tables (Zones)**
- ✅ MBR (Master Boot Record)
  - CHS addressing support
  - 15+ partition type detection
- ✅ GPT (GUID Partition Table)
  - **CRC32 integrity validation** (header + entries)
  - UTF-16LE partition names
  - GUID-based type identification

**Filesystems (Territories)**
- ✅ FAT12/16/32
  - **Secure BPB parsing** with checked arithmetic
  - Cluster chain traversal
  - File listing and extraction
- ✅ ISO-9660
  - Both-endian integer parsing
  - Volume descriptor parsing
  - Primary volume support

**CLI Tool**
- `totalimage-cli info <image>` - Display vault information
- `totalimage-cli zones <image>` - List partitions
- `totalimage-cli list <image> --zone N` - List files in partition
- `totalimage-cli extract <image> <file> --zone N` - Extract file

**Web API** (REST)
- `GET /api/vault/info?path=<image>` - Vault metadata
- `GET /api/vault/zones?path=<image>` - Partition listing
- Metadata caching with redb (30-day TTL)

## Security

TotalImage has undergone comprehensive security hardening (see [SECURITY.md](SECURITY.md)):

### ✅ Mitigated Vulnerabilities

- **✅ SEC-001 (Critical)**: Integer overflow protection with checked arithmetic
- **✅ SEC-002 (Critical)**: Memory allocation limits (256 MB max per buffer)
- **✅ SEC-003 (Critical)**: Path traversal prevention in web API
- **✅ SEC-004 (High)**: Memory-mapped file validation (16 GB limit, type checking)
- **✅ SEC-005 (High)**: Explicit CLI argument error handling
- **✅ SEC-006 (High)**: GPT CRC32 integrity verification

### Security Features

- **Zero unsafe code** in application layer (memmap2 only)
- **Checked arithmetic** throughout parsers
- **Allocation limits**: 256 MB general, 100 MB FAT, 1 GB extraction
- **Path validation**: Canonical paths, no traversal attacks
- **Checksum enforcement**: VHD footers, GPT headers/entries
- **Error sanitization**: No internal details exposed

See [GAP-ANALYSIS.md](steering/GAP-ANALYSIS.md) for complete security audit.

## Installation

### From Source

```bash
# Clone repository
git clone https://github.com/Ununp3ntium115/TotalImage.git
cd TotalImage

# Build CLI tool
cargo build --release -p totalimage-cli

# Build web server
cargo build --release -p totalimage-web

# Run tests
cargo test --all-targets
```

### System Requirements

- **Rust**: 1.75+ (2021 edition)
- **Platform**: Linux, macOS, Windows
- **Dependencies**: See `Cargo.toml` (all vendored)

## Usage

### CLI Examples

```bash
# Display disk image information
totalimage-cli info disk.img

# List partitions
totalimage-cli zones disk.img

# List files in first partition
totalimage-cli list disk.img --zone 0

# Extract file from partition
totalimage-cli extract disk.img README.TXT --zone 0 --output readme.txt
```

### Web API

```bash
# Start web server
totalimage-web
# Listening on http://127.0.0.1:3000

# Query vault info
curl 'http://localhost:3000/api/vault/info?path=/path/to/disk.img'

# List zones
curl 'http://localhost:3000/api/vault/zones?path=/path/to/disk.img'
```

### Environment Variables

- `TOTALIMAGE_CACHE_DIR`: Cache directory (default: `~/.cache/totalimage`)
- `RUST_LOG`: Logging level (e.g., `info`, `debug`)

## Architecture

TotalImage uses a modular architecture with clear separation of concerns:

```
totalimage-core       # Traits and error types
totalimage-pipeline   # I/O abstractions (mmap, streaming)
totalimage-vaults     # Container format parsers (VHD, raw)
totalimage-zones      # Partition table parsers (MBR, GPT)
totalimage-territories # Filesystem parsers (FAT, ISO)
totalimage-cli        # Command-line interface
totalimage-web        # REST API server
```

### Anarchist Terminology

- **Vault** = Container format (sabotage proprietary formats)
- **Territory** = Filesystem (autonomous data domain)
- **Zone** = Partition (segregated storage area)
- **Cell** = Component/Module
- **Liberation** = Data extraction
- **Direct Action** = Memory-mapped I/O

## Development

### Building

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Check without building
cargo check --all-targets

# Run clippy lints
cargo clippy -- -D warnings
```

### Testing

```bash
# Run all tests
cargo test --all-targets

# Run specific crate tests
cargo test -p totalimage-zones

# Run with output
cargo test -- --nocapture
```

### Code Quality

- **Zero compiler warnings** ✅
- **94 passing tests** (100% pass rate)
- **Documented TODOs** for future work
- **Comprehensive error handling**

## Roadmap

### Phase 5 (Future)

- [ ] Additional filesystem support (NTFS, ext2/3/4)
- [ ] Write support (currently read-only)
- [ ] GUI application (egui or iced)
- [ ] Additional container formats (VMDK, QCOW2)
- [ ] Web API rate limiting and production hardening
- [ ] MCP server integration for PYRO Platform
- [ ] Standalone executable packaging

See [TODO markers](grep -r "TODO:" crates/) in code for detailed future work items.

## Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) (TODO) for guidelines.

### Areas for Contribution

- Additional filesystem support
- Performance optimizations
- Documentation improvements
- Test coverage expansion
- Security auditing

## Security Disclosure

Found a security vulnerability? Please see [SECURITY.md](SECURITY.md) for responsible disclosure process.

**Do not** report security issues via public GitHub issues.

## License

TotalImage is licensed under the MIT License. See [LICENSE](LICENSE) for details.

## Acknowledgments

- Original C# TotalImage project inspiration
- Rust community for excellent tooling
- Security review contributors

---

**Status**: Alpha (v0.1.0) - Core functionality complete, hardened for security
**Rust Version**: 1.75+ (2021 edition)
**Last Updated**: 2025-11-24
