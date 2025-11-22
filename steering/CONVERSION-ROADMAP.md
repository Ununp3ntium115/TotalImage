# CONVERSION ROADMAP: C# → Rust/redb/Svelte

**Project:** Total Liberation
**Objective:** Transform TotalImage from C# Windows Forms to Rust/redb/Svelte web application
**Status:** Phase 1 - Reconnaissance Complete

---

## Executive Summary

TotalImage is a disk image editor written in C# with Windows Forms UI. This roadmap outlines the complete conversion to a modern, autonomous stack:

- **Backend:** Rust (performance, safety, portability)
- **Database:** redb (embedded metadata caching)
- **Frontend:** Svelte (reactive web UI)

**Current Codebase:**
- 162 C# files
- 2 projects (UI + Core Library)
- ~20,000 lines of code
- Read-only support for FAT12/16/32, exFAT, ISO-9660
- Container formats: Raw, VHD, NHD, IMZ, Anex86, PCjs

---

## Phase Breakdown

### Phase 1: Reconnaissance ✅ COMPLETE

**Objective:** Document entire C# codebase with anarchist-themed terminology

**Deliverables:**
- ✅ Cryptex-Dictionary master index
- ✅ Vault Collective documentation (Container layer)
- ✅ Territory Collective documentation (FileSystem layer)
- ✅ Zone Collective documentation (Partition layer)
- ✅ Front Collective documentation (UI layer)
- ✅ Rust crate structure specification
- ✅ redb schema design

**Location:** `/steering/` directory

---

### Phase 2: Arsenal Foundation (Weeks 1-4)

**Objective:** Build core Rust infrastructure

#### Week 1: Core Traits & Pipeline
```bash
# Tasks
- Create workspace structure
- Implement totalimage-core crate
  - Error types
  - Vault, Territory, ZoneTable traits
  - Core type definitions
- Implement totalimage-pipeline crate
  - PartialPipeline
  - MmapPipeline
  - BufferedPipeline
```

**Files to Create:**
```
crates/totalimage-core/src/
├── lib.rs
├── error.rs
├── traits/
│   ├── vault.rs
│   ├── territory.rs
│   └── zone_table.rs
└── types/
    ├── occupant.rs
    └── zone.rs

crates/totalimage-pipeline/src/
├── lib.rs
├── partial.rs
├── mmap.rs
└── buffered.rs
```

**Test Coverage:**
- Unit tests for each pipeline type
- Benchmark against C# Stream performance

---

#### Week 2: Vault Collective Implementation

```bash
# Tasks
- Implement totalimage-vaults crate
  - RawVault (priority 1)
  - MicrosoftVault/VHD (priority 2)
  - Factory pattern + network
- Create test fixtures
- Port C# container tests
```

**Implementation Order:**
1. **RawVault** (simplest, most common)
   - Direct passthrough to file
   - Manufacturing capability (create blank images)
2. **MicrosoftVault** (VHD)
   - Fixed disk support
   - Dynamic disk support
   - VHD stream implementation
3. **Remaining Vaults** (lower priority)
   - NhdVault, ImzVault, Anex86Vault, PCjsVault

**Test Images:** Use existing test images from C# project

---

#### Week 3: Zone Collective Implementation

```bash
# Tasks
- Implement totalimage-zones crate
  - MbrZoneTable
  - GptZoneTable
  - DirectTerritory (no partitions)
  - MBR/GPT factory
- Test with various partition layouts
```

**Test Cases:**
- MBR with 1-4 partitions
- GPT with varying partition counts
- Protective MBR + GPT
- Direct territory (floppy images)

---

#### Week 4: Territory Collective - FAT Implementation

```bash
# Tasks
- Implement totalimage-territories crate
  - FAT12Territory (priority 1)
  - FAT16Territory (priority 2)
  - FAT32Territory (priority 3)
  - BPB parsing
  - Cluster chain tracing
  - Directory enumeration
  - File extraction
```

**FAT Implementation Priority:**
1. **FAT12** (floppy disks, most common in retro computing)
2. **FAT16** (hard disks, MS-DOS)
3. **FAT32** (modern USB drives, large volumes)

**Test Cases:**
- Read boot sector
- Parse FAT tables
- Enumerate root directory
- Navigate subdirectories
- Extract files with correct content
- Long filename (LFN) support

---

### Phase 3: Extended Territory Support (Weeks 5-6)

#### Week 5: ISO-9660 Territory

```bash
# Tasks
- Implement IsoTerritory
  - Volume descriptor parsing
  - Primary/supplementary descriptors
  - Directory record parsing
  - Joliet extension support
  - High Sierra format support
```

**Test Cases:**
- Standard ISO-9660
- Joliet (Unicode names)
- High Sierra format
- Multi-session discs

---

#### Week 6: exFAT and Raw Territories

```bash
# Tasks
- Implement ExFatTerritory
  - Boot sector parsing
  - Bitmap allocation table
  - Directory entry parsing
- Implement RawTerritory (fallback)
- Territory factory pattern
```

---

### Phase 4: CLI Liberation Tool (Week 7)

**Objective:** Create command-line interface for testing and automation

```bash
# Tasks
- Implement totalimage-cli crate
- Commands:
  - open: Load and analyze vault
  - info: Display vault/territory info
  - zones: List partition table
  - list: Directory listing
  - extract: File extraction
  - hash: Calculate MD5/SHA1
  - tree: Display directory tree
```

**Example Usage:**
```bash
# Open and analyze
totalimage open disk.img

# Show zones
totalimage zones disk.vhd

# List files
totalimage list disk.img --zone 0 --path /

# Extract file
totalimage extract disk.img --zone 0 --file /AUTOEXEC.BAT

# Calculate hash
totalimage hash disk.img --md5 --sha1

# Extract all files
totalimage extract disk.img --zone 0 --all --output ./extracted/
```

**Testing:**
- Compare output with C# version
- Verify byte-for-byte file extraction
- Benchmark performance vs C#

---

### Phase 5: Web API Backend (Weeks 8-9)

**Objective:** Build REST API server with redb caching

#### Week 8: Core Web Server

```bash
# Tasks
- Implement totalimage-web crate
- axum server setup
- API route structure
- Vault registry
- Session management
```

**API Endpoints (Priority Order):**

**Tier 1 (Critical):**
```
POST   /api/vault/open
GET    /api/vault/{id}/info
GET    /api/vault/{id}/zones
POST   /api/vault/{id}/zones/{idx}/select
GET    /api/territory/info
GET    /api/territory/dir/{path}
POST   /api/territory/file/{path}/extract
```

**Tier 2 (Important):**
```
GET    /api/vault/{id}/hash/md5
GET    /api/vault/{id}/hash/sha1
GET    /api/file/download/{path}
POST   /api/file/batch-extract
```

**Tier 3 (Nice to have):**
```
GET    /api/file/hex/{path}
GET    /api/manifesto/boot-sector
GET    /api/session
```

---

#### Week 9: Metadata Caching with redb

```bash
# Tasks
- Integrate redb
- Implement MetadataCache
- Cache vault metadata
- Cache zone tables
- Cache directory listings
- Periodic cleanup task
```

**Cache Strategy:**
- Write-through caching
- 30-day TTL
- LRU eviction
- Automatic cleanup every 24 hours

**Performance Targets:**
- Cached directory listing: < 1ms
- Cached vault info: < 1ms
- Cache hit rate: > 80%

---

### Phase 6: Svelte Frontend (Weeks 10-12)

**Objective:** Build reactive web interface

#### Week 10: Core UI Components

```bash
# Tasks
- Setup Svelte + Vite project
- Create base layout
  - Header with menu
  - Left panel (vault tree)
  - Right panel (file list)
  - Status bar
- Implement stores
  - currentVault
  - zoneTable
  - currentTerritory
  - fileList
  - selectedFiles
```

**Components:**
```svelte
App.svelte
├── Header.svelte
│   └── MenuBar.svelte
├── MainView.svelte
│   ├── VaultTreeView.svelte
│   └── FileListView.svelte
└── StatusBar.svelte
```

---

#### Week 11: Modal Dialogs & Advanced Features

```bash
# Tasks
- VaultOpener modal (file upload)
- VaultProperties dialog
- ExtractionWizard
- HexViewer component
- SettingsPanel
```

**Modals:**
- Open vault (drag & drop or file picker)
- Vault information display
- Extract files wizard
- Hex viewer
- Settings (cache, preferences)

---

#### Week 12: Polish & UX Enhancements

```bash
# Tasks
- Drag & drop file extraction
- Context menus (right-click)
- Keyboard shortcuts
- Loading states
- Error handling
- Responsive design
- Dark/light theme
```

**UX Features:**
- Progress indicators for long operations
- Toast notifications
- Breadcrumb navigation
- File preview (text files)
- Icon system (file type icons)

---

### Phase 7: Integration & Testing (Weeks 13-14)

#### Week 13: End-to-End Testing

```bash
# Tasks
- Integration test suite
- Compare outputs with C# version
- Test all vault formats
- Test all file systems
- Performance benchmarking
```

**Test Matrix:**

| Vault Type | Partition | File System | Test Status |
|------------|-----------|-------------|-------------|
| Raw        | None      | FAT12       | ⏳          |
| Raw        | None      | FAT16       | ⏳          |
| Raw        | None      | FAT32       | ⏳          |
| Raw        | None      | ISO-9660    | ⏳          |
| Raw        | None      | exFAT       | ⏳          |
| Raw        | MBR       | FAT16       | ⏳          |
| Raw        | MBR       | FAT32       | ⏳          |
| Raw        | GPT       | FAT32       | ⏳          |
| VHD (Fixed)| MBR       | FAT32       | ⏳          |
| VHD (Dyn)  | MBR       | FAT32       | ⏳          |

---

#### Week 14: Documentation & Polish

```bash
# Tasks
- Write comprehensive README
- API documentation (OpenAPI/Swagger)
- User guide
- Developer guide
- Deployment guide
- Create demo video
```

**Documentation Deliverables:**
- README.md (Quick start)
- ARCHITECTURE.md (System design)
- API.md (REST API reference)
- BUILDING.md (Build instructions)
- CONTRIBUTING.md (Development guide)
- CHANGELOG.md (Version history)

---

### Phase 8: Advanced Features (Weeks 15-16)

**Write Support (Future Enhancement)**

```bash
# Tasks (LOW PRIORITY)
- Implement write operations for FAT
  - Create files
  - Create directories
  - Delete files
  - Modify files
- Implement defragmentation
- Implement undelete functionality
```

**Note:** Write support is complex and risky. Start with read-only, add write later.

---

## Technology Decisions

### Why Rust?
- **Performance:** Comparable to C, faster than C#
- **Safety:** Memory safety without GC
- **Portability:** Cross-platform (Linux, Windows, macOS)
- **Ecosystem:** Excellent web frameworks (axum)
- **Concurrency:** Fearless concurrency with tokio

### Why redb?
- **Embedded:** No separate database server
- **Performance:** Fast key-value store
- **ACID:** Full transactional support
- **Simplicity:** Easy to integrate
- **Rust-native:** Pure Rust, no FFI

### Why Svelte?
- **Simplicity:** Less boilerplate than React/Vue
- **Performance:** Compile-time optimization
- **Reactivity:** Built-in reactive stores
- **Size:** Smaller bundle size
- **Developer Experience:** Excellent DX

---

## Migration Strategy

### Parallel Development
- Keep C# version running during development
- Use C# version as reference implementation
- Compare outputs byte-for-byte

### Feature Parity Checklist

**Core Features:**
- ✅ Open raw sector images
- ✅ Open VHD containers
- ✅ Detect MBR partitions
- ✅ Detect GPT partitions
- ✅ Read FAT12/16/32 file systems
- ✅ Read ISO-9660 file systems
- ✅ Read exFAT file systems
- ✅ Extract files
- ✅ Calculate MD5/SHA1 hashes

**UI Features:**
- ✅ Vault tree view
- ✅ File list view
- ✅ Extract files
- ✅ Vault properties
- ⏳ Hex viewer
- ⏳ Boot sector editor
- ⏳ Create new images
- ⏳ Defragment
- ⏳ Undelete

---

## Performance Targets

### Backend (Rust)
- **Open vault:** < 100ms (cold), < 10ms (cached)
- **List directory:** < 50ms (cold), < 1ms (cached)
- **Extract file:** Limited by disk I/O
- **Calculate MD5:** > 100 MB/s

### Frontend (Svelte)
- **Initial load:** < 2s
- **Time to interactive:** < 3s
- **Directory navigation:** < 100ms
- **File selection:** < 16ms (60 FPS)

### Comparison with C# Version
- **Vault opening:** 2-5x faster (Rust + caching)
- **Directory listing:** 10-100x faster (caching)
- **File extraction:** 1-2x faster (optimized I/O)
- **Memory usage:** 50-80% less (no GC)

---

## Deployment Options

### Option 1: Desktop Web App (Tauri)
```bash
# Package as standalone desktop app
cargo install tauri-cli
cd web && npm run tauri:build

# Creates native executables for:
# - Windows (.exe)
# - macOS (.app)
# - Linux (.AppImage)
```

**Pros:**
- Native app experience
- No browser required
- File system access

**Cons:**
- Larger bundle size
- Platform-specific builds

---

### Option 2: Standalone Web Server
```bash
# Run as local web server
cargo build --release
./target/release/totalimage-web

# Access at http://localhost:3000
```

**Pros:**
- Cross-platform (any modern browser)
- Easy updates
- Remote access possible

**Cons:**
- Requires browser
- File upload limitations

---

### Option 3: Docker Container
```dockerfile
# Dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/totalimage-web /usr/local/bin/
COPY --from=builder /app/web/dist /usr/local/share/totalimage/web
CMD ["totalimage-web"]
```

```bash
# Build and run
docker build -t totalimage .
docker run -p 3000:3000 -v $(pwd)/images:/images totalimage
```

**Pros:**
- Consistent environment
- Easy deployment
- Isolated dependencies

---

## Risk Mitigation

### Technical Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Complex FAT32 parsing | High | Extensive testing, reference C# implementation |
| VHD dynamic disk support | Medium | Start with fixed disks, add dynamic later |
| LFN (Long File Name) edge cases | Medium | Test with real-world images |
| exFAT specification gaps | Medium | Focus on common cases first |
| Performance regressions | Low | Benchmark against C# continuously |

### Project Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Scope creep | High | Stick to read-only for v1.0 |
| Time overruns | Medium | Prioritize core features |
| Testing coverage | Medium | Automated test suite + manual testing |

---

## Success Criteria

### Minimum Viable Product (MVP)
- ✅ Read raw disk images
- ✅ Read VHD containers
- ✅ Detect MBR/GPT partitions
- ✅ Read FAT12/16/32 file systems
- ✅ Extract files via web UI
- ✅ Calculate hashes

### Version 1.0
- All MVP features
- ISO-9660 support
- exFAT support
- Full UI parity with C# version
- Documentation complete
- Deployment guides

### Future Enhancements (v2.0+)
- Write support (create/modify files)
- Defragmentation
- Undelete functionality
- Additional container formats
- Additional file systems (NTFS, ext4, etc.)

---

## Timeline Summary

| Phase | Duration | Status |
|-------|----------|--------|
| 1. Reconnaissance | Week 0 | ✅ Complete |
| 2. Arsenal Foundation | Weeks 1-4 | ⏳ Pending |
| 3. Extended Territories | Weeks 5-6 | ⏳ Pending |
| 4. CLI Tool | Week 7 | ⏳ Pending |
| 5. Web API Backend | Weeks 8-9 | ⏳ Pending |
| 6. Svelte Frontend | Weeks 10-12 | ⏳ Pending |
| 7. Integration & Testing | Weeks 13-14 | ⏳ Pending |
| 8. Advanced Features | Weeks 15-16 | ⏳ Pending |

**Total Estimated Time:** 14-16 weeks for MVP

---

## Next Immediate Actions

1. ✅ **Complete reconnaissance** (DONE)
2. 🔲 **Create Rust workspace structure**
   ```bash
   mkdir -p crates/{totalimage-core,totalimage-pipeline,totalimage-vaults,totalimage-territories,totalimage-zones,totalimage-cli,totalimage-web}
   ```
3. 🔲 **Implement totalimage-core traits**
4. 🔲 **Implement totalimage-pipeline**
5. 🔲 **Start RawVault implementation**

---

## Resources

### Documentation
- `/steering/CRYPTEX-DICTIONARY.md` - Master index
- `/steering/cells/vaults/` - Vault implementations
- `/steering/cells/territories/` - Territory implementations
- `/steering/cells/zones/` - Zone table implementations
- `/steering/cells/front/` - Frontend design
- `/steering/infrastructure/` - Rust/redb architecture

### Reference Implementations
- Current C# codebase: `/TotalImage.IO/`
- C# UI: `/TotalImage/`

### Test Data
- Existing test images (if available)
- Create synthetic test images with known content
- Download retro computing disk images for testing

---

## License

**Recommendation:** GNU GPL v3 or MIT

The C# version appears to be open source. Maintain compatibility with a permissive or copyleft license.

---

## Conclusion

This roadmap provides a comprehensive path from the current C# Windows Forms application to a modern Rust/redb/Svelte web application. The anarchist-themed cryptex-dictionary provides complete documentation of every component, enabling autonomous reconstruction in Rust.

**The Liberation Begins Now! ✊**

---

**Generated:** 2025-11-22
**Status:** Reconnaissance Complete, Ready for Implementation
**Next Phase:** Arsenal Foundation (Weeks 1-4)
