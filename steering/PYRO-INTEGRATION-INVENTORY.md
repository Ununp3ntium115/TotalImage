# PYRO Platform Integration Inventory

**Generated:** 2025-11-25
**Purpose:** Self-inventory of TotalImage components for PYRO Platform integration

---

## 1. Current State Summary

### Components Ready for PYRO Integration

| Component | Status | API Ready | Notes |
|-----------|--------|-----------|-------|
| **TotalImage MCP Server** | ✅ Complete | Yes | 5 tools, dual-mode (stdio/HTTP) |
| **Fire Marshal Framework** | ✅ Complete | Yes | Rate limiting, tool registry |
| **Node-RED Contrib** | ✅ Complete | Yes | 6 nodes |
| **REST API (Web)** | ✅ Complete | Yes | Axum-based, cached |
| **Docker Deployment** | ✅ Complete | Yes | Multi-service compose |
| **Vault Factory** | ✅ Complete | Yes | Auto-detection |

### Test Coverage
- **Total Tests:** 147+ passing
- **Vaults:** 59 tests (Raw, VHD, E01, AFF4, Factory)
- **Zones:** 20 tests (MBR, GPT)
- **Territories:** 36 tests (FAT, exFAT, ISO)
- **Acquire:** 15 tests (raw/VHD creation)

---

## 2. API Endpoints & Workers

### 2.1 MCP Server Tools (Port 3002)

| Tool | Endpoint | Method | Purpose |
|------|----------|--------|---------|
| `analyze_disk_image` | `/mcp` | POST | Comprehensive analysis |
| `list_partitions` | `/mcp` | POST | Zone enumeration |
| `list_files` | `/mcp` | POST | Directory listing |
| `extract_file` | `/mcp` | POST | File extraction |
| `validate_integrity` | `/mcp` | POST | Forensic validation |

**Protocol:** JSON-RPC 2.0 over HTTP

**Request Format:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "analyze_disk_image",
    "arguments": {
      "path": "/path/to/image.img",
      "deep_scan": false,
      "cache": true
    }
  }
}
```

### 2.2 Web API Endpoints (Port 3000)

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/health` | GET | Health check |
| `/api/vault/info` | GET | Vault metadata |
| `/api/vault/zones` | GET | Partition list |

### 2.3 Fire Marshal Endpoints (Port 3001)

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/health` | GET | Health check |
| `/tools/register` | POST | Register external tool |
| `/tools/list` | GET | List registered tools |
| `/tools/call` | POST | Execute tool |
| `/stats` | GET | Database statistics |

---

## 3. Worker Types

### 3.1 Online Workers (HTTP-based)

```
┌─────────────────────────────────────────────────────────────┐
│                    Online Worker Architecture                │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────┐    HTTP    ┌──────────────────────────┐  │
│  │ PYRO Backend │───────────▶│ TotalImage MCP Server    │  │
│  │ (Node.js)    │            │ (Rust, port 3002)        │  │
│  └──────────────┘            └──────────────────────────┘  │
│          │                              │                    │
│          │ HTTP                         │ Read               │
│          ▼                              ▼                    │
│  ┌──────────────┐            ┌──────────────────────────┐  │
│  │ Fire Marshal │            │ Disk Images              │  │
│  │ (port 3001)  │            │ (mounted volume)         │  │
│  └──────────────┘            └──────────────────────────┘  │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

**Configuration:**
- Environment: `TOTALIMAGE_MCP_URL=http://localhost:3002`
- Authentication: Bearer token (optional)
- Timeout: 30 seconds (configurable)

### 3.2 Offline Workers (Direct Rust)

```
┌─────────────────────────────────────────────────────────────┐
│                   Offline Worker Architecture                │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────────────────────────────────────────────┐  │
│  │              Offline Processing Node                  │  │
│  │                                                        │  │
│  │  ┌────────────────┐    Direct Call    ┌────────────┐ │  │
│  │  │ totalimage-cli │──────────────────▶│ Disk Image │ │  │
│  │  │ (Rust binary)  │                   │ Files      │ │  │
│  │  └────────────────┘                   └────────────┘ │  │
│  │           │                                           │  │
│  │           │ Uses                                      │  │
│  │           ▼                                           │  │
│  │  ┌────────────────────────────────────────────────┐  │  │
│  │  │ totalimage-vaults (open_vault)                 │  │  │
│  │  │ totalimage-zones (MBR/GPT)                     │  │  │
│  │  │ totalimage-territories (FAT/exFAT/ISO)         │  │  │
│  │  └────────────────────────────────────────────────┘  │  │
│  │                                                        │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

**Usage:**
```bash
# Analyze disk image
./totalimage info /path/to/image.vhd

# List partitions
./totalimage zones /path/to/image.e01

# List files
./totalimage list /path/to/image.aff4 --zone 0

# Extract file
./totalimage extract /path/to/image.img AUTOEXEC.BAT --output /tmp/
```

---

## 4. Integration Points for PYRO

### 4.1 Required PYRO Services

| Service | Purpose | Port | Status |
|---------|---------|------|--------|
| PYRO API Gateway | Route requests | 8080 | Needs config |
| PYRO Auth Service | API authentication | 8081 | Needs integration |
| PYRO Job Queue | Async processing | - | Needs worker |
| PYRO File Storage | Image storage | - | Mount point |

### 4.2 Integration Tasks

#### High Priority

1. **Create PYRO Worker Package**
   - NPM package: `@pyro/worker-totalimage`
   - Wraps MCP HTTP calls
   - Handles job queue integration
   - Estimated: 8 hours

2. **Add PYRO Authentication**
   - JWT token validation in MCP server
   - API key management
   - Estimated: 4 hours

3. **Implement Job Queue Worker**
   - Bull/BullMQ integration
   - Long-running analysis jobs
   - Progress reporting
   - Estimated: 8 hours

#### Medium Priority

4. **Add WebSocket Support**
   - Real-time progress updates
   - Live analysis streaming
   - Estimated: 8 hours

5. **Create Offline Worker Binary**
   - Standalone processing node
   - S3/MinIO integration for images
   - Estimated: 8 hours

6. **Add Metrics/Telemetry**
   - Prometheus metrics endpoint
   - OpenTelemetry tracing
   - Estimated: 4 hours

---

## 5. Data Flow Diagrams

### 5.1 Online Analysis Flow

```
User Request
     │
     ▼
┌─────────────┐
│ PYRO Gateway│
└─────────────┘
     │
     ▼
┌─────────────┐     ┌──────────────┐
│ Auth Service│────▶│ Validate JWT │
└─────────────┘     └──────────────┘
     │
     ▼
┌─────────────┐
│ Job Queue   │
└─────────────┘
     │
     ▼
┌─────────────────────────────────┐
│ TotalImage Worker               │
│ ┌─────────────────────────────┐ │
│ │ HTTP POST to MCP Server     │ │
│ │ analyze_disk_image          │ │
│ └─────────────────────────────┘ │
│              │                   │
│              ▼                   │
│ ┌─────────────────────────────┐ │
│ │ Results (JSON)              │ │
│ └─────────────────────────────┘ │
└─────────────────────────────────┘
     │
     ▼
┌─────────────┐
│ PYRO DB     │
│ (Results)   │
└─────────────┘
```

### 5.2 Offline Processing Flow

```
S3/MinIO Bucket
     │
     │ Download
     ▼
┌─────────────────────────────────┐
│ Offline Processing Node         │
│                                  │
│  ┌────────────────────────────┐ │
│  │ totalimage CLI             │ │
│  │ - Direct filesystem access │ │
│  │ - No network calls        │ │
│  └────────────────────────────┘ │
│              │                   │
│              ▼                   │
│  ┌────────────────────────────┐ │
│  │ Results JSON               │ │
│  └────────────────────────────┘ │
└─────────────────────────────────┘
     │
     │ Upload
     ▼
┌─────────────┐
│ S3/MinIO    │
│ (Results)   │
└─────────────┘
```

---

## 6. Configuration Templates

### 6.1 Docker Environment (.env)

```bash
# TotalImage Configuration
TOTALIMAGE_CACHE_DIR=/data/cache
RUST_LOG=info

# Fire Marshal Configuration
FIRE_MARSHAL_PORT=3001
FIRE_MARSHAL_RATE_LIMIT=100
FIRE_MARSHAL_TIMEOUT=30000

# MCP Server Configuration
MCP_SERVER_PORT=3002
MCP_AUTH_ENABLED=true
MCP_API_KEY=your-api-key-here

# PYRO Integration
PYRO_API_URL=http://pyro-gateway:8080
PYRO_AUTH_URL=http://pyro-auth:8081
PYRO_JOB_QUEUE_URL=redis://redis:6379
```

### 6.2 PYRO Worker Configuration

```typescript
// @pyro/worker-totalimage config
export const config = {
  mcp: {
    url: process.env.TOTALIMAGE_MCP_URL || 'http://localhost:3002',
    timeout: 30000,
    apiKey: process.env.MCP_API_KEY,
  },
  queue: {
    redis: process.env.PYRO_JOB_QUEUE_URL,
    concurrency: 5,
    maxRetries: 3,
  },
  storage: {
    type: 's3',
    bucket: process.env.PYRO_IMAGES_BUCKET,
    region: process.env.AWS_REGION,
  },
};
```

---

## 7. Next Steps

### Immediate (This Week)

1. [x] Merge branches to master
2. [ ] Create PYRO worker NPM package
3. [ ] Add JWT authentication to MCP server
4. [ ] Test Docker deployment with PYRO

### Short Term (Next 2 Weeks)

5. [ ] Implement job queue worker
6. [ ] Add WebSocket progress updates
7. [ ] Create offline worker binary
8. [ ] Integration testing with PYRO

### Medium Term (Next Month)

9. [ ] Add metrics/telemetry
10. [ ] Performance optimization
11. [ ] NTFS filesystem support
12. [ ] Svelte UI frontend

---

## 8. API Reference

### 8.1 analyze_disk_image

**Input:**
```json
{
  "path": "/images/disk.vhd",
  "deep_scan": false,
  "cache": true
}
```

**Output:**
```json
{
  "vault_type": "VHD Dynamic",
  "vault_size": 10737418240,
  "partitions": [
    {
      "index": 0,
      "type": "FAT32 (LBA)",
      "offset": 1048576,
      "length": 10736369664
    }
  ],
  "filesystems": [
    {
      "zone_index": 0,
      "type": "FAT32",
      "label": "MYDISK",
      "total_size": 10736369664,
      "free_size": 5368184832
    }
  ]
}
```

### 8.2 list_partitions

**Input:**
```json
{
  "path": "/images/disk.e01",
  "cache": true
}
```

**Output:**
```json
{
  "partition_table": "GPT",
  "zones": [
    {
      "index": 0,
      "type": "EFI System Partition",
      "offset": 1048576,
      "length": 104857600
    },
    {
      "index": 1,
      "type": "Microsoft Basic Data",
      "offset": 105906176,
      "length": 107268112384
    }
  ]
}
```

### 8.3 list_files

**Input:**
```json
{
  "path": "/images/floppy.img",
  "zone_index": 0,
  "directory": "/",
  "cache": true
}
```

**Output:**
```json
{
  "files": [
    {"name": "AUTOEXEC.BAT", "size": 128, "is_directory": false},
    {"name": "CONFIG.SYS", "size": 256, "is_directory": false},
    {"name": "DOS", "size": 0, "is_directory": true}
  ]
}
```

### 8.4 extract_file

**Input:**
```json
{
  "image_path": "/images/disk.img",
  "file_path": "AUTOEXEC.BAT",
  "zone_index": 0,
  "output_path": "/tmp/extracted/"
}
```

**Output:**
```json
{
  "success": true,
  "output_path": "/tmp/extracted/AUTOEXEC.BAT",
  "size": 128,
  "checksum": "d41d8cd98f00b204e9800998ecf8427e"
}
```

### 8.5 validate_integrity

**Input:**
```json
{
  "path": "/images/evidence.e01",
  "check_checksums": true,
  "check_boot_sectors": true
}
```

**Output:**
```json
{
  "valid": true,
  "issues": [],
  "checksums": {
    "md5": "d41d8cd98f00b204e9800998ecf8427e",
    "sha1": "da39a3ee5e6b4b0d3255bfef95601890afd80709",
    "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
  },
  "boot_sector_valid": true,
  "partition_table_valid": true
}
```

---

*This inventory should be updated as integration progresses.*
