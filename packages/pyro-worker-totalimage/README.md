# @pyro/worker-totalimage

PYRO Platform worker package for TotalImage disk image analysis.

## Installation

```bash
npm install @pyro/worker-totalimage
```

## Quick Start

### Standalone Client

```typescript
import { createMCPClient } from '@pyro/worker-totalimage';

const client = createMCPClient({
  url: 'http://localhost:3002',
  timeout: 30000,
  apiKey: process.env.MCP_API_KEY,
});

// Analyze a disk image
const analysis = await client.analyzeImage({
  path: '/images/evidence.e01',
  deep_scan: false,
  cache: true,
});

console.log(`Vault type: ${analysis.vault_type}`);
console.log(`Partitions: ${analysis.partitions.length}`);
```

### Job Queue Worker

```typescript
import { createWorker } from '@pyro/worker-totalimage';

const worker = createWorker({
  mcp: {
    url: process.env.TOTALIMAGE_MCP_URL || 'http://localhost:3002',
    timeout: 30000,
    apiKey: process.env.MCP_API_KEY,
  },
  queue: {
    redis: process.env.REDIS_URL || 'redis://localhost:6379',
    concurrency: 5,
    maxRetries: 3,
  },
  logging: {
    level: 'info',
    format: 'json',
  },
});

// Start processing jobs
await worker.start();

// Add jobs programmatically
const jobId = await worker.addJob({
  job_type: 'analyze_disk_image',
  user_id: 'user-123',
  project_id: 'project-456',
  input: {
    path: '/images/disk.vhd',
    deep_scan: true,
  },
});

// Monitor queue
const stats = await worker.getStats();
console.log(`Active: ${stats.active}, Waiting: ${stats.waiting}`);

// Graceful shutdown
process.on('SIGTERM', () => worker.stop());
```

## API Reference

### Client Methods

#### `analyzeImage(input)`
Analyze a disk image file for vault type, partitions, and filesystems.

#### `listPartitions(input)`
List all partitions in a disk image.

#### `listFiles(input)`
List files in a specific partition.

#### `extractFile(input)`
Extract a file from a disk image.

#### `validateIntegrity(input)`
Validate disk image integrity with checksums.

### Job Types

| Job Type | Description |
|----------|-------------|
| `analyze_disk_image` | Full disk analysis |
| `list_partitions` | Partition enumeration |
| `list_files` | Directory listing |
| `extract_file` | File extraction |
| `validate_integrity` | Integrity check |
| `batch_analyze` | Analyze multiple images |
| `batch_extract` | Extract multiple files |

### Configuration

```typescript
interface WorkerConfig {
  mcp: {
    url: string;          // MCP server URL
    timeout: number;      // Request timeout (ms)
    apiKey?: string;      // Optional API key
    retries?: number;     // Retry attempts (default: 3)
    retryDelay?: number;  // Base retry delay (default: 1000ms)
  };
  queue: {
    redis: string;        // Redis connection URL
    concurrency: number;  // Concurrent jobs
    maxRetries: number;   // Job retry attempts
    backoffType?: 'exponential' | 'fixed';
    backoffDelay?: number;
  };
  storage?: {
    type: 's3' | 'local' | 'minio';
    bucket?: string;
    region?: string;
    endpoint?: string;
    localPath?: string;
  };
  logging?: {
    level: 'debug' | 'info' | 'warn' | 'error';
    format?: 'json' | 'pretty';
  };
}
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `TOTALIMAGE_MCP_URL` | MCP server URL | `http://localhost:3002` |
| `MCP_API_KEY` | API key for authentication | - |
| `REDIS_URL` | Redis connection URL | `redis://localhost:6379` |
| `LOG_LEVEL` | Logging level | `info` |

## Supported Image Formats

- **Raw** (.img, .raw, .bin, .dd)
- **VHD** (.vhd) - Fixed, Dynamic, Differencing
- **E01** (.e01) - EnCase forensic format
- **AFF4** (.aff4) - Advanced Forensic Format 4

## License

GPL-3.0
