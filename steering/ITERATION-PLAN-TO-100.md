# Iteration Plan to 100% Completion
**Date:** December 1, 2025
**Target:** Production-Ready TotalImage with Full Feature Parity
**Timeline:** 8-9 weeks (330 hours estimated)

---

## Overview

This document provides a detailed, actionable iteration plan to take TotalImage from its current state (~75% complete) to 100% production-ready status. Each iteration includes specific tasks, acceptance criteria, and time estimates.

---

## Current State Snapshot

### What Works ✅
- 10 Rust crates fully implemented
- 190+ unit tests passing
- MCP server operational (dual-mode)
- Fire Marshal framework complete
- Node-RED integration (6 nodes)
- Docker deployment configured
- Core vaults: Raw, VHD, E01, AFF4
- Core filesystems: FAT, exFAT, ISO, NTFS
- Partition tables: MBR, GPT

### What's Missing ⚠️
- 17 security issues remaining
- 70 additional unit tests needed
- Integration/E2E/fuzzing tests missing
- TLS/HTTPS not configured
- CI/CD pipeline not set up
- WinPE bootable USB not implemented
- Svelte web UI not started
- Production monitoring not configured

---

## Iteration 1: Critical Security Fixes
**Duration:** 3 days (8 hours)
**Priority:** P0 - Blockers
**Owner:** Development Team
**Goal:** Eliminate all data corruption risks

### Tasks

#### 1.1 Fix AFF4 Silent Decompression Failure (GAP-001)
**Location:** `crates/totalimage-vaults/src/aff4/mod.rs:361-366`
**Effort:** 2 hours

**Current Code:**
```rust
let data = match self.stream.compression {
    Aff4Compression::Deflate => {
        // Could silently fail, return empty Vec
        decompress_deflate(&compressed).unwrap_or_default()
    }
    // ...
};
```

**Fix:**
```rust
let data = match self.stream.compression {
    Aff4Compression::Deflate => {
        decompress_deflate(&compressed)
            .map_err(|e| Error::invalid_vault(format!("AFF4 deflate decompression failed: {}", e)))?
    }
    // ...
};
```

**Test:**
```rust
#[test]
fn test_aff4_deflate_corruption() {
    // Create AFF4 with corrupted deflate stream
    // Verify error is returned, not silent empty data
}
```

**Acceptance Criteria:**
- [ ] Decompression failures throw explicit errors
- [ ] No silent data corruption possible
- [ ] Test added for corrupted AFF4 chunks

---

#### 1.2 Fix E01 Silent Decompression Failure (GAP-002)
**Location:** `crates/totalimage-vaults/src/e01/mod.rs:298-304`
**Effort:** 2 hours

**Current Code:**
```rust
if chunk_info.is_compressed {
    data = zlib_decompress(raw_data).unwrap_or_default();
}
```

**Fix:**
```rust
if chunk_info.is_compressed {
    data = zlib_decompress(raw_data)
        .map_err(|e| Error::invalid_vault(format!("E01 zlib decompression failed at chunk {}: {}", chunk_index, e)))?;
}
```

**Test:**
```rust
#[test]
fn test_e01_zlib_corruption() {
    // Create E01 with corrupted zlib chunk
    // Verify error is returned with chunk number
}
```

**Acceptance Criteria:**
- [ ] Decompression failures throw explicit errors with context
- [ ] Chunk index included in error message
- [ ] Test added for corrupted E01 chunks

---

#### 1.3 Fix AFF4 Chunk Offset Calculation (GAP-003)
**Location:** `crates/totalimage-vaults/src/aff4/mod.rs:346`
**Effort:** 4 hours

**Current Code:**
```rust
let bevy_index = chunk_index / self.stream.chunks_per_segment;
let chunk_in_bevy = chunk_index % self.stream.chunks_per_segment;
let entry = self.bevy_index[chunk_index];  // ← Bug: should use chunk_in_bevy
```

**Fix:**
```rust
let bevy_index = chunk_index / self.stream.chunks_per_segment;
let chunk_in_bevy = chunk_index % self.stream.chunks_per_segment;

// Bounds check
if bevy_index as usize >= self.bevy_index.len() {
    return Err(Error::invalid_vault(format!(
        "Bevy index {} out of bounds (max {})",
        bevy_index, self.bevy_index.len()
    )));
}

let entry = &self.bevy_index[bevy_index as usize];
let chunk_offset = entry.chunk_offsets.get(chunk_in_bevy as usize)
    .ok_or_else(|| Error::invalid_vault(format!(
        "Chunk {} not found in bevy {}",
        chunk_in_bevy, bevy_index
    )))?;
```

**Test:**
```rust
#[test]
fn test_aff4_chunk_addressing() {
    // Create AFF4 with multiple bevies
    // Read chunks from different bevies
    // Verify correct data returned
}
```

**Acceptance Criteria:**
- [ ] Chunk offset calculation uses correct index
- [ ] Bounds checking prevents out-of-range access
- [ ] Test verifies multi-bevy chunk reads

---

### Iteration 1 Deliverables
- [ ] All P0 issues fixed
- [ ] 3 new tests added
- [ ] No silent data corruption paths remain
- [ ] Code review completed
- [ ] Documentation updated

---

## Iteration 2: Security Hardening
**Duration:** 5 days (21 hours)
**Priority:** P1 - Required for Production
**Goal:** Complete Phase 2 security improvements

### Tasks

#### 2.1 Complete Path Traversal Prevention (GAP-006)
**Location:** `crates/totalimage-core/src/security.rs`
**Effort:** 4 hours

**Enhancement:**
```rust
pub fn validate_file_path(path: &str) -> Result<PathBuf> {
    // Existing checks...

    // Add whitelist-based validation
    let allowed_directories = vec![
        PathBuf::from("/data/images"),
        PathBuf::from("/mnt/forensic"),
    ];

    let canonical = canonicalize(&path)
        .map_err(|e| Error::InvalidPath(format!("Cannot canonicalize path: {}", e)))?;

    // Check if path is within any allowed directory
    let is_allowed = allowed_directories.iter().any(|allowed| {
        canonical.starts_with(allowed)
    });

    if !is_allowed && !cfg!(test) {
        return Err(Error::PermissionDenied(
            "Path outside allowed directories".into()
        ));
    }

    Ok(canonical)
}
```

**Test:**
```rust
#[test]
fn test_path_whitelist() {
    // Test allowed paths succeed
    // Test disallowed paths fail
    // Test symlink traversal blocked
}
```

---

#### 2.2 Add AFF4 Cache Size Limits (GAP-007)
**Location:** `crates/totalimage-vaults/src/aff4/mod.rs`
**Effort:** 3 hours

**Implementation:**
```rust
use lru::LruCache;

const MAX_AFF4_CACHE_SIZE: usize = 100; // chunks
const MAX_AFF4_CACHE_BYTES: usize = 100 * 1024 * 1024; // 100 MB

pub struct Aff4Vault {
    // ...
    chunk_cache: LruCache<u32, Vec<u8>>,
    cache_size_bytes: usize,
}

impl Aff4Vault {
    fn cache_chunk(&mut self, chunk_index: u32, data: Vec<u8>) {
        let data_size = data.len();

        // Evict if adding would exceed byte limit
        while self.cache_size_bytes + data_size > MAX_AFF4_CACHE_BYTES {
            if let Some((_, evicted)) = self.chunk_cache.pop_lru() {
                self.cache_size_bytes -= evicted.len();
            } else {
                break;
            }
        }

        self.cache_size_bytes += data_size;
        self.chunk_cache.put(chunk_index, data);
    }
}
```

---

#### 2.3 Document VHD Chain Depth Limit (GAP-009)
**Location:** `crates/totalimage-vaults/src/vhd/mod.rs`
**Effort:** 2 hours

**Add constant:**
```rust
/// Maximum depth of VHD differencing chain.
/// Prevents infinite recursion and excessive memory usage.
const MAX_VHD_CHAIN_DEPTH: usize = 10;

impl VhdVault {
    pub fn open(path: &Path, config: VaultConfig) -> Result<Self> {
        Self::open_with_depth(path, config, 0)
    }

    fn open_with_depth(path: &Path, config: VaultConfig, depth: usize) -> Result<Self> {
        if depth > MAX_VHD_CHAIN_DEPTH {
            return Err(Error::invalid_vault(format!(
                "VHD differencing chain depth {} exceeds maximum {}",
                depth, MAX_VHD_CHAIN_DEPTH
            )));
        }

        // ... existing code ...

        if footer.disk_type == VhdType::Differencing {
            let parent_path = resolve_parent_locator(&footer, path)?;
            let parent = Self::open_with_depth(&parent_path, config, depth + 1)?;
            // ...
        }
    }
}
```

---

#### 2.4 Implement Snappy/LZ4 Decompression (GAP-011)
**Location:** `crates/totalimage-vaults/src/aff4/mod.rs`
**Effort:** 8 hours

**Add dependencies:**
```toml
# Cargo.toml
[dependencies]
snap = "1.1"
lz4 = "1.24"
```

**Implementation:**
```rust
use snap::raw::Decoder as SnappyDecoder;
use lz4::block::decompress;

impl Aff4Vault {
    fn decompress_chunk(&self, compressed: &[u8]) -> Result<Vec<u8>> {
        match self.stream.compression {
            Aff4Compression::Deflate => {
                flate2::read::ZlibDecoder::new(compressed)
                    .read_to_end(&mut vec![])
                    .map_err(|e| Error::invalid_vault(format!("Deflate: {}", e)))
            }
            Aff4Compression::Snappy => {
                let mut decoder = SnappyDecoder::new();
                decoder.decompress_vec(compressed)
                    .map_err(|e| Error::invalid_vault(format!("Snappy: {}", e)))
            }
            Aff4Compression::Lz4 => {
                // LZ4 requires decompressed size hint
                let decompressed_size = self.stream.chunk_size as usize;
                decompress(compressed, Some(decompressed_size as i32))
                    .map_err(|e| Error::invalid_vault(format!("LZ4: {}", e)))
            }
            Aff4Compression::None => {
                Ok(compressed.to_vec())
            }
        }
    }
}
```

**Tests:**
```rust
#[test]
fn test_snappy_decompression() { /* ... */ }

#[test]
fn test_lz4_decompression() { /* ... */ }
```

---

#### 2.5 Add TLS/HTTPS Support
**Location:** `crates/totalimage-web/src/main.rs`, `crates/totalimage-mcp/src/server.rs`
**Effort:** 8 hours

**Implementation:**
```rust
use axum_server::tls_rustls::RustlsConfig;

#[tokio::main]
async fn main() -> Result<()> {
    // ... existing code ...

    let app = Router::new()
        .route("/health", get(health))
        // ... other routes ...

    if let Some(tls_config) = load_tls_config()? {
        let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
        tracing::info!("Starting HTTPS server on {}", addr);
        axum_server::bind_rustls(addr, tls_config)
            .serve(app.into_make_service())
            .await?;
    } else {
        let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
        tracing::info!("Starting HTTP server on {}", addr);
        axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .await?;
    }

    Ok(())
}

fn load_tls_config() -> Result<Option<RustlsConfig>> {
    let cert_path = std::env::var("TOTALIMAGE_TLS_CERT").ok();
    let key_path = std::env::var("TOTALIMAGE_TLS_KEY").ok();

    match (cert_path, key_path) {
        (Some(cert), Some(key)) => {
            let config = RustlsConfig::from_pem_file(cert, key)
                .await
                .map_err(|e| Error::Configuration(format!("TLS config: {}", e)))?;
            Ok(Some(config))
        }
        _ => Ok(None),
    }
}
```

**Documentation:**
```markdown
# TLS Configuration

Set environment variables:
- `TOTALIMAGE_TLS_CERT=/path/to/cert.pem`
- `TOTALIMAGE_TLS_KEY=/path/to/key.pem`

Generate self-signed cert for testing:
```bash
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes
```
```

---

### Iteration 2 Deliverables
- [ ] All P1 security issues resolved
- [ ] TLS/HTTPS operational
- [ ] 5 new security tests added
- [ ] Security documentation updated
- [ ] Code review and penetration testing

---

## Iteration 3: Test Coverage Expansion
**Duration:** 10 days (59 hours)
**Priority:** P1 - Required for Production
**Goal:** Achieve 260+ tests with integration/fuzzing

### 3.1 Add 70 Unit Tests (35 hours)

**Breakdown:**
- totalimage-core: +6 tests (security validation) - 3h
- totalimage-vaults: +26 tests (edge cases, error paths) - 13h
- totalimage-zones: +10 tests (GPT backup, MBR edge cases) - 5h
- totalimage-territories: +19 tests (NTFS deep, Joliet, exFAT) - 10h
- totalimage-mcp: +5 tests (WebSocket, auth edge cases) - 3h
- fire-marshal: +4 tests (registry, transport) - 2h

**Template for each test:**
```rust
#[test]
fn test_<component>_<scenario>_<expected_result>() {
    // Arrange
    let input = create_test_<component>();

    // Act
    let result = component.operation(input);

    // Assert
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), expected_value);
}
```

---

### 3.2 Integration Tests (16 hours)

**Create:** `tests/integration/`

```rust
// tests/integration/test_full_vault_to_file_extract.rs

#[tokio::test]
async fn test_extract_file_from_vhd_fat32() {
    // 1. Open VHD vault
    let vault = open_vault("tests/fixtures/disk.vhd", VaultConfig::default())?;

    // 2. Parse partition table
    let zones = parse_zone_table(vault.content())?;

    // 3. Open FAT32 filesystem
    let partition = PartialPipeline::new(vault.content(), zones[0].offset, zones[0].length);
    let territory = FatTerritory::parse(partition)?;

    // 4. List files
    let root = territory.headquarters();
    let files = root.list_occupants();
    assert!(files.len() > 0);

    // 5. Extract file
    let data = territory.extract_file("AUTOEXEC.BAT")?;
    assert_eq!(data.len(), expected_size);
}
```

**Tests to create:**
- [ ] Full VHD → FAT → extract pipeline
- [ ] Full E01 → NTFS → extract pipeline
- [ ] Full AFF4 → exFAT → extract pipeline
- [ ] MCP server end-to-end
- [ ] Fire Marshal → TotalImage integration
- [ ] Node-RED node integration

---

### 3.3 Fuzzing Setup (8 hours)

**Install:**
```bash
cargo install cargo-fuzz
```

**Create fuzz targets:**
```
fuzz/
├── Cargo.toml
└── fuzz_targets/
    ├── fuzz_mbr_parser.rs
    ├── fuzz_gpt_parser.rs
    ├── fuzz_fat_bpb.rs
    ├── fuzz_vhd_footer.rs
    └── fuzz_e01_header.rs
```

**Example:**
```rust
// fuzz/fuzz_targets/fuzz_mbr_parser.rs
#![no_main]
use libfuzzer_sys::fuzz_target;
use totalimage_zones::MbrZoneTable;
use std::io::Cursor;

fuzz_target!(|data: &[u8]| {
    let mut cursor = Cursor::new(data);
    let _ = MbrZoneTable::parse(&mut cursor, 512);
});
```

**Run fuzzing:**
```bash
# Run each target for 1 hour minimum
cargo fuzz run fuzz_mbr_parser -- -max_total_time=3600
cargo fuzz run fuzz_gpt_parser -- -max_total_time=3600
cargo fuzz run fuzz_fat_bpb -- -max_total_time=3600
```

---

### Iteration 3 Deliverables
- [ ] 260+ total tests passing
- [ ] Integration test suite operational
- [ ] Fuzzing targets created and run
- [ ] CI/CD running all tests
- [ ] Coverage report generated (target: >80%)

---

## Iteration 4: PYRO Platform Integration
**Duration:** 8 days (40 hours)
**Priority:** P1 - Required for Platform
**Goal:** Full PYRO Platform integration

### 4.1 Create PYRO Worker Package (8 hours)

**Location:** `packages/pyro-worker-totalimage/`

**Structure:**
```
packages/pyro-worker-totalimage/
├── package.json
├── src/
│   ├── index.ts
│   ├── worker.ts
│   ├── mcp-client.ts
│   └── queue.ts
├── test/
│   └── worker.test.ts
└── README.md
```

**Implementation:**
```typescript
// src/worker.ts
import { Worker } from 'bullmq';
import { MCPClient } from './mcp-client';

export class TotalImageWorker {
  private worker: Worker;
  private mcp: MCPClient;

  constructor(config: WorkerConfig) {
    this.mcp = new MCPClient(config.mcpUrl);

    this.worker = new Worker('totalimage', async (job) => {
      const { tool, args } = job.data;

      try {
        const result = await this.mcp.callTool(tool, args);
        return result;
      } catch (error) {
        throw new Error(`Tool execution failed: ${error}`);
      }
    }, {
      connection: config.redisConfig,
      concurrency: config.concurrency || 5,
    });
  }
}
```

---

### 4.2 Add JWT Authentication (4 hours)

**Location:** `crates/totalimage-mcp/src/auth.rs`

**Implementation:**
```rust
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};

pub struct JwtAuth {
    secret: String,
    algorithm: Algorithm,
}

impl JwtAuth {
    pub fn new(secret: String) -> Self {
        Self {
            secret,
            algorithm: Algorithm::HS256,
        }
    }

    pub fn validate_token(&self, token: &str) -> Result<Claims> {
        let key = DecodingKey::from_secret(self.secret.as_bytes());
        let validation = Validation::new(self.algorithm);

        let token_data = decode::<Claims>(token, &key, &validation)
            .map_err(|e| Error::Unauthorized(format!("Invalid token: {}", e)))?;

        Ok(token_data.claims)
    }
}

// Middleware
pub async fn auth_middleware(
    State(auth): State<Arc<JwtAuth>>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response> {
    let auth_header = headers.get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| Error::Unauthorized("Missing authorization header".into()))?;

    let token = auth_header.strip_prefix("Bearer ")
        .ok_or_else(|| Error::Unauthorized("Invalid authorization format".into()))?;

    let claims = auth.validate_token(token)?;

    // Add claims to request extensions
    request.extensions_mut().insert(claims);

    Ok(next.run(request).await)
}
```

---

### 4.3 Job Queue Integration (8 hours)

**Implementation:**
```rust
// crates/totalimage-mcp/src/queue.rs
use redis::aio::ConnectionManager;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub tool: String,
    pub args: serde_json::Value,
    pub status: JobStatus,
}

pub struct JobQueue {
    redis: ConnectionManager,
}

impl JobQueue {
    pub async fn enqueue(&self, tool: &str, args: serde_json::Value) -> Result<String> {
        let job_id = uuid::Uuid::new_v4().to_string();
        let job = Job {
            id: job_id.clone(),
            tool: tool.to_string(),
            args,
            status: JobStatus::Pending,
        };

        let job_json = serde_json::to_string(&job)?;
        self.redis.lpush("totalimage:jobs", job_json).await?;

        Ok(job_id)
    }

    pub async fn get_job(&self, job_id: &str) -> Result<Job> {
        let job_json: String = self.redis.get(format!("totalimage:job:{}", job_id)).await?;
        let job = serde_json::from_str(&job_json)?;
        Ok(job)
    }
}
```

---

### Iteration 4 Deliverables
- [ ] PYRO worker package published to npm
- [ ] JWT authentication operational
- [ ] Job queue integration working
- [ ] TLS configured for all services
- [ ] End-to-end PYRO integration tested

---

## Iteration 5: Production Infrastructure
**Duration:** 6 days (32 hours)
**Priority:** P1/P2 - Production Readiness
**Goal:** Automated deployment and monitoring

### 5.1 CI/CD Pipeline (12 hours)

**GitHub Actions:**
```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - run: cargo test --all-features --workspace
      - run: cargo clippy -- -D warnings
      - run: cargo fmt -- --check

  security:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - run: cargo install cargo-audit
      - run: cargo audit

  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/tarpaulin@v0.1
        with:
          args: '--all-features --workspace'
      - uses: codecov/codecov-action@v3

  docker:
    runs-on: ubuntu-latest
    needs: test
    steps:
      - uses: actions/checkout@v3
      - uses: docker/build-push-action@v4
        with:
          push: true
          tags: totalimage/mcp:${{ github.sha }}
```

---

### 5.2 Prometheus Metrics (4 hours)

**Add dependency:**
```toml
[dependencies]
metrics = "0.21"
metrics-exporter-prometheus = "0.13"
```

**Implementation:**
```rust
use metrics::{counter, histogram, gauge};
use metrics_exporter_prometheus::PrometheusBuilder;

pub fn setup_metrics() -> Result<()> {
    PrometheusBuilder::new()
        .install()
        .map_err(|e| Error::Configuration(format!("Metrics setup: {}", e)))?;
    Ok(())
}

// In tool execution:
counter!("totalimage.tool.calls.total", 1, "tool" => tool_name);
histogram!("totalimage.tool.duration.seconds", duration.as_secs_f64(), "tool" => tool_name);
gauge!("totalimage.cache.size.bytes", cache_size as f64);
```

**Expose endpoint:**
```rust
async fn metrics_handler() -> impl IntoResponse {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer).unwrap();
    Response::builder()
        .header(CONTENT_TYPE, "text/plain; version=0.0.4")
        .body(Body::from(buffer))
        .unwrap()
}
```

---

### 5.3 Kubernetes Manifests (8 hours)

**Create:** `k8s/`

```yaml
# k8s/deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: totalimage-mcp
spec:
  replicas: 3
  selector:
    matchLabels:
      app: totalimage-mcp
  template:
    metadata:
      labels:
        app: totalimage-mcp
    spec:
      containers:
      - name: mcp
        image: totalimage/mcp:latest
        ports:
        - containerPort: 3002
        env:
        - name: RUST_LOG
          value: "info"
        - name: FIRE_MARSHAL_URL
          value: "http://fire-marshal:3001"
        resources:
          requests:
            memory: "256Mi"
            cpu: "500m"
          limits:
            memory: "512Mi"
            cpu: "1000m"
        livenessProbe:
          httpGet:
            path: /health
            port: 3002
          initialDelaySeconds: 10
          periodSeconds: 30
        readinessProbe:
          httpGet:
            path: /health
            port: 3002
          initialDelaySeconds: 5
          periodSeconds: 10
---
apiVersion: v1
kind: Service
metadata:
  name: totalimage-mcp
spec:
  selector:
    app: totalimage-mcp
  ports:
  - port: 3002
    targetPort: 3002
  type: LoadBalancer
```

---

### 5.4 Performance Benchmarking (8 hours)

**Create:** `benches/`

```rust
// benches/vault_parsing.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_parse_mbr(c: &mut Criterion) {
    let data = include_bytes!("../tests/fixtures/mbr_disk.img");

    c.bench_function("parse_mbr", |b| {
        b.iter(|| {
            let mut cursor = Cursor::new(black_box(data));
            MbrZoneTable::parse(&mut cursor, 512).unwrap()
        })
    });
}

fn bench_open_vhd_dynamic(c: &mut Criterion) {
    let path = "tests/fixtures/dynamic.vhd";

    c.bench_function("open_vhd_dynamic", |b| {
        b.iter(|| {
            VhdVault::open(Path::new(black_box(path)), VaultConfig::default()).unwrap()
        })
    });
}

criterion_group!(benches, bench_parse_mbr, bench_open_vhd_dynamic);
criterion_main!(benches);
```

**Run benchmarks:**
```bash
cargo bench --workspace
```

---

### Iteration 5 Deliverables
- [ ] CI/CD pipeline operational
- [ ] Prometheus metrics exposed
- [ ] Kubernetes manifests validated
- [ ] Performance benchmarks baselined
- [ ] Deployment documentation complete

---

## Iteration 6: Feature Completeness
**Duration:** 10 days (80 hours)
**Priority:** P1/P2 - Feature Parity
**Goal:** Achieve FTK Imager replacement status

### 6.1 WinPE Bootable USB Creation (32 hours)

This is the largest single feature remaining. See detailed breakdown in PYRO-INTEGRATION-DESIGN.md Section 11.

**High-level tasks:**
1. USB drive detection (4h)
2. Partition creation (GPT/MBR) (4h)
3. FAT32 formatting (4h)
4. WinPE deployment (8h)
5. Driver injection (4h)
6. Boot configuration (4h)
7. Testing on physical hardware (4h)

---

### 6.2 E01 Write Support (16 hours)

**Add to:** `crates/totalimage-acquire/src/e01.rs`

Implementation requires:
- E01 section writing
- CRC32 calculation
- Zlib compression
- Multi-segment support
- Hash embedding

---

### 6.3 Joliet ISO Extension (8 hours)

**Add to:** `crates/totalimage-territories/src/iso/mod.rs`

```rust
pub struct JolietDescriptor {
    pub escape_sequences: [u8; 3],
    pub volume_identifier: String,  // UTF-16BE
}

impl IsoTerritory {
    fn parse_joliet_descriptor(&mut self) -> Result<()> {
        // Parse supplementary volume descriptor at sector 17
        // Check escape sequences for Joliet (0x25, 0x2F, ...)
        // Use UTF-16BE decoding for filenames
    }
}
```

---

### Iteration 6 Deliverables
- [ ] WinPE USB creation working
- [ ] E01 write support functional
- [ ] Joliet extension implemented
- [ ] All P2 features complete
- [ ] FTK Imager feature parity achieved

---

## Iteration 7: Web UI Development
**Duration:** 12 days (60 hours)
**Priority:** P2 - User Experience
**Goal:** Functional Svelte frontend

### 7.1 Project Setup (4 hours)

```bash
npm create vite@latest web -- --template svelte-ts
cd web
npm install
npm install -D tailwindcss postcss autoprefixer
npm install axios @tanstack/svelte-query
```

---

### 7.2 Core Components (40 hours)

**Component breakdown:**
- ImageUploader (8h)
- PartitionViewer (8h)
- FileExplorer (12h)
- ExtractionPanel (8h)
- IntegrityValidator (6h)
- ProgressIndicator (WebSocket) (6h)

---

### 7.3 Styling & Polish (8 hours)

- Dark mode
- Responsive design
- Loading states
- Error handling

---

### Iteration 7 Deliverables
- [ ] Svelte UI functional
- [ ] All core workflows working
- [ ] Responsive design
- [ ] Dark mode implemented

---

## Iteration 8: Documentation & Polish
**Duration:** 6 days (30 hours)
**Priority:** P2/P3 - Completeness
**Goal:** Production-ready documentation

### Tasks

1. **API Reference (OpenAPI)** - 4h
2. **Deployment Guide** - 4h
3. **User Guide** - 8h
4. **Video Tutorials** - 16h
5. **CONTRIBUTING.md** - 2h
6. **CHANGELOG.md** - 2h

---

## Summary Timeline

| Iteration | Duration | Cumulative | Priority | Status |
|-----------|----------|------------|----------|--------|
| 1. Critical Fixes | 3 days | 3 days | P0 | ⏳ Not Started |
| 2. Security | 5 days | 8 days | P1 | ⏳ Not Started |
| 3. Test Coverage | 10 days | 18 days | P1 | ⏳ Not Started |
| 4. PYRO Integration | 8 days | 26 days | P1 | ⏳ Not Started |
| 5. Infrastructure | 6 days | 32 days | P1/P2 | ⏳ Not Started |
| 6. Feature Complete | 10 days | 42 days | P1/P2 | ⏳ Not Started |
| 7. Web UI | 12 days | 54 days | P2 | ⏳ Not Started |
| 8. Documentation | 6 days | 60 days | P2/P3 | ⏳ Not Started |

**Total: 60 working days (~12 weeks)**

---

## Definition of Done

### MVP (After Iteration 4): ~26 days
- ✅ All P0/P1 issues fixed
- ✅ >260 tests passing
- ✅ PYRO integration complete
- ✅ Security hardened
- ✅ CI/CD operational

### 100% Complete (After Iteration 8): ~60 days
- ✅ All features implemented
- ✅ Web UI functional
- ✅ Documentation complete
- ✅ WinPE USB working
- ✅ External audit passed
- ✅ Performance validated

---

**Next Action:** Begin Iteration 1 - Fix P0 issues immediately.
