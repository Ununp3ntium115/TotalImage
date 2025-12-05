# TotalImage REST API Documentation

## Overview

TotalImage provides a RESTful API for analyzing forensic disk images, parsing partition tables, and extracting files from filesystems.

**Base URL:** `http://localhost:3000`
**Version:** 0.1.0
**Authentication:** JWT or API Key (optional, configured via environment)

---

## Endpoints

### Health Check

#### `GET /health`

Check if the server is running and healthy.

**Response:**
```json
{
  "status": "ok",
  "version": "0.1.0",
  "uptime_seconds": 3600,
  "cache_stats": {
    "vault_info_count": 42,
    "zone_table_count": 15,
    "dir_listings_count": 128,
    "estimated_size_bytes": 2097152
  }
}
```

---

### Vault Information

#### `GET /api/vault/info`

Get metadata about a disk image container.

**Query Parameters:**
- `path` (required): Path to the disk image file

**Example:**
```bash
curl "http://localhost:3000/api/vault/info?path=/data/evidence.vhd"
```

**Response:**
```json
{
  "path": "/data/evidence.vhd",
  "vault_type": "VHD Dynamic",
  "size_bytes": 10737418240,
  "partition_table": {
    "table_type": "GPT",
    "partition_count": 3,
    "disk_guid": "a1b2c3d4-e5f6-7890-abcd-ef0123456789"
  }
}
```

---

### List Partitions

#### `GET /api/vault/zones`

List all partitions (zones) in a disk image.

**Query Parameters:**
- `path` (required): Path to the disk image file

**Example:**
```bash
curl "http://localhost:3000/api/vault/zones?path=/data/evidence.e01"
```

**Response:**
```json
{
  "path": "/data/evidence.e01",
  "partition_table": "MBR",
  "zones": [
    {
      "index": 0,
      "offset": 1048576,
      "length": 536870912,
      "zone_type": "NTFS (0x07)"
    },
    {
      "index": 1,
      "offset": 537919488,
      "length": 10200547328,
      "zone_type": "FAT32 LBA (0x0C)"
    }
  ]
}
```

---

### List Files in Partition

#### `GET /api/vault/files`

List files and directories in a partition's filesystem.

**Query Parameters:**
- `path` (required): Path to the disk image file
- `zone` (optional): Partition index (default: 0)
- `directory` (optional): Directory path within filesystem (default: "/")
- `offset` (optional): Pagination offset (default: 0)
- `limit` (optional): Max results to return (default: 100, max: 1000)

**Example:**
```bash
curl "http://localhost:3000/api/vault/files?path=/data/disk.vhd&zone=1&directory=/Users/alice&limit=50"
```

**Response:**
```json
{
  "path": "/data/disk.vhd",
  "zone_index": 1,
  "directory": "/Users/alice",
  "total_entries": 127,
  "offset": 0,
  "limit": 50,
  "files": [
    {
      "name": "Documents",
      "size": 0,
      "is_directory": true,
      "created": "2024-01-15T14:30:00Z",
      "modified": "2024-03-20T09:15:30Z"
    },
    {
      "name": "report.pdf",
      "size": 2458624,
      "is_directory": false,
      "created": "2024-03-10T11:22:15Z",
      "modified": "2024-03-10T11:22:45Z"
    }
  ]
}
```

---

## Authentication

### API Key Authentication

Include the API key in the `X-API-Key` header:

```bash
curl -H "X-API-Key: your-api-key-here" \
  "http://localhost:3000/api/vault/info?path=/data/image.vhd"
```

### JWT Authentication

Include the JWT token in the `Authorization` header:

```bash
curl -H "Authorization: Bearer your-jwt-token-here" \
  "http://localhost:3000/api/vault/zones?path=/data/image.e01"
```

---

## Error Responses

All errors follow this format:

```json
{
  "error": "Detailed error message",
  "code": "ERROR_CODE",
  "details": {
    "field": "additional context"
  }
}
```

### Common Error Codes

| Status Code | Description |
|-------------|-------------|
| 400 | Bad Request - Invalid parameters |
| 401 | Unauthorized - Missing or invalid authentication |
| 403 | Forbidden - Access denied to file path |
| 404 | Not Found - File or partition doesn't exist |
| 429 | Too Many Requests - Rate limit exceeded |
| 500 | Internal Server Error - Unexpected failure |
| 503 | Service Unavailable - Server overloaded |

---

## Rate Limiting

- **Global Limit:** 100 requests/second
- **Concurrency Limit:** 10 concurrent requests
- **Request Timeout:** 30 seconds

Exceeded rate limits return `429 Too Many Requests`:

```json
{
  "error": "Rate limit exceeded. Try again in 1 second."
}
```

---

## Caching

Responses are cached for 30 days to improve performance:
- Vault metadata
- Partition tables
- Directory listings

Cache headers:
```
X-Cache: HIT
Cache-Control: public, max-age=2592000
```

---

## Examples

### Complete Workflow: Extract File from VHD

```bash
# 1. Get vault info
curl "http://localhost:3000/api/vault/info?path=/data/evidence.vhd"

# 2. List partitions
curl "http://localhost:3000/api/vault/zones?path=/data/evidence.vhd"

# 3. Browse files in partition 1
curl "http://localhost:3000/api/vault/files?path=/data/evidence.vhd&zone=1"

# 4. Extract specific file (via CLI, API extraction coming soon)
totalimage-cli extract /data/evidence.vhd "/Users/suspect/document.pdf" \
  --zone 1 --output ./extracted/document.pdf
```

### Batch Processing

```bash
# Process multiple images
for image in /data/*.vhd; do
  echo "Processing: $image"
  curl "http://localhost:3000/api/vault/zones?path=$image" | jq
done
```

---

## Configuration

Environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `TOTALIMAGE_WEB_ADDR` | Listen address | `127.0.0.1:3000` |
| `TOTALIMAGE_WEB_ALLOWED_ROOT` | Allowed file paths | Required |
| `TOTALIMAGE_CACHE_DIR` | Cache directory | `~/.cache/totalimage` |
| `TOTALIMAGE_WEB_MAX_CONCURRENT` | Max concurrent requests | `10` |
| `TOTALIMAGE_AUTH_ENABLED` | Enable authentication | `false` |
| `TOTALIMAGE_API_KEYS` | Comma-separated API keys | - |
| `TOTALIMAGE_JWT_SECRET` | JWT signing secret | - |

---

## See Also

- [CLI Documentation](CLI.md)
- [MCP Integration](MCP.md)
- [Deployment Guide](../steering/PRODUCTION-DEPLOYMENT.md)
