# TotalImage Pseudocode Documentation

**Location:** `pseudocode/`  
**Purpose:** Comprehensive pseudocode documentation for entire TotalImage project  
**Total Lines:** 5,302+ lines across 11 documents

---

## Quick Start

1. **Start Here:** [00-INDEX.md](00-INDEX.md) - Master index with navigation
2. **Core Concepts:** [01-core.md](01-core.md) - Foundation traits and types
3. **Component Docs:** [02-vaults.md](02-vaults.md) through [10-cli.md](10-cli.md)

---

## Document Organization

### Core Components
- **[01-core.md](01-core.md)** - Core traits, error types, security utilities
- **[05-pipeline.md](05-pipeline.md)** - I/O abstractions (PartialPipeline, MmapPipeline)

### Format Parsers
- **[02-vaults.md](02-vaults.md)** - Container formats (Raw, VHD, E01, AFF4)
- **[03-zones.md](03-zones.md)** - Partition tables (MBR, GPT)
- **[04-territories.md](04-territories.md)** - Filesystems (FAT, exFAT, ISO, NTFS)

### Tools & Interfaces
- **[06-acquire.md](06-acquire.md)** - Image acquisition (E01 writer, WinPE USB)
- **[07-mcp-server.md](07-mcp-server.md)** - MCP server for Claude Desktop
- **[08-fire-marshal.md](08-fire-marshal.md)** - Fire Marshal API (orchestration)
- **[09-web-api.md](09-web-api.md)** - Web REST API
- **[10-cli.md](10-cli.md)** - Command-line interface

---

## Code Reference System

### Format
Each pseudocode section includes code references in this format:
```
[Component Name] → File:Line
Example: Vault trait → crates/totalimage-core/src/traits.rs:10
```

### Search Tips
- Use `Ctrl+F` / `Cmd+F` to search within documents
- Line numbers reference actual source code
- Cross-references use `[Document Name](#section)` format
- Code blocks show pseudocode, not actual implementation

---

## Navigation

### By Component
- **Core Library:** [01-core.md](01-core.md)
- **Vaults:** [02-vaults.md](02-vaults.md)
- **Zones:** [03-zones.md](03-zones.md)
- **Territories:** [04-territories.md](04-territories.md)
- **Pipeline:** [05-pipeline.md](05-pipeline.md)
- **Acquire:** [06-acquire.md](06-acquire.md)
- **MCP Server:** [07-mcp-server.md](07-mcp-server.md)
- **Fire Marshal:** [08-fire-marshal.md](08-fire-marshal.md)
- **Web API:** [09-web-api.md](09-web-api.md)
- **CLI:** [10-cli.md](10-cli.md)

### By Task
- **Understanding Architecture:** Start with [00-INDEX.md](00-INDEX.md), then [01-core.md](01-core.md)
- **Adding New Format:** See [02-vaults.md](02-vaults.md) for vault patterns
- **Adding New Filesystem:** See [04-territories.md](04-territories.md) for territory patterns
- **API Development:** See [08-fire-marshal.md](08-fire-marshal.md) for API best practices
- **Tool Integration:** See [07-mcp-server.md](07-mcp-server.md) for tool patterns

---

## Document Features

### Each Document Includes:
1. **Overview** - Component purpose and responsibilities
2. **Data Structures** - Types, structs, enums with code references
3. **Core Functions** - Main operations with pseudocode
4. **Algorithms** - Complex logic explained step-by-step
5. **Error Handling** - Error cases and recovery
6. **Security Considerations** - Security measures
7. **Code References** - Links to actual implementation files and line numbers

### Code Reference Format:
```
FUNCTION_NAME → crates/component/src/file.rs:LINE_NUMBER
```

---

## Usage Examples

### Finding Implementation Details

**Question:** "How does FAT parsing work?"

1. Go to [04-territories.md](04-territories.md)
2. Find "FAT Parser" section
3. Read pseudocode for `FatTerritory::parse`
4. Follow code reference: `crates/totalimage-territories/src/fat/mod.rs:35-83`
5. Open actual code file at that location

### Understanding Data Flow

**Question:** "How does a disk image get analyzed?"

1. Start with [02-vaults.md](02-vaults.md) - Vault opening
2. Follow to [03-zones.md](03-zones.md) - Zone parsing
3. Continue to [04-territories.md](04-territories.md) - Territory parsing
4. See [07-mcp-server.md](07-mcp-server.md) - Tool orchestration

### Adding New Features

**Task:** "Add support for new container format"

1. Review [02-vaults.md](02-vaults.md) - Vault implementation pattern
2. Check [01-core.md](01-core.md) - Vault trait requirements
3. Follow pseudocode structure for new vault type
4. Reference existing implementations (Raw, VHD, E01)

---

## Maintenance

### When to Update
- **After major refactoring:** Update affected documents
- **After new features:** Add pseudocode sections
- **After bug fixes:** Update error handling sections
- **Monthly:** Review and update code references

### Version Control
- Each document has version number
- Changes tracked in git
- Major changes require version bump

---

## Statistics

- **Total Documents:** 11
- **Total Lines:** 5,302+
- **Code References:** 200+
- **Coverage:** All major components
- **Status:** ✅ Complete and ready for implementation

---

## Related Documentation

- **[CLAUDE.md](../CLAUDE.md)** - Project overview and build commands
- **[steering/CRYPTEX-DICTIONARY.md](../steering/CRYPTEX-DICTIONARY.md)** - Terminology reference
- **[steering/PYRO-INTEGRATION-DESIGN.md](../steering/PYRO-INTEGRATION-DESIGN.md)** - PYRO integration
- **[Docs/ARCHITECTURE.md](../Docs/ARCHITECTURE.md)** - System architecture

---

**Last Updated:** December 21, 2025  
**Maintainer:** TotalImage Development Team  
**Version:** 1.0.0
