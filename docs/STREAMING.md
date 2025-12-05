# TotalImage Streaming Architecture

## Overview

TotalImage is designed from the ground up to handle disk images **larger than available disk space or RAM** through streaming I/O, memory-mapped files, and zero-copy operations.

## Use Cases

1. **Limited Disk Space**: Analyze 500GB disk image with only 50GB free space
2. **Network Mounts**: Work with images on NFS/SMB without local copies
3. **Cloud Integration**: Stream extracted files directly to S3/Azure Storage
4. **Real-time Acquisition**: Stream from physical disk to remote server
5. **Worker Queues**: Process forensic tasks without storing entire images

---

## Current Streaming Capabilities

### 1. Memory-Mapped I/O (Zero-Copy Reads)

**Location**: `crates/totalimage-pipeline/src/mmap.rs`

TotalImage uses `mmap()` to access disk images without loading them into RAM:

```rust
// Open 500GB VHD but use <10MB RAM
let vault = open_vault("/data/500gb-disk.vhd", VaultConfig::default())?;

// Jump to partition at offset 250GB instantly
vault.seek(250 * 1024 * 1024 * 1024)?;

// Read just 4KB sector
let sector = vault.read(4096)?;
```

**Benefits**:
- ✅ Constant memory usage regardless of image size
- ✅ Instant seeking to any offset
- ✅ OS handles caching automatically
- ✅ Works on network mounts (NFS, SMB)

### 2. Partial Pipeline (Window into Partitions)

**Location**: `crates/totalimage-pipeline/src/partial.rs`

Access specific partitions without reading entire disk:

```rust
// Create view into partition at offset 10GB, length 100GB
let partial = PartialPipeline::new(vault.content(), 10_737_418_240, 107_374_182_400)?;

// All reads are relative to partition start
partial.seek(SeekFrom::Start(0))?;
partial.read(&mut buffer)?;
```

### 3. Filesystem Streaming

**Location**: `crates/totalimage-territories/`

Filesystems parse metadata without reading file contents:

```rust
// Parse FAT filesystem (reads only FAT table, ~1MB)
let fat = FatTerritory::parse(&mut partial)?;

// List 10,000 files by reading directory entries only
let files = fat.list_files(&mut partial)?;

// Extract single 5GB file without reading other files
let entry = fat.find_file("large-video.mp4")?;
let data = fat.read_file_data(&mut partial, &entry)?;
```

---

## Disk Space Scenarios

### Scenario 1: Limited Local Disk Space

**Problem**: Need to analyze 500GB VHD but only have 50GB free disk space.

**Solution**: TotalImage never copies the image. All operations work directly on the source file:

```bash
# Analyze image on network mount
export TOTALIMAGE_ALLOWED_ROOT=/mnt/network-storage

# Start API server (uses <50MB RAM)
totalimage-web

# List files (streams from network)
curl "http://localhost:3000/api/vault/files?path=/mnt/network-storage/500gb.vhd&zone=0"

# Extract single file (streams without full disk read)
totalimage-cli extract /mnt/network-storage/500gb.vhd "/Users/suspect/evidence.pdf" \
  --output ./evidence.pdf
```

**Memory usage**: ~10-50MB regardless of image size
**Disk space required**: 0 bytes (reads directly from source)

### Scenario 2: Cloud Worker with Minimal Storage

**Problem**: Cloud VM has only 20GB disk but needs to extract files from 100GB images stored in S3.

**Solution**: Mount S3 as filesystem, stream extraction directly to destination:

```bash
# Mount S3 bucket (using s3fs-fuse)
s3fs evidence-bucket /mnt/s3 -o use_cache=/tmp/s3cache,max_cache_size=1024

# Extract file and stream to output bucket
totalimage-cli extract /mnt/s3/disk-images/evidence.vhd "/Documents/report.pdf" \
  --output /tmp/extracted.pdf

# Upload to destination (can stream this too)
aws s3 cp /tmp/extracted.pdf s3://processed-evidence/
rm /tmp/extracted.pdf
```

### Scenario 3: Real-time Acquisition Streaming

**Problem**: Need to acquire disk image from suspect computer with limited storage.

**Solution** (Future Enhancement - see below):

```bash
# On suspect computer:
totalimage-acquire --source /dev/sda \
  --stream-to https://forensics-server.com/api/vault/upload \
  --chunk-size 100MB

# Server receives and processes in real-time without storing entire disk
```

---

## API Streaming Support

### Current Endpoints (Read-Only Streaming)

All existing endpoints use streaming internally:

#### 1. Vault Info
```bash
GET /api/vault/info?path=/data/huge-disk.vhd
```
- Reads only vault header (512 bytes - 64KB)
- Memory usage: <1MB

#### 2. List Partitions
```bash
GET /api/vault/zones?path=/data/huge-disk.vhd
```
- Reads MBR (512 bytes) or GPT header (~34KB)
- Memory usage: <1MB

#### 3. List Files
```bash
GET /api/vault/files?path=/data/huge-disk.vhd&zone=0
```
- Reads filesystem metadata only (FAT table, MFT, etc.)
- Memory usage: ~10-50MB depending on filesystem size
- Does NOT read file contents

### Proposed: File Extraction Endpoint

**Endpoint**: `GET /api/vault/extract`

**Query Parameters**:
- `path` (required): Vault path
- `zone` (required): Partition index
- `file` (required): File path within filesystem
- `stream` (optional): Enable chunked streaming (default: false)
- `chunk_size` (optional): Chunk size in bytes (default: 1MB)

**Standard Response** (buffers entire file):
```bash
curl "http://localhost:3000/api/vault/extract?path=/data/disk.vhd&zone=0&file=/report.pdf" \
  --output report.pdf
```

**Streaming Response** (chunked transfer encoding):
```bash
curl "http://localhost:3000/api/vault/extract?path=/data/disk.vhd&zone=0&file=/large.mp4&stream=true&chunk_size=10485760" \
  --output large.mp4
```

**Implementation** (pseudo-code):
```rust
async fn extract_file(Query(params): Query<ExtractParams>) -> impl IntoResponse {
    // Open vault (mmap, no disk usage)
    let vault = open_vault(&params.path)?;

    // Get partition (no data read)
    let zone = get_zone(&vault, params.zone)?;
    let mut partial = PartialPipeline::new(vault.content(), zone.offset, zone.length)?;

    // Parse filesystem (reads metadata only)
    let fs = parse_filesystem(&mut partial)?;

    // Find file entry (directory traversal, minimal reads)
    let entry = fs.find_file(&params.file)?;

    if params.stream {
        // Stream in chunks
        let stream = extract_file_chunked(&mut partial, &fs, &entry, params.chunk_size);
        return StreamBody::new(stream);
    } else {
        // Buffer entire file (use for small files <10MB)
        let data = fs.read_file_data(&mut partial, &entry)?;
        return ([(header::CONTENT_TYPE, "application/octet-stream")], data);
    }
}
```

---

## Worker Integration

### Pyro Worker Example

**Location**: `packages/pyro-worker-totalimage/src/worker.ts`

**Current state**: Workers can list files but not extract them via API (must use CLI).

**Proposed Enhancement**:

```typescript
import { Readable } from 'stream';
import { S3 } from '@aws-sdk/client-s3';
import { Upload } from '@aws-sdk/lib-storage';

interface ExtractTask {
  vaultPath: string;
  zone: number;
  filePath: string;
  outputBucket: string;
  outputKey: string;
}

async function extractToS3(task: ExtractTask): Promise<void> {
  const extractUrl = new URL('/api/vault/extract', TOTALIMAGE_API_URL);
  extractUrl.searchParams.set('path', task.vaultPath);
  extractUrl.searchParams.set('zone', task.zone.toString());
  extractUrl.searchParams.set('file', task.filePath);
  extractUrl.searchParams.set('stream', 'true');
  extractUrl.searchParams.set('chunk_size', (10 * 1024 * 1024).toString()); // 10MB chunks

  // Fetch as stream
  const response = await fetch(extractUrl.toString());
  if (!response.ok) {
    throw new Error(`Extraction failed: ${response.statusText}`);
  }

  // Convert web stream to Node.js stream
  const webStream = response.body!;
  const nodeStream = Readable.fromWeb(webStream);

  // Stream directly to S3 (no local disk usage!)
  const s3Client = new S3({ region: 'us-east-1' });
  const upload = new Upload({
    client: s3Client,
    params: {
      Bucket: task.outputBucket,
      Key: task.outputKey,
      Body: nodeStream,
    },
  });

  await upload.done();

  console.log(`✓ Streamed ${task.filePath} to s3://${task.outputBucket}/${task.outputKey}`);
  // Total disk usage: ~10MB (chunk buffer)
}

// Usage
await extractToS3({
  vaultPath: '/mnt/evidence/disk.vhd',
  zone: 0,
  filePath: '/Users/suspect/database.sqlite',
  outputBucket: 'forensic-evidence',
  outputKey: 'case-123/database.sqlite',
});
```

**Benefits**:
- ✅ Extract 100GB files with only 10MB RAM
- ✅ No local disk space required
- ✅ Parallel extraction to multiple destinations
- ✅ Progress tracking via chunk callbacks

---

## Advanced Streaming Scenarios

### Scenario 4: HTTP Range Requests for Remote Vaults

**Use case**: Disk image is on remote HTTP server, no local copy.

**Proposed Enhancement**:

```bash
# Analyze image directly from HTTP URL
curl "http://localhost:3000/api/vault/info?path=https://evidence-server.com/images/disk.vhd"
```

**Implementation**:
- Detect `http://` or `https://` in path
- Use HTTP Range requests (`Range: bytes=0-511`) to read specific offsets
- Cache frequently accessed sectors (headers, FAT tables)
- Vault and filesystem parsers work unchanged (they just call `seek()` and `read()`)

**Example**:
```rust
struct HttpVault {
    url: String,
    client: reqwest::Client,
    size: u64,
}

impl Read for HttpVault {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // Current position tracked internally
        let range_header = format!("bytes={}-{}", self.position, self.position + buf.len() - 1);

        let response = self.client.get(&self.url)
            .header("Range", range_header)
            .send()
            .await?;

        let data = response.bytes().await?;
        buf[..data.len()].copy_from_slice(&data);

        self.position += data.len() as u64;
        Ok(data.len())
    }
}
```

### Scenario 5: Parallel Chunk Extraction

**Use case**: Extract 10GB file faster by downloading chunks in parallel.

**Proposed Enhancement**:

```bash
# Request file in 100MB chunks
curl "http://localhost:3000/api/vault/extract?path=/data/disk.vhd&zone=0&file=/large.mp4&chunk=0&chunk_size=104857600"
curl "http://localhost:3000/api/vault/extract?path=/data/disk.vhd&zone=0&file=/large.mp4&chunk=1&chunk_size=104857600"
...

# Client reassembles chunks
cat chunk-*.bin > large.mp4
```

---

## Performance Characteristics

| Operation | Memory Usage | Disk Space | Network I/O |
|-----------|-------------|------------|-------------|
| Open 500GB vault | ~10MB | 0 bytes | 0 bytes (local) / ~64KB (HTTP) |
| List partitions | ~1MB | 0 bytes | ~34KB (GPT header) |
| Parse FAT32 filesystem | ~10-50MB | 0 bytes | ~1-10MB (FAT table) |
| List 10,000 files | ~20MB | 0 bytes | ~2MB (directory entries) |
| Extract 5GB file (buffered) | 5GB | 5GB temp | 5GB |
| Extract 5GB file (streaming) | ~10MB | 0 bytes | 5GB (streamed) |

**Key Insight**: Streaming extraction uses **constant memory and zero disk space** regardless of file size.

---

## Implementation Roadmap

### Phase 1: Basic Extraction Endpoint (Priority: High)
- [x] Design API endpoint schema
- [ ] Implement `/api/vault/extract` with buffered response
- [ ] Add FAT filesystem extraction
- [ ] Add NTFS filesystem extraction
- [ ] Add exFAT filesystem extraction
- [ ] Add ISO-9660 filesystem extraction
- [ ] Add authentication and rate limiting
- [ ] Update API documentation

### Phase 2: Chunked Streaming (Priority: High)
- [ ] Implement chunked transfer encoding response
- [ ] Add `stream=true` parameter
- [ ] Add `chunk_size` parameter
- [ ] Test with files >10GB
- [ ] Update worker integration examples

### Phase 3: HTTP Remote Vaults (Priority: Medium)
- [ ] Implement `HttpVault` for remote images
- [ ] Add HTTP Range request support
- [ ] Add caching for frequently accessed sectors
- [ ] Handle authentication (S3 signed URLs, Bearer tokens)
- [ ] Test with S3, Azure Blob, Google Cloud Storage

### Phase 4: Advanced Features (Priority: Low)
- [ ] Parallel chunk extraction
- [ ] Resume interrupted extractions
- [ ] Compression on-the-fly (gzip/zstd)
- [ ] Deduplication detection
- [ ] Sparse file support

---

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `TOTALIMAGE_STREAM_CHUNK_SIZE` | Default chunk size for streaming | `1048576` (1MB) |
| `TOTALIMAGE_STREAM_MAX_BUFFER` | Max buffered file size (larger files force streaming) | `10485760` (10MB) |
| `TOTALIMAGE_HTTP_TIMEOUT` | Timeout for HTTP vault requests | `30s` |
| `TOTALIMAGE_HTTP_CACHE_SIZE` | HTTP response cache size | `256MB` |

### Example Configuration

```bash
# .env
TOTALIMAGE_ALLOWED_ROOT=/mnt/evidence
TOTALIMAGE_STREAM_CHUNK_SIZE=10485760  # 10MB chunks
TOTALIMAGE_STREAM_MAX_BUFFER=52428800   # 50MB max buffer
TOTALIMAGE_HTTP_TIMEOUT=60              # 60 second timeout
```

---

## Security Considerations

### Path Traversal Prevention

All file paths are validated against `TOTALIMAGE_ALLOWED_ROOT`:

```rust
// ❌ Rejected: Path outside allowed root
/api/vault/extract?path=/etc/passwd&zone=0&file=/shadow

// ✅ Allowed: Path within allowed root
/api/vault/extract?path=/mnt/evidence/disk.vhd&zone=0&file=/Documents/file.pdf
```

### Denial of Service Prevention

1. **Rate Limiting**: 100 requests/second per IP
2. **Concurrency Limits**: 10 concurrent extractions
3. **Timeout**: 30 second max request time
4. **Size Limits**: Files >1GB require `stream=true`
5. **Memory Limits**: Buffered responses capped at 50MB

### Data Integrity

1. **Read-Only Access**: All vault operations are read-only
2. **Checksum Verification**: Optional SHA256 verification for extractions
3. **Audit Logging**: All extraction requests logged with user, IP, timestamp

---

## Monitoring

### Metrics

Expose Prometheus metrics at `/metrics`:

```
# Vault operations
totalimage_vault_opens_total{status="success|error"}
totalimage_vault_open_duration_seconds

# Extraction operations
totalimage_extractions_total{filesystem="fat|ntfs|exfat|iso", status="success|error"}
totalimage_extraction_bytes_total{filesystem="fat|ntfs|exfat|iso"}
totalimage_extraction_duration_seconds

# Streaming operations
totalimage_stream_chunks_sent_total
totalimage_stream_bytes_sent_total
totalimage_stream_errors_total
```

### Example Grafana Dashboard

```
Rate of successful extractions: rate(totalimage_extractions_total{status="success"}[5m])
Average extraction size: rate(totalimage_extraction_bytes_total[5m]) / rate(totalimage_extractions_total[5m])
P95 extraction latency: histogram_quantile(0.95, totalimage_extraction_duration_seconds)
```

---

## Testing

### Unit Tests

```rust
#[tokio::test]
async fn test_streaming_extraction() {
    let vault = create_test_vault_with_5gb_file();

    let response = extract_file_streaming(ExtractParams {
        path: vault.path(),
        zone: 0,
        file: "/large-file.bin".to_string(),
        stream: true,
        chunk_size: 1_048_576, // 1MB
    }).await.unwrap();

    let mut total_bytes = 0;
    let mut chunks = 0;

    while let Some(chunk) = response.next_chunk().await {
        total_bytes += chunk.len();
        chunks += 1;
    }

    assert_eq!(total_bytes, 5_368_709_120); // 5GB
    assert_eq!(chunks, 5120); // 5GB / 1MB
}
```

### Integration Tests

```bash
# Test extraction from 100GB image
./tests/test-large-extraction.sh

# Test streaming to S3
./tests/test-s3-streaming.sh

# Test HTTP vault access
./tests/test-http-vault.sh
```

---

## See Also

- [Architecture Documentation](ARCHITECTURE.md)
- [API Reference](API.md)
- [Production Deployment](../steering/PRODUCTION-DEPLOYMENT.md)
- [Performance Tuning](PERFORMANCE.md)
