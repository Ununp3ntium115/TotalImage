# CRYPTEX-DICTIONARY: TotalImage Liberation Project

## Anarchist Terminology Framework

This cryptex-dictionary uses revolutionary terminology to document the complete architecture of TotalImage for conversion to Rust/redb/Svelte.

### Core Terminology

| Actual Name | Brand Name (Anarchist) | Concept |
|-------------|------------------------|---------|
| Component | **Cell** | Independent functional unit |
| Class/Module | **Collective** | Group of related operations |
| Method/Function | **Action** | Executable operation |
| Container Format | **Vault** | Encapsulated storage format |
| File System | **Territory** | Organized data domain |
| Partition | **Zone** | Segregated storage area |
| Read Operation | **Sabotage** | Extract data from proprietary formats |
| Write Operation | **Propaganda** | Inject data into structures |
| Extract | **Liberation** | Free data from containers |
| Parse | **Decrypt** | Decode structure |
| Factory Pattern | **Underground Network** | Discovery and instantiation system |
| Stream | **Pipeline** | Data flow channel |
| UI Layer | **Front** | Public-facing interface |
| Core Library | **Arsenal** | Core capabilities |
| Detection | **Reconnaissance** | Identify format/structure |
| Boot Sector | **Manifesto** | System declaration |
| Dependencies | **Solidarity** | Inter-cell cooperation |
| Entry Point | **Ignition** | System activation |
| Memory-mapped | **Direct Action** | Immediate access to resources |

---

## Architecture Overview

**Project Codename:** TOTAL-LIBERATION
**Current State:** C# .NET 8.0 Windows Forms
**Target State:** Rust + redb + Svelte

### Layer Structure

```
┌─────────────────────────────────────┐
│  FRONT (UI Layer)                   │  ← Svelte Web Interface
│  Windows Forms → Svelte Components  │
└─────────────────────────────────────┘
           ↓ Solidarity ↓
┌─────────────────────────────────────┐
│  ARSENAL (Core Library)             │  ← Rust + redb
│  TotalImage.IO → Rust Crates        │
├─────────────────────────────────────┤
│  ├─ Vaults (Containers)             │
│  ├─ Territories (FileSystems)       │
│  ├─ Zones (Partitions)              │
│  └─ Pipelines (I/O)                 │
└─────────────────────────────────────┘
```

---

## Cell Registry

### Active Cells (Components)

1. **Vault Cells** - Container format handlers
2. **Territory Cells** - File system implementations
3. **Zone Cells** - Partition table parsers
4. **Front Cells** - UI components
5. **Pipeline Cells** - Stream/I/O operations
6. **Reconnaissance Cells** - Format detection
7. **Liberation Cells** - Data extraction

---

## Directory Structure

```
/steering/
├── CRYPTEX-DICTIONARY.md          # This master index
├── cells/                         # Component documentation
│   ├── vaults/                   # Container cells
│   ├── territories/              # FileSystem cells
│   ├── zones/                    # Partition cells
│   └── front/                    # UI cells
├── manifests/                    # System specifications
│   ├── boot-sector-manifesto.md
│   └── format-specifications.md
├── operations/                   # Pseudocode for actions
│   ├── sabotage-ops.md          # Read operations
│   ├── propaganda-ops.md        # Write operations
│   └── liberation-ops.md        # Extract operations
└── infrastructure/               # Rust conversion specs
    ├── crate-structure.md
    ├── redb-schema.md
    └── svelte-components.md
```

---

## Conversion Strategy

### Phase 1: Reconnaissance (Current)
- Document all C# components
- Map dependencies
- Create pseudocode specifications

### Phase 2: Arsenal Construction
- Build Rust crates mirroring cell structure
- Implement redb schemas for metadata
- Create trait-based architecture

### Phase 3: Front Construction
- Design Svelte component tree
- Implement Web API layer
- Build reactive UI

### Phase 4: Liberation
- Port core functionality
- Maintain format compatibility
- Deploy autonomous system

---

**Status:** Phase 1 - Reconnaissance Active
**Next Actions:** Document all cells with pseudocode
