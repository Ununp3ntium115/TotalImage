# TotalImage Quick Start Guide

Get up and running with TotalImage in 5 minutes!

## Installation

### Option 1: Pre-built Binaries (Coming Soon)

```bash
# Download latest release
curl -LO https://github.com/Ununp3ntium115/TotalImage/releases/latest/download/totalimage-linux-x86_64.tar.gz

# Extract
tar -xzf totalimage-linux-x86_64.tar.gz

# Install
sudo mv totalimage /usr/local/bin/
```

### Option 2: Build from Source

**Prerequisites:** Rust 1.75+

```bash
# Clone repository
git clone https://github.com/Ununp3ntium115/TotalImage.git
cd TotalImage

# Build release
cargo build --release

# Binaries will be in target/release/
./target/release/totalimage --version
```

### Option 3: Docker

```bash
docker pull totalimage/totalimage:latest
docker run -p 3000:3000 -v /data:/data totalimage/totalimage
```

---

## Quick Example: Analyze a Disk Image

### CLI Usage

```bash
# 1. Check image info
$ totalimage info /path/to/evidence.vhd

Vault: /path/to/evidence.vhd
Type: VHD Dynamic
Size: 10.0 GB

Partition Table: MBR
Partitions: 2

# 2. List partitions
$ totalimage zones /path/to/evidence.vhd

Zone 0: NTFS (0x07)
  Offset: 1048576
  Length: 5368709120

Zone 1: FAT32 LBA (0x0C)
  Offset: 5369757696
  Length: 5367660544

# 3. Browse files
$ totalimage list /path/to/evidence.vhd --zone 1

Documents/       [DIR]
Pictures/        [DIR]
readme.txt       1.2 KB
report.pdf       2.4 MB

# 4. Extract a file
$ totalimage extract /path/to/evidence.vhd "/report.pdf" \
    --zone 1 \
    --output ./extracted/report.pdf

✓ Extracted 2.4 MB to ./extracted/report.pdf
```

---

## Quick Example: REST API

### Start the Server

```bash
# Set allowed paths (security requirement)
export TOTALIMAGE_ALLOWED_ROOT=/data/evidence

# Start server
totalimage-web

# Server running at http://localhost:3000
```

### Make Requests

```bash
# Get vault info
curl "http://localhost:3000/api/vault/info?path=/data/evidence/disk.vhd"

# List partitions
curl "http://localhost:3000/api/vault/zones?path=/data/evidence/disk.vhd"

# Browse files
curl "http://localhost:3000/api/vault/files?path=/data/evidence/disk.vhd&zone=1"
```

---

## Client Libraries

### Python

```python
import requests

class TotalImageClient:
    def __init__(self, base_url="http://localhost:3000"):
        self.base_url = base_url

    def get_vault_info(self, path):
        response = requests.get(
            f"{self.base_url}/api/vault/info",
            params={"path": path}
        )
        return response.json()

    def list_files(self, path, zone=0, directory="/"):
        response = requests.get(
            f"{self.base_url}/api/vault/files",
            params={
                "path": path,
                "zone": zone,
                "directory": directory
            }
        )
        return response.json()

# Usage
client = TotalImageClient()

# Get info
info = client.get_vault_info("/data/evidence.vhd")
print(f"Vault type: {info['vault_type']}")
print(f"Size: {info['size_bytes']} bytes")

# List files
files = client.list_files("/data/evidence.vhd", zone=1)
for file in files['files']:
    print(f"{file['name']:30} {file['size']:>10} bytes")
```

### JavaScript/TypeScript

```typescript
class TotalImageClient {
  constructor(private baseUrl: string = 'http://localhost:3000') {}

  async getVaultInfo(path: string) {
    const response = await fetch(
      `${this.baseUrl}/api/vault/info?path=${encodeURIComponent(path)}`
    );
    return response.json();
  }

  async listFiles(path: string, zone: number = 0, directory: string = '/') {
    const params = new URLSearchParams({
      path,
      zone: zone.toString(),
      directory
    });

    const response = await fetch(
      `${this.baseUrl}/api/vault/files?${params}`
    );
    return response.json();
  }
}

// Usage
const client = new TotalImageClient();

// Get info
const info = await client.getVaultInfo('/data/evidence.vhd');
console.log(`Vault type: ${info.vault_type}`);
console.log(`Size: ${info.size_bytes} bytes`);

// List files
const files = await client.listFiles('/data/evidence.vhd', 1);
files.files.forEach(file => {
  console.log(`${file.name.padEnd(30)} ${file.size.toString().padStart(10)} bytes`);
});
```

### Go

```go
package main

import (
    "encoding/json"
    "fmt"
    "net/http"
    "net/url"
)

type TotalImageClient struct {
    BaseURL string
}

type VaultInfo struct {
    Path      string `json:"path"`
    VaultType string `json:"vault_type"`
    SizeBytes uint64 `json:"size_bytes"`
}

func (c *TotalImageClient) GetVaultInfo(path string) (*VaultInfo, error) {
    params := url.Values{}
    params.Add("path", path)

    resp, err := http.Get(c.BaseURL + "/api/vault/info?" + params.Encode())
    if err != nil {
        return nil, err
    }
    defer resp.Body.Close()

    var info VaultInfo
    if err := json.NewDecoder(resp.Body).Decode(&info); err != nil {
        return nil, err
    }

    return &info, nil
}

func main() {
    client := &TotalImageClient{BaseURL: "http://localhost:3000"}

    info, err := client.GetVaultInfo("/data/evidence.vhd")
    if err != nil {
        panic(err)
    }

    fmt.Printf("Vault type: %s\n", info.VaultType)
    fmt.Printf("Size: %d bytes\n", info.SizeBytes)
}
```

### Rust

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct VaultInfo {
    path: String,
    vault_type: String,
    size_bytes: u64,
}

struct TotalImageClient {
    base_url: String,
    client: reqwest::Client,
}

impl TotalImageClient {
    fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: reqwest::Client::new(),
        }
    }

    async fn get_vault_info(&self, path: &str) -> Result<VaultInfo, Box<dyn std::error::Error>> {
        let url = format!("{}/api/vault/info?path={}", self.base_url, path);
        let info: VaultInfo = self.client.get(&url).send().await?.json().await?;
        Ok(info)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = TotalImageClient::new("http://localhost:3000");

    let info = client.get_vault_info("/data/evidence.vhd").await?;
    println!("Vault type: {}", info.vault_type);
    println!("Size: {} bytes", info.size_bytes);

    Ok(())
}
```

---

## Docker Compose Example

```yaml
version: '3.8'

services:
  totalimage:
    image: totalimage/totalimage:latest
    ports:
      - "3000:3000"  # Web API
      - "3001:3001"  # Fire Marshal
      - "3002:3002"  # MCP Server
    volumes:
      - /path/to/evidence:/data/images:ro  # Read-only evidence mount
      - totalimage-cache:/data/cache
    environment:
      - TOTALIMAGE_ALLOWED_ROOT=/data/images
      - TOTALIMAGE_CACHE_DIR=/data/cache
      - RUST_LOG=info
      - TOTALIMAGE_AUTH_ENABLED=true
      - TOTALIMAGE_API_KEYS=secret-key-1,secret-key-2
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/health"]
      interval: 30s
      timeout: 10s
      retries: 3

volumes:
  totalimage-cache:
```

**Usage:**

```bash
# Start services
docker-compose up -d

# View logs
docker-compose logs -f totalimage

# Stop services
docker-compose down
```

---

## Kubernetes Deployment

**1. Create namespace:**
```bash
kubectl create namespace forensics
```

**2. Deploy application:**
```bash
kubectl apply -f k8s/
```

**3. Verify deployment:**
```bash
kubectl get pods -n forensics
kubectl get svc -n forensics
```

**4. Access service:**
```bash
# Port-forward for testing
kubectl port-forward -n forensics svc/totalimage 3000:3000

# Or use Ingress URL
curl http://totalimage.forensics.local/health
```

---

## Authentication Setup

### API Key Auth

```bash
# Set API keys (comma-separated)
export TOTALIMAGE_AUTH_ENABLED=true
export TOTALIMAGE_API_KEYS="key1,key2,key3"

# Start server
totalimage-web
```

**Usage:**
```bash
curl -H "X-API-Key: key1" \
  "http://localhost:3000/api/vault/info?path=/data/disk.vhd"
```

### JWT Auth

```bash
# Set JWT secret
export TOTALIMAGE_AUTH_ENABLED=true
export TOTALIMAGE_JWT_SECRET="your-secret-key-min-32-chars"

# Start server
totalimage-web
```

**Generate Token (example with Python):**
```python
import jwt
import datetime

payload = {
    'sub': 'user@example.com',
    'exp': datetime.datetime.utcnow() + datetime.timedelta(hours=1)
}

token = jwt.encode(payload, 'your-secret-key-min-32-chars', algorithm='HS256')
print(f"Token: {token}")
```

**Usage:**
```bash
curl -H "Authorization: Bearer eyJ..." \
  "http://localhost:3000/api/vault/zones?path=/data/disk.vhd"
```

---

## Troubleshooting

### Server Won't Start

**Error:** `TOTALIMAGE_ALLOWED_ROOT must be set`

**Solution:**
```bash
export TOTALIMAGE_ALLOWED_ROOT=/data
totalimage-web
```

### Permission Denied

**Error:** `Permission denied: /data/evidence.vhd`

**Solutions:**
1. Check file permissions: `ls -l /data/evidence.vhd`
2. Run with appropriate user/group
3. Use Docker volume with correct permissions

### Empty Response

**Error:** API returns empty or null data

**Solutions:**
1. Check path is within `TOTALIMAGE_ALLOWED_ROOT`
2. Verify file format is supported
3. Enable debug logging: `RUST_LOG=debug totalimage-web`

### Slow Performance

**Solutions:**
1. Ensure cache directory is on fast storage (SSD)
2. Increase cache size (default: 256MB)
3. Use local files instead of network mounts
4. Check memory-mapped I/O limits

---

## Next Steps

- 📖 Read the [API Documentation](API.md)
- 🏗️ Understand the [Architecture](ARCHITECTURE.md)
- 🤝 Learn how to [Contribute](../CONTRIBUTING.md)
- 🔒 Review [Security Practices](../SECURITY.md)

---

## Getting Help

- **Issues:** https://github.com/Ununp3ntium115/TotalImage/issues
- **Discussions:** https://github.com/Ununp3ntium115/TotalImage/discussions
- **Security:** Email security@totalimage.com
