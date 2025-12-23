# TotalImage Project - Pseudocode Documentation Index

**Version:** 1.0.0  
**Date:** December 21, 2025  
**Purpose:** Master index for all pseudocode documentation with code references

---

## Quick Navigation

| Document | Component | Lines | Status |
|----------|-----------|-------|--------|
| [00-INDEX.md](00-INDEX.md) | Master Index | 300+ | ✅ Complete |
| [01-core.md](01-core.md) | Core Traits & Types | 500+ | ✅ Complete |
| [02-vaults.md](02-vaults.md) | Container Formats | 800+ | ✅ Complete |
| [03-zones.md](03-zones.md) | Partition Tables | 600+ | ✅ Complete |
| [04-territories.md](04-territories.md) | Filesystems | 1000+ | ✅ Complete |
| [05-pipeline.md](05-pipeline.md) | I/O Abstractions | 400+ | ✅ Complete |
| [06-acquire.md](06-acquire.md) | Image Acquisition | 700+ | ✅ Complete |
| [07-mcp-server.md](07-mcp-server.md) | MCP Server | 900+ | ✅ Complete |
| [08-fire-marshal.md](08-fire-marshal.md) | Fire Marshal API | 1700+ | ✅ Complete |
| [09-web-api.md](09-web-api.md) | Web REST API | 250+ | ✅ Complete |
| [10-cli.md](10-cli.md) | CLI Interface | 370+ | ✅ Complete |

**Total:** 5,302+ lines of pseudocode documentation

---

## Code Reference System

### Format
```
[Component Name] → File:Line
Example: Vault trait → crates/totalimage-core/src/traits.rs:10
```

### Search Tips
- Use `Ctrl+F` to search within documents
- Line numbers reference actual source code
- Cross-references use `[Document Name](#section)` format
- Code blocks show pseudocode, not actual implementation

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    TotalImage Project                        │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │    Core      │  │   Pipeline   │  │   Security  │      │
│  │  [01-core]   │  │  [05-pipeline]│  │  [01-core]  │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│         │                  │                  │              │
│         └──────────────────┼──────────────────┘            │
│                            │                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │    Vaults    │  │    Zones     │  │ Territories  │      │
│  │  [02-vaults] │  │  [03-zones] │  │ [04-territories]│    │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│         │                  │                  │              │
│         └──────────────────┼──────────────────┘            │
│                            │                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │   Acquire    │  │  MCP Server  │  │ Fire Marshal │      │
│  │ [06-acquire] │  │ [07-mcp-server]│ │[08-fire-marshal]│  │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│         │                  │                  │              │
│         └──────────────────┼──────────────────┘            │
│                            │                                 │
│  ┌──────────────┐  ┌──────────────┐                        │
│  │  Web API     │  │     CLI      │                        │
│  │  [09-web-api]│  │  [10-cli]    │                        │
│  └──────────────┘  └──────────────┘                        │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

## Document Structure

Each pseudocode document follows this structure:

1. **Overview** - Component purpose and responsibilities
2. **Data Structures** - Types, structs, enums with code references
3. **Core Functions** - Main operations with pseudocode
4. **Algorithms** - Complex logic explained step-by-step
5. **Error Handling** - Error cases and recovery
6. **Security Considerations** - Security measures
7. **Code References** - Links to actual implementation

---

## Component Dependencies

```
Core (01)
  ├── Used by: All components
  └── Provides: Traits, Error types, Security utilities

Vaults (02)
  ├── Depends on: Core (01), Pipeline (05)
  └── Provides: Container format access

Zones (03)
  ├── Depends on: Core (01), Pipeline (05)
  └── Provides: Partition table parsing

Territories (04)
  ├── Depends on: Core (01), Pipeline (05)
  └── Provides: Filesystem parsing

Pipeline (05)
  ├── Depends on: Core (01)
  └── Provides: I/O abstractions

Acquire (06)
  ├── Depends on: Core (01), Vaults (02), Zones (03), Territories (04)
  └── Provides: Image creation and writing

MCP Server (07)
  ├── Depends on: All above
  └── Provides: Claude Desktop integration

Fire Marshal (08)
  ├── Depends on: Core (01), MCP Server (07)
  └── Provides: Tool orchestration

Web API (09)
  ├── Depends on: All above
  └── Provides: REST API

CLI (10)
  ├── Depends on: All above
  └── Provides: Command-line interface
```

---

## Key Terminology

See [CRYPTEX-DICTIONARY.md](../steering/CRYPTEX-DICTIONARY.md) for complete terminology.

| Term | Meaning | Code Reference |
|------|---------|----------------|
| **Vault** | Container format | `crates/totalimage-core/src/traits.rs:10` |
| **Territory** | Filesystem | `crates/totalimage-core/src/traits.rs:36` |
| **Zone** | Partition | `crates/totalimage-core/src/types.rs:121` |
| **Cell** | Directory | `crates/totalimage-core/src/traits.rs:73` |
| **Occupant** | File/Directory entry | `crates/totalimage-core/src/types.rs:9` |

---

## Implementation Status

| Component | Implementation | Tests | Documentation |
|-----------|----------------|-------|---------------|
| Core | ✅ Complete | ✅ 80+ tests | ✅ Complete |
| Vaults | ✅ Complete | ✅ 105+ tests | ✅ Complete |
| Zones | ✅ Complete | ✅ 43+ tests | ✅ Complete |
| Territories | ✅ Complete | ✅ 82+ tests | ✅ Complete |
| Pipeline | ✅ Complete | ✅ 9+ tests | ✅ Complete |
| Acquire | ✅ Complete | ✅ 15+ tests | ✅ Complete |
| MCP Server | ✅ Complete | ✅ 54+ tests | ✅ Complete |
| Fire Marshal | ✅ Complete | ✅ 8+ tests | ✅ Complete |
| Web API | ✅ Complete | ✅ 8+ tests | ✅ Complete |
| CLI | ✅ Complete | ✅ 2+ tests | ✅ Complete |

---

## How to Use This Documentation

### For Developers
1. Start with [01-core.md](01-core.md) to understand foundational concepts
2. Read component-specific documents as needed
3. Use code references to jump to actual implementation
4. Follow cross-references between documents

### For Code Review
1. Check pseudocode matches actual implementation
2. Verify code references are accurate
3. Ensure security considerations are addressed
4. Validate error handling matches pseudocode

### For New Features
1. Add pseudocode to relevant document
2. Update index with new sections
3. Add code references when implementation complete
4. Cross-reference related components

---

## Document Maintenance

### Update Frequency
- **After major refactoring**: Update affected documents
- **After new features**: Add pseudocode sections
- **After bug fixes**: Update error handling sections
- **Monthly**: Review and update code references

### Version Control
- Each document has version number
- Changes tracked in git
- Major changes require version bump

---

## Quick Reference Links

### Core Concepts
- [Vault Trait](01-core.md#vault-trait) → `crates/totalimage-core/src/traits.rs:10`
- [Territory Trait](01-core.md#territory-trait) → `crates/totalimage-core/src/traits.rs:36`
- [ZoneTable Trait](01-core.md#zonetable-trait) → `crates/totalimage-core/src/traits.rs:22`
- [Error Types](01-core.md#error-handling) → `crates/totalimage-core/src/error.rs:7`

### Container Formats
- [Raw Vault](02-vaults.md#raw-vault) → `crates/totalimage-vaults/src/raw.rs`
- [VHD Vault](02-vaults.md#vhd-vault) → `crates/totalimage-vaults/src/vhd/mod.rs`
- [E01 Vault](02-vaults.md#e01-vault) → `crates/totalimage-vaults/src/e01/mod.rs`
- [AFF4 Vault](02-vaults.md#aff4-vault) → `crates/totalimage-vaults/src/aff4/mod.rs`

### Partition Tables
- [MBR Parser](03-zones.md#mbr-parser) → `crates/totalimage-zones/src/mbr/mod.rs`
- [GPT Parser](03-zones.md#gpt-parser) → `crates/totalimage-zones/src/gpt/mod.rs`

### Filesystems
- [FAT Parser](04-territories.md#fat-parser) → `crates/totalimage-territories/src/fat/mod.rs`
- [exFAT Parser](04-territories.md#exfat-parser) → `crates/totalimage-territories/src/exfat/mod.rs`
- [ISO Parser](04-territories.md#iso-parser) → `crates/totalimage-territories/src/iso/mod.rs`
- [NTFS Parser](04-territories.md#ntfs-parser) → `crates/totalimage-territories/src/ntfs/mod.rs`

### APIs
- [MCP Server](07-mcp-server.md) → `crates/totalimage-mcp/src/`
- [Fire Marshal API](08-fire-marshal.md) → `crates/fire-marshal/src/`
- [Web REST API](09-web-api.md) → `crates/totalimage-web/src/`

---

**Last Updated:** December 21, 2025  
**Maintainer:** TotalImage Development Team  
**Version:** 1.0.0
