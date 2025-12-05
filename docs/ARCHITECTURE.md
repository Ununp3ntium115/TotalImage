# TotalImage Architecture

## System Overview

```
┌─────────────────────────────────────────────────────────────┐
│                        User Interfaces                       │
├──────────────┬─────────────────┬────────────────────────────┤
│   CLI Tool   │   REST API      │   MCP Server (AI/Claude)   │
│  totalimage  │ totalimage-web  │   totalimage-mcp           │
└──────┬───────┴────────┬────────┴────────┬───────────────────┘
       │                 │                 │
       └─────────────────┴─────────────────┘
                         │
         ┌───────────────┴────────────────┐
         │      Business Logic Layer       │
         ├─────────────────────────────────┤
         │   • Vault Factory (auto-detect) │
         │   • Zone Parser (MBR/GPT)       │
         │   • Territory Reader (FS)       │
         │   • Security Validation         │
         │   • Caching (redb)              │
         └───────────────┬─────────────────┘
                         │
         ┌───────────────┴────────────────┐
         │      Format Parsers Layer       │
         ├──────────┬─────────┬───────────┤
         │ Vaults   │ Zones   │Territories│
         ├──────────┼─────────┼───────────┤
         │ • VHD    │ • MBR   │ • FAT     │
         │ • E01    │ • GPT   │ • exFAT   │
         │ • AFF4   │         │ • NTFS    │
         │ • Raw    │         │ • ISO9660 │
         └──────────┴────┬────┴───────────┘
                         │
         ┌───────────────┴────────────────┐
         │      I/O Pipeline Layer         │
         ├─────────────────────────────────┤
         │   • Buffered Reading            │
         │   • Memory-mapped I/O           │
         │   • Streaming (zero-copy)       │
         │   • Decompression (zlib/snappy) │
         └───────────────┬─────────────────┘
                         │
         ┌───────────────┴────────────────┐
         │         Core Types              │
         ├─────────────────────────────────┤
         │   • Traits (Vault, Zone, etc)   │
         │   • Error Types                 │
         │   • Security (path validation)  │
         └─────────────────────────────────┘
```

## Layered Architecture

### Layer 1: Core (`totalimage-core`)

**Purpose:** Foundation types, traits, and security primitives

**Key Components:**
- `Vault` trait: Abstract interface for all container formats
- `Zone` trait: Partition/region abstraction
- `Territory` trait: Filesystem abstraction
- `Error` types: Unified error handling
- `Security`: Path validation, sandboxing, allocation limits

**Dependencies:** None (base layer)

**Design Principles:**
- No I/O operations (pure abstractions)
- Memory-safe by design
- Zero unsafe code
- Generic over implementation details

---

### Layer 2: I/O Pipeline (`totalimage-pipeline`)

**Purpose:** Efficient data access patterns

**Strategies:**
1. **Buffered Reading:** Sequential access with configurable buffer sizes
2. **Memory-Mapped I/O:** Random access for small files (<16GB)
3. **Streaming:** Zero-copy for large operations
4. **Partial Streams:** Windowed access for specific byte ranges

**Performance:**
- Buffer size: 64KB (configurable)
- mmap limit: 16GB (safety)
- Zero-copy where possible
- Lazy evaluation

---

### Layer 3: Format Parsers

#### Vaults (`totalimage-vaults`)

Container format implementations:

```rust
pub trait Vault {
    fn content(&mut self) -> &mut dyn Substrate;
    fn size(&self) -> u64;
    fn vault_type(&self) -> &'static str;
}
```

**Implementations:**
- `RawVault`: Direct sector access (`.img`, `.bin`)
- `VhdVault`: Microsoft VHD parser
  - Fixed: Pre-allocated blocks
  - Dynamic: Sparse allocation with BAT
  - Differencing: Parent chain resolution
- `E01Vault`: EnCase Evidence Format
  - Multi-segment support
  - zlib decompression
  - Sector-level integrity
- `Aff4Vault`: Advanced Forensic Format
  - Snappy/LZ4/Deflate compression
  - Multi-bevy architecture
  - LRU caching (64MB limit)

**Factory Pattern:**
```rust
pub fn detect_and_open(path: &Path) -> Result<Box<dyn Vault>> {
    // Magic byte detection
    // Auto-instantiate correct vault type
}
```

#### Zones (`totalimage-zones`)

Partition table parsers:

```rust
pub trait ZoneTable {
    fn enumerate_zones(&self) -> Vec<Zone>;
}
```

**Implementations:**
- `MbrZoneTable`: Master Boot Record
  - CHS addressing
  - 15+ partition type detection
- `GptZoneTable`: GUID Partition Table
  - CRC32 validation
  - UTF-16LE names
  - 128 partition maximum

#### Territories (`totalimage-territories`)

Filesystem readers:

```rust
pub trait Territory {
    fn headquarters(&mut self) -> Result<Box<dyn DirectoryCell>>;
}

pub trait DirectoryCell {
    fn list_occupants(&self) -> Result<Vec<Occupant>>;
    fn open_subdirectory(&self, name: &str) -> Result<Box<dyn DirectoryCell>>;
    fn read_file(&self, name: &str) -> Result<Vec<u8>>;
}
```

**Implementations:**
- `FatTerritory`: FAT12/16/32
  - BPB parsing with checked arithmetic
  - LFN (Long File Names)
  - Cluster chain traversal
- `ExfatTerritory`: exFAT
  - 64-bit file sizes
  - Cluster bitmap
- `NtfsTerritory`: NTFS (read-only)
  - MFT parsing via `ntfs` crate
  - Alternate Data Streams
- `IsoTerritory`: ISO-9660
  - Both-endian integers
  - Rock Ridge extensions

---

### Layer 4: Business Logic

#### Vault Factory

Auto-detection via magic bytes:

```rust
let mut file = File::open(path)?;
let mut magic = [0u8; 8];
file.read_exact(&mut magic)?;

match &magic {
    b"conectix" => VhdVault::open(path),      // VHD footer
    [0x45, 0x56, 0x46, ..] => E01Vault::open(path),  // E01 header
    _ if is_aff4(path) => Aff4Vault::open(path),
    _ => RawVault::open(path),  // Fallback
}
```

#### Caching (`redb`)

**Cache Key Design:**
```
vault_info:{blake3_hash(path)}
zone_table:{blake3_hash(path)}
dir_listing:{blake3_hash(path)}:zone{N}:{path_hash}
```

**TTL:** 30 days
**Eviction:** LRU when size > 256MB
**Maintenance:** Background task every hour

**Statistics:**
```rust
pub struct CacheStats {
    pub vault_info_count: usize,
    pub zone_table_count: usize,
    pub dir_listings_count: usize,
    pub estimated_size_bytes: usize,
}
```

---

### Layer 5: User Interfaces

#### CLI (`totalimage-cli`)

Commands implemented:
1. `info <image>` → Vault metadata
2. `zones <image>` → Partition list
3. `list <image> --zone N` → File browser
4. `extract <image> <file> --zone N` → File extraction

**Design:**
- Single binary
- No daemon/server
- Direct file access
- Progress bars for large operations

#### REST API (`totalimage-web`)

**Stack:**
- Framework: `axum` (Tokio async)
- Server: `axum-server` with TLS support
- Middleware: CORS, Auth, Timeout, Rate Limiting

**Endpoints:**
```
GET /health
GET /api/vault/info?path=
GET /api/vault/zones?path=
GET /api/vault/files?path=&zone=&directory=
```

**Security:**
- JWT or API key authentication
- Sandboxed file access (allowed_roots)
- Request size limits (10MB)
- Rate limiting (100 req/s)
- Timeout (30s per request)

#### MCP Server (`totalimage-mcp`)

**Purpose:** AI integration via Model Context Protocol

**Tools Exposed:**
1. `analyze_disk_image` → Vault + partition info
2. `list_partitions` → Zone enumeration
3. `list_files` → Directory browsing
4. `extract_file` → File extraction
5. `validate_integrity` → Checksum verification

**Modes:**
- **Standalone:** stdio for Claude Desktop
- **Integrated:** HTTP for Fire Marshal orchestration

**Features:**
- Result caching (shared with web)
- Progress reporting (WebSocket)
- Metrics (Prometheus)

---

## Data Flow Example

### Complete File Extraction Pipeline

```
User Request: Extract /Users/alice/document.pdf from evidence.vhd, partition 1

1. CLI Entry Point
   ├─> totalimage-cli extract evidence.vhd "/Users/alice/document.pdf" --zone 1

2. Vault Opening
   ├─> Factory detects VHD via magic bytes
   ├─> VhdVault::open("evidence.vhd")
   │   ├─> Read footer (512 bytes from end)
   │   ├─> Parse BAT (Block Allocation Table)
   │   └─> Return VhdVault instance

3. Zone Selection
   ├─> MbrZoneTable::parse(vault.content())
   │   ├─> Read MBR (sector 0)
   │   ├─> Parse 4 partition entries
   │   └─> Return Vec<Zone>
   ├─> Select zone[1]

4. Territory Opening
   ├─> Partial::from_zone(vault, zone[1])
   │   └─> Creates windowed view of partition bytes
   ├─> FatTerritory::parse(partial)
   │   ├─> Read BPB (BIOS Parameter Block)
   │   ├─> Validate cluster size, FAT size
   │   └─> Return FatTerritory instance

5. File Navigation
   ├─> territory.headquarters() → Root directory
   ├─> root.open_subdirectory("Users")
   ├─> users.open_subdirectory("alice")
   └─> alice.read_file("document.pdf")
       ├─> Find directory entry
       ├─> Get first cluster number
       ├─> Traverse cluster chain via FAT
       ├─> Read all clusters
       └─> Return file bytes

6. Output
   └─> Write bytes to ./document.pdf
```

**Performance Characteristics:**
- VHD footer read: ~1ms (cached after first access)
- MBR parse: ~0.1ms
- FAT BPB parse: ~0.5ms
- Directory traversal: ~2ms per directory
- File read: Depends on size (streaming for large files)

**Memory Usage:**
- VHD BAT: ~4KB for 1TB dynamic disk
- FAT cache: ~64KB buffer
- File read: Streamed (constant memory for any size)

---

## Security Architecture

### Threat Model

**Assets to Protect:**
- Host filesystem
- Server resources (CPU, RAM, disk)
- Cached data integrity

**Threats:**
1. **Malicious Disk Images:** Crafted to exploit parser bugs
2. **Path Traversal:** Filesystem attempts to access outside allowed paths
3. **Resource Exhaustion:** DoS via large allocations or infinite loops
4. **TOCTOU:** Time-of-check-time-of-use race conditions

### Mitigations

#### 1. Input Validation

```rust
// All user inputs validated before processing
pub fn validate_file_path(path: &str, allowed_roots: &[PathBuf]) -> Result<PathBuf> {
    // Reject empty paths
    // Reject ".." traversal
    // Reject absolute paths outside allowed roots
    // Canonicalize to resolve symlinks
}
```

#### 2. Checked Arithmetic

```rust
// All size calculations use checked operations
let total_size = cluster_size
    .checked_mul(cluster_count)
    .ok_or(Error::overflow("Size calculation"))?;
```

#### 3. Allocation Limits

```rust
const MAX_ALLOCATION: usize = 256 * 1024 * 1024; // 256 MB

if requested_size > MAX_ALLOCATION {
    return Err(Error::too_large("Allocation exceeds limit"));
}
```

#### 4. Sandboxing

```rust
// Web and MCP servers only access files within allowed_roots
let allowed_roots = vec![PathBuf::from("/data/evidence")];
let validated_path = validate_file_path(user_input, &allowed_roots)?;
```

#### 5. CRC/Checksum Validation

```rust
// GPT partition tables verified
if calculated_crc != header.crc32 {
    return Err(Error::integrity("GPT CRC mismatch"));
}
```

---

## Performance Optimizations

### 1. Lazy Parsing

Parse only what's needed:
```rust
// Don't parse entire FAT up front
// Parse clusters on-demand as files are accessed
```

### 2. Memory-Mapped I/O

For random access patterns:
```rust
// Small files: mmap entire file
// Large files: Stream in chunks
let strategy = if size < 16 * GB {
    IoStrategy::MemoryMapped
} else {
    IoStrategy::Streaming
};
```

### 3. LRU Caching

AFF4 chunk cache:
```rust
// Keep 256 most recent chunks in memory (64MB max)
let cache: LruCache<usize, Vec<u8>> = LruCache::new(256);
```

### 4. Metadata Caching

Persistent cache via redb:
```rust
// Cache vault info, zone tables, directory listings
// 30-day TTL, background cleanup
```

### 5. Zero-Copy Where Possible

```rust
// Use &[u8] slices instead of Vec<u8> when possible
// Avoid allocations in hot paths
```

---

## Testing Strategy

### Unit Tests (357 total)

- **Core types:** Trait implementations, error handling
- **Parsers:** Valid inputs, malformed inputs, edge cases
- **Security:** Overflow attempts, traversal attacks, large allocations

### Integration Tests

- **End-to-end workflows:** Full vault → zone → territory → file
- **Multi-format support:** VHD+FAT32, E01+NTFS, AFF4+exFAT
- **Error recovery:** Corrupted headers, incomplete files

### Fuzzing (cargo-fuzz)

Targets:
- VHD footer parser
- E01 header parser
- FAT BPB parser
- GPT header parser
- MBR parser

### Property-Based Testing

```rust
#[quickcheck]
fn sector_alignment_always_valid(size: u64, sector: u64) -> bool {
    let aligned = align_to_sector(size, sector);
    aligned % sector == 0 && aligned >= size
}
```

---

## Deployment Architecture

### Docker Container

```
┌─────────────────────────────────────┐
│   Debian Slim (Runtime)             │
│   ├─ totalimage-web (REST API)      │
│   ├─ totalimage-mcp (MCP Server)    │
│   ├─ fire-marshal (Orchestration)   │
│   └─ Cache (redb at /data/cache)    │
└─────────────────────────────────────┘
        │
        ├─ Port 3000: Web API
        ├─ Port 3001: Fire Marshal
        └─ Port 3002: MCP Server
```

### Kubernetes Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: totalimage
spec:
  replicas: 3  # Horizontal scaling
  template:
    spec:
      containers:
      - name: totalimage-web
        image: totalimage:latest
        resources:
          limits:
            memory: "512Mi"
            cpu: "500m"
        volumeMounts:
        - name: evidence-data
          mountPath: /data/images
          readOnly: true
        - name: cache
          mountPath: /data/cache
      volumes:
      - name: evidence-data
        persistentVolumeClaim:
          claimName: evidence-pvc
      - name: cache
        emptyDir:
          sizeLimit: 1Gi
```

### High Availability Setup

```
             ┌──────────────┐
             │ Load Balancer│
             └──────┬───────┘
                    │
       ┌────────────┼────────────┐
       │            │            │
   ┌───▼───┐   ┌───▼───┐   ┌───▼───┐
   │ Pod 1 │   │ Pod 2 │   │ Pod 3 │
   └───┬───┘   └───┬───┘   └───┬───┘
       │            │            │
       └────────────┼────────────┘
                    │
            ┌───────▼────────┐
            │ Shared Storage │
            │ (Evidence PVC) │
            └────────────────┘
```

---

## Monitoring and Observability

### Metrics (Prometheus)

```rust
// MCP server exposes Prometheus metrics
totalimage_mcp_tool_calls_total{tool="analyze_disk_image",status="success"} 142
totalimage_mcp_tool_duration_seconds{tool="list_files",quantile="0.99"} 0.052
totalimage_mcp_cache_operations_total{operation="hit"} 1024
```

### Logging (structured)

```rust
tracing::info!(
    vault_type = "VHD Dynamic",
    size_bytes = 10737418240,
    "Vault opened successfully"
);
```

### Health Checks

```json
GET /health
{
  "status": "ok",
  "version": "0.1.0",
  "uptime_seconds": 3600,
  "cache_stats": {
    "vault_info_count": 42,
    "estimated_size_bytes": 2097152
  }
}
```

---

## Future Architecture Considerations

### Planned Enhancements

1. **Write Support:** Modify disk images (high complexity)
2. **Streaming API:** WebSocket for large file downloads
3. **Per-IP Rate Limiting:** Keyed rate limiter
4. **Multi-Node Caching:** Distributed cache (Redis)
5. **GPU Acceleration:** Parallel decompression
6. **WASM Target:** Browser-based analysis

### Non-Goals

- **GUI:** Use web UI or integrate with existing forensic tools
- **Live System Analysis:** Focus on offline disk images
- **Network Forensics:** Out of scope
- **Timeline Analysis:** Leave to specialized tools

---

## References

- [VHD Specification](https://www.microsoft.com/en-us/download/details.aspx?id=23850)
- [EnCase E01 Format](https://www.loc.gov/preservation/digital/formats/fdd/fdd000406.shtml)
- [AFF4 Specification](https://github.com/aff4/aff4)
- [GPT Spec (UEFI)](https://uefi.org/specifications)
- [ISO-9660 Standard](https://wiki.osdev.org/ISO_9660)
- [Model Context Protocol](https://modelcontextprotocol.io/)
