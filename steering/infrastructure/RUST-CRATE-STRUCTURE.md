# RUST CRATE STRUCTURE: Arsenal Organization

**Codename:** Autonomous Arsenal
**Purpose:** Organize Rust implementation into modular, autonomous crates
**Architecture:** Workspace with multiple sub-crates

---

## Workspace Structure

```
total-liberation/
├── Cargo.toml                    # Workspace root
├── README.md                     # Liberation manifesto
├── LICENSE                       # GNU GPL v3 (or similar)
│
├── crates/
│   ├── totalimage-core/          # Core traits and types
│   ├── totalimage-vaults/        # Container format handlers
│   ├── totalimage-territories/   # File system implementations
│   ├── totalimage-zones/         # Partition table handlers
│   ├── totalimage-pipeline/      # I/O abstractions
│   ├── totalimage-cli/           # Command-line interface
│   ├── totalimage-web/           # Web server + API
│   └── totalimage-gui/           # Tauri desktop app (optional)
│
├── web/                          # Svelte frontend
│   ├── src/
│   ├── public/
│   ├── package.json
│   └── vite.config.ts
│
├── docs/                         # Documentation
│   └── specifications/           # Format specs
│
└── tests/                        # Integration tests
    ├── vaults/
    ├── territories/
    └── end-to-end/
```

---

## Workspace Cargo.toml

```toml
[workspace]
members = [
    "crates/totalimage-core",
    "crates/totalimage-pipeline",
    "crates/totalimage-vaults",
    "crates/totalimage-territories",
    "crates/totalimage-zones",
    "crates/totalimage-cli",
    "crates/totalimage-web",
]

resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
rust-version = "1.75"
license = "GPL-3.0-or-later"
authors = ["Total Liberation Collective"]
repository = "https://github.com/liberation/total-image"

[workspace.dependencies]
# SHARED DEPENDENCIES
thiserror = "1.0"
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"

# ASYNC
tokio = { version = "1.35", features = ["full"] }
async-trait = "0.1"

# SERIALIZATION
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# I/O
bytes = "1.5"
memmap2 = "0.9"

# CRYPTO
md5 = "0.7"
sha1 = "0.10"
uuid = { version = "1.6", features = ["v4", "serde"] }

# ENCODING
encoding_rs = "0.8"
unicode-normalization = "0.1"

# WEB
axum = { version = "0.7", features = ["multipart"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["fs", "cors"] }

# DATABASE
redb = "2.1"

# DATETIME
chrono = { version = "0.4", features = ["serde"] }
```

---

## Crate 1: totalimage-core

**Purpose:** Core traits, types, and error definitions

### Cargo.toml
```toml
[package]
name = "totalimage-core"
version.workspace = true
edition.workspace = true

[dependencies]
thiserror.workspace = true
serde.workspace = true
async-trait.workspace = true
```

### Structure
```
totalimage-core/
├── src/
│   ├── lib.rs
│   ├── error.rs              # Liberation errors
│   ├── traits/
│   │   ├── mod.rs
│   │   ├── vault.rs          # Vault trait
│   │   ├── territory.rs      # Territory trait
│   │   ├── zone_table.rs     # ZoneTable trait
│   │   └── pipeline.rs       # ReadSeek trait extensions
│   ├── types/
│   │   ├── mod.rs
│   │   ├── occupant.rs       # OccupantInfo
│   │   ├── zone.rs           # Zone struct
│   │   └── manifesto.rs      # Common manifesto types
│   └── utils/
│       ├── mod.rs
│       ├── encoding.rs       # Character encoding utilities
│       └── checksum.rs       # Hash/checksum helpers
```

### Key Exports

```rust
// lib.rs
pub mod error;
pub mod traits;
pub mod types;
pub mod utils;

pub use error::{Error, Result};
pub use traits::{Vault, Territory, ZoneTable, DirectoryCell};
pub use types::{OccupantInfo, Zone};
```

---

## Crate 2: totalimage-pipeline

**Purpose:** I/O abstractions and pipeline utilities

### Cargo.toml
```toml
[package]
name = "totalimage-pipeline"
version.workspace = true
edition.workspace = true

[dependencies]
totalimage-core = { path = "../totalimage-core" }
bytes.workspace = true
memmap2.workspace = true
thiserror.workspace = true
```

### Structure
```
totalimage-pipeline/
├── src/
│   ├── lib.rs
│   ├── partial.rs            # PartialPipeline (stream subset)
│   ├── buffered.rs           # BufferedPipeline
│   ├── memory.rs             # MemoryPipeline
│   └── mmap.rs               # MmapPipeline (memory-mapped)
```

### Key Types

```rust
// partial.rs
pub struct PartialPipeline<R: Read + Seek> {
    inner: R,
    start: u64,
    length: u64,
    position: u64,
}

// mmap.rs
pub struct MmapPipeline {
    mmap: Mmap,
    position: u64,
}
```

---

## Crate 3: totalimage-vaults

**Purpose:** Container format implementations

### Cargo.toml
```toml
[package]
name = "totalimage-vaults"
version.workspace = true
edition.workspace = true

[dependencies]
totalimage-core = { path = "../totalimage-core" }
totalimage-pipeline = { path = "../totalimage-pipeline" }
md5.workspace = true
sha1.workspace = true
uuid.workspace = true
serde.workspace = true
thiserror.workspace = true
```

### Structure
```
totalimage-vaults/
├── src/
│   ├── lib.rs
│   ├── factory.rs            # VaultFactory trait + network
│   ├── raw/
│   │   ├── mod.rs
│   │   └── vault.rs          # RawVault
│   ├── vhd/
│   │   ├── mod.rs
│   │   ├── vault.rs          # MicrosoftVault
│   │   ├── footer.rs         # VHD footer structure
│   │   ├── dynamic.rs        # Dynamic header + BAT
│   │   └── stream.rs         # VHD dynamic stream
│   ├── nhd/
│   │   ├── mod.rs
│   │   └── vault.rs          # NhdVault
│   ├── imz/
│   │   ├── mod.rs
│   │   └── vault.rs          # ImzVault
│   ├── anex86/
│   │   ├── mod.rs
│   │   └── vault.rs          # Anex86Vault
│   └── pcjs/
│       ├── mod.rs
│       └── vault.rs          # PCjsVault
```

---

## Crate 4: totalimage-territories

**Purpose:** File system implementations

### Cargo.toml
```toml
[package]
name = "totalimage-territories"
version.workspace = true
edition.workspace = true

[dependencies]
totalimage-core = { path = "../totalimage-core" }
totalimage-pipeline = { path = "../totalimage-pipeline" }
encoding_rs.workspace = true
unicode-normalization.workspace = true
chrono.workspace = true
serde.workspace = true
thiserror.workspace = true
```

### Structure
```
totalimage-territories/
├── src/
│   ├── lib.rs
│   ├── factory.rs            # TerritoryFactory trait + network
│   ├── fat/
│   │   ├── mod.rs
│   │   ├── territory.rs      # FatTerritory
│   │   ├── bpb.rs            # BIOS Parameter Block
│   │   ├── fat12.rs          # FAT12 specifics
│   │   ├── fat16.rs          # FAT16 specifics
│   │   ├── fat32.rs          # FAT32 specifics
│   │   ├── directory.rs      # FatDirectoryCell
│   │   ├── entry.rs          # Directory entry parsing
│   │   └── lfn.rs            # Long file name support
│   ├── iso/
│   │   ├── mod.rs
│   │   ├── territory.rs      # IsoTerritory
│   │   ├── volume_descriptor.rs
│   │   ├── directory.rs      # IsoDirectoryCell
│   │   ├── joliet.rs         # Joliet extensions
│   │   └── high_sierra.rs    # High Sierra format
│   ├── exfat/
│   │   ├── mod.rs
│   │   ├── territory.rs      # ExFatTerritory
│   │   ├── boot_sector.rs
│   │   └── directory.rs
│   └── raw/
│       ├── mod.rs
│       └── territory.rs      # RawTerritory (fallback)
```

---

## Crate 5: totalimage-zones

**Purpose:** Partition table implementations

### Cargo.toml
```toml
[package]
name = "totalimage-zones"
version.workspace = true
edition.workspace = true

[dependencies]
totalimage-core = { path = "../totalimage-core" }
totalimage-pipeline = { path = "../totalimage-pipeline" }
uuid.workspace = true
serde.workspace = true
thiserror.workspace = true
```

### Structure
```
totalimage-zones/
├── src/
│   ├── lib.rs
│   ├── factory.rs            # ZoneTableFactory trait + network
│   ├── mbr/
│   │   ├── mod.rs
│   │   ├── zone_table.rs     # MbrZoneTable
│   │   ├── types.rs          # MbrPartitionType enum
│   │   └── chs.rs            # CHS addressing
│   ├── gpt/
│   │   ├── mod.rs
│   │   ├── zone_table.rs     # GptZoneTable
│   │   ├── header.rs         # GPT header structure
│   │   └── guids.rs          # Known partition type GUIDs
│   └── direct/
│       ├── mod.rs
│       └── zone_table.rs     # DirectTerritory (no partitions)
```

---

## Crate 6: totalimage-cli

**Purpose:** Command-line interface

### Cargo.toml
```toml
[package]
name = "totalimage-cli"
version.workspace = true
edition.workspace = true

[[bin]]
name = "totalimage"
path = "src/main.rs"

[dependencies]
totalimage-core = { path = "../totalimage-core" }
totalimage-vaults = { path = "../totalimage-vaults" }
totalimage-territories = { path = "../totalimage-territories" }
totalimage-zones = { path = "../totalimage-zones" }
clap = { version = "4.4", features = ["derive"] }
tracing.workspace = true
tracing-subscriber.workspace = true
anyhow.workspace = true
```

### Structure
```
totalimage-cli/
├── src/
│   ├── main.rs
│   ├── commands/
│   │   ├── mod.rs
│   │   ├── open.rs           # Open vault
│   │   ├── info.rs           # Display info
│   │   ├── list.rs           # List files
│   │   ├── extract.rs        # Extract files
│   │   └── hash.rs           # Calculate hashes
│   └── output/
│       ├── mod.rs
│       └── formatter.rs      # Output formatting
```

### CLI Interface

```bash
# Open vault and show info
totalimage open disk.img

# List zones
totalimage zones disk.img

# List files in partition 0
totalimage list disk.img --zone 0

# Extract file
totalimage extract disk.img --zone 0 --file /path/to/file.txt --output ./

# Calculate MD5
totalimage hash disk.img --md5

# Batch extract
totalimage extract disk.img --zone 0 --all --output ./extracted/
```

---

## Crate 7: totalimage-web

**Purpose:** Web server with REST API

### Cargo.toml
```toml
[package]
name = "totalimage-web"
version.workspace = true
edition.workspace = true

[[bin]]
name = "totalimage-web"
path = "src/main.rs"

[dependencies]
totalimage-core = { path = "../totalimage-core" }
totalimage-vaults = { path = "../totalimage-vaults" }
totalimage-territories = { path = "../totalimage-territories" }
totalimage-zones = { path = "../totalimage-zones" }
axum.workspace = true
tokio.workspace = true
tower.workspace = true
tower-http.workspace = true
serde.workspace = true
serde_json.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true
redb.workspace = true
```

### Structure
```
totalimage-web/
├── src/
│   ├── main.rs
│   ├── api/
│   │   ├── mod.rs
│   │   ├── vault.rs          # Vault endpoints
│   │   ├── territory.rs      # Territory endpoints
│   │   ├── zone.rs           # Zone endpoints
│   │   └── file.rs           # File endpoints
│   ├── state/
│   │   ├── mod.rs
│   │   ├── registry.rs       # VaultRegistry
│   │   └── session.rs        # SessionState
│   ├── cache/
│   │   ├── mod.rs
│   │   └── metadata.rs       # redb-backed metadata cache
│   └── error.rs              # API error responses
```

---

## Build Configuration

### Root Cargo.toml profiles

```toml
[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
strip = true

[profile.dev]
opt-level = 0
debug = true

[profile.dev.package."*"]
opt-level = 2  # Optimize dependencies even in dev
```

---

## Testing Strategy

### Unit Tests
- Each crate has `src/tests/` modules
- Test individual components in isolation

### Integration Tests
```
tests/
├── vaults/
│   ├── test_raw.rs
│   ├── test_vhd.rs
│   └── fixtures/
│       ├── test.img
│       └── test.vhd
├── territories/
│   ├── test_fat12.rs
│   ├── test_iso9660.rs
│   └── fixtures/
└── end_to_end/
    └── test_full_flow.rs
```

### Test Execution
```bash
# Run all tests
cargo test --workspace

# Run with sample images
cargo test --workspace -- --include-ignored

# Benchmark
cargo bench --workspace
```

---

## Documentation

### Cargo Doc
```bash
# Generate documentation
cargo doc --workspace --no-deps --open

# Document private items
cargo doc --workspace --document-private-items
```

### README Structure
```
README.md
├── Introduction
├── Installation
├── Quick Start
├── Architecture Overview
├── CLI Usage
├── Web API Documentation
├── Contributing
└── License
```

---

## CI/CD Pipeline

### GitHub Actions
```yaml
name: Liberation CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --workspace
      - run: cargo clippy --workspace -- -D warnings
      - run: cargo fmt --check

  build-web:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 20
      - run: cd web && npm install && npm run build
```

---

## Status

- ✅ Workspace structure designed
- ✅ Crate organization complete
- ✅ Dependencies specified
- ✅ CLI structure defined
- ✅ Web API structure defined
- ⏳ Implementation pending
- ⏳ Testing framework pending

**Next:** redb schema design for metadata caching
