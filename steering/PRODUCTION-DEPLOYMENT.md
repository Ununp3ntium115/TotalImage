# TotalImage Production Deployment Guide

**Version:** 1.0.0
**Last Updated:** 2025-12-02
**Status:** Production Ready

## Overview

This guide covers deploying TotalImage in production environments with:
- ✅ Rate limiting and request timeouts
- ✅ TLS/HTTPS encryption
- ✅ Prometheus metrics
- ✅ Docker containerization
- ✅ High availability setup
- ✅ Monitoring and alerting

---

## Table of Contents

1. [Quick Start](#quick-start)
2. [Architecture](#architecture)
3. [Deployment Options](#deployment-options)
4. [Configuration](#configuration)
5. [Security Hardening](#security-hardening)
6. [Monitoring](#monitoring)
7. [Scaling](#scaling)
8. [Troubleshooting](#troubleshooting)

---

## Quick Start

### Docker Compose (Recommended)

```bash
# Clone repository
git clone https://github.com/Ununp3ntium115/TotalImage.git
cd TotalImage

# Start all services
docker-compose up -d

# Check service health
curl http://localhost:3000/health  # Web API
curl http://localhost:3002/health  # MCP Server
curl http://localhost:3001/health  # Fire Marshal
curl http://localhost:3002/metrics # Prometheus metrics
```

**Services Started:**
- **totalimage-web**: REST API (`:3000`)
- **totalimage-mcp**: MCP Server (`:3002`)
- **fire-marshal**: Tool orchestration (`:3001`)
- **node-red**: Visual workflows (`:1880`)

### Manual Binary Installation

```bash
# Build release binaries
cargo build --release --workspace

# Install binaries
sudo cp target/release/totalimage /usr/local/bin/
sudo cp target/release/totalimage-web /usr/local/bin/
sudo cp target/release/totalimage-mcp /usr/local/bin/
sudo cp target/release/fire-marshal /usr/local/bin/

# Create system user
sudo useradd -r -s /bin/false totalimage

# Create directories
sudo mkdir -p /var/lib/totalimage/{cache,images}
sudo chown -R totalimage:totalimage /var/lib/totalimage

# Run web server
sudo -u totalimage totalimage-web
```

---

## Architecture

### Component Overview

```
┌─────────────────────────────────────────────────────────────┐
│                     Reverse Proxy (nginx/Traefik)            │
│                    TLS Termination + Rate Limiting            │
└─────────────────────────────────────────────────────────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        │                     │                     │
        ▼                     ▼                     ▼
┌──────────────┐      ┌──────────────┐     ┌──────────────┐
│  Web API     │      │  MCP Server  │     │ Fire Marshal │
│  Port 3000   │      │  Port 3002   │     │  Port 3001   │
│              │      │              │     │              │
│ ✓ Rate Limit │      │ ✓ JWT Auth   │     │ ✓ Rate Limit │
│ ✓ Timeouts   │      │ ✓ WebSocket  │     │ ✓ Registry   │
│ ✓ CORS       │      │ ✓ Metrics    │     │ ✓ Metrics    │
└──────────────┘      └──────────────┘     └──────────────┘
        │                     │                     │
        └─────────────────────┼─────────────────────┘
                              │
                              ▼
                  ┌──────────────────────┐
                  │  Shared redb Cache    │
                  │  /var/lib/totalimage  │
                  └──────────────────────┘
```

### Data Flow

1. **Client Request** → Reverse Proxy (TLS termination)
2. **Reverse Proxy** → Application Server (HTTP)
3. **Application** → redb Cache (metadata lookup)
4. **Application** → Disk Image File (read-only)
5. **Application** → Response + Metrics Update

---

## Deployment Options

### Option 1: Docker Compose (Easiest)

**Pros:**
- One-command deployment
- Automatic networking
- Health checks included
- Volume management

**Cons:**
- Requires Docker Engine
- Less fine-grained control

**Setup:**

```yaml
# docker-compose.prod.yml
version: '3.8'

services:
  web:
    image: totalimage:latest
    command: ["totalimage-web"]
    environment:
      - RUST_LOG=info
      - TOTALIMAGE_CACHE_DIR=/data/cache
    volumes:
      - ./images:/data/images:ro
      - cache-data:/data/cache
    restart: always
    deploy:
      resources:
        limits:
          memory: 2G
          cpus: '2.0'

  nginx:
    image: nginx:alpine
    ports:
      - "443:443"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf:ro
      - ./certs:/etc/nginx/certs:ro
    depends_on:
      - web
    restart: always

volumes:
  cache-data:
```

### Option 2: Kubernetes (Most Scalable)

**Deployment:**

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: totalimage-web
spec:
  replicas: 3
  selector:
    matchLabels:
      app: totalimage-web
  template:
    metadata:
      labels:
        app: totalimage-web
    spec:
      containers:
      - name: totalimage-web
        image: totalimage:latest
        ports:
        - containerPort: 3000
        env:
        - name: RUST_LOG
          value: "info"
        - name: TOTALIMAGE_CACHE_DIR
          value: "/data/cache"
        volumeMounts:
        - name: cache
          mountPath: /data/cache
        - name: images
          mountPath: /data/images
          readOnly: true
        resources:
          requests:
            memory: "512Mi"
            cpu: "500m"
          limits:
            memory: "2Gi"
            cpu: "2000m"
        livenessProbe:
          httpGet:
            path: /health
            port: 3000
          initialDelaySeconds: 10
          periodSeconds: 30
        readinessProbe:
          httpGet:
            path: /health
            port: 3000
          initialDelaySeconds: 5
          periodSeconds: 10
      volumes:
      - name: cache
        persistentVolumeClaim:
          claimName: totalimage-cache
      - name: images
        persistentVolumeClaim:
          claimName: forensic-images
```

### Option 3: Systemd Service (Traditional)

**Service File** (`/etc/systemd/system/totalimage-web.service`):

```ini
[Unit]
Description=TotalImage Web API Server
After=network.target

[Service]
Type=simple
User=totalimage
Group=totalimage
WorkingDirectory=/var/lib/totalimage
Environment="RUST_LOG=info"
Environment="TOTALIMAGE_CACHE_DIR=/var/lib/totalimage/cache"
ExecStart=/usr/local/bin/totalimage-web
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/totalimage/cache
ReadOnlyPaths=/var/lib/totalimage/images

[Install]
WantedBy=multi-user.target
```

**Enable and Start:**

```bash
sudo systemctl daemon-reload
sudo systemctl enable totalimage-web
sudo systemctl start totalimage-web
sudo systemctl status totalimage-web
```

---

## Configuration

### Environment Variables

| Variable | Service | Default | Description |
|----------|---------|---------|-------------|
| `RUST_LOG` | All | `info` | Logging level (trace, debug, info, warn, error) |
| `TOTALIMAGE_CACHE_DIR` | All | `~/.cache/totalimage` | Cache directory path |
| `TOTALIMAGE_MCP_PORT` | MCP | `3002` | MCP server port |
| `MCP_AUTH_ENABLED` | MCP | `false` | Enable JWT/API key authentication |
| `MCP_JWT_SECRET` | MCP | - | JWT signing secret (required if auth enabled) |
| `MCP_API_KEYS` | MCP | - | Comma-separated API keys |
| `MCP_WEBSOCKET_ENABLED` | MCP | `true` | Enable WebSocket for progress updates |
| `FIRE_MARSHAL_URL` | MCP | - | Fire Marshal URL for registration |
| `FIRE_MARSHAL_RATE_LIMIT` | Fire Marshal | `100` | Requests per second |
| `FIRE_MARSHAL_TIMEOUT` | Fire Marshal | `30000` | Request timeout (ms) |
| `REDIS_URL` | PYRO Worker | `redis://localhost:6379` | Redis connection URL |

### Rate Limiting

**Web API** (totalimage-web):
- **Global:** 100 requests/second
- **Per-endpoint:** No additional limits
- **Request timeout:** 30 seconds
- **Max body size:** 10 MB

**MCP Server** (totalimage-mcp):
- **Rate limiting:** Handled by reverse proxy
- **Authentication:** Optional JWT/API keys
- **WebSocket:** Concurrent connections limited by system

**Fire Marshal**:
- **Global:** 100 requests/second (configurable)
- **Per-tool:** Rate limits inherited from tool executors
- **Request timeout:** 30 seconds (configurable)

### Cache Configuration

**redb Cache Settings:**

```rust
const MAX_CACHE_SIZE_BYTES: u64 = 1024 * 1024 * 1024; // 1 GB
const CACHE_TTL_DAYS: u64 = 30; // 30 days
const EVICTION_THRESHOLD: f64 = 0.9; // Evict at 90% full
```

**Cache Maintenance:**

```bash
# Clear expired entries
totalimage cache clean

# View cache statistics
totalimage cache stats

# Clear all cache
totalimage cache purge
```

---

## Security Hardening

### 1. TLS/HTTPS Configuration

**See:** [`TLS-DEPLOYMENT.md`](./TLS-DEPLOYMENT.md) for comprehensive TLS setup.

**Quick nginx Example:**

```nginx
server {
    listen 443 ssl http2;
    server_name totalimage.example.com;

    ssl_certificate /etc/nginx/certs/fullchain.pem;
    ssl_certificate_key /etc/nginx/certs/privkey.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;

    location / {
        proxy_pass http://localhost:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

### 2. JWT Authentication (MCP Server)

**Enable JWT Auth:**

```bash
# Generate secret
openssl rand -hex 32 > /etc/totalimage/jwt-secret

# Set environment variables
export MCP_AUTH_ENABLED=true
export MCP_JWT_SECRET=$(cat /etc/totalimage/jwt-secret)
export MCP_JWT_ISSUER="totalimage-prod"
export MCP_JWT_AUDIENCE="totalimage-clients"
```

**Create JWT Token:**

```bash
# Using totalimage-mcp CLI
totalimage-mcp auth create-token \
  --user "analyst@example.com" \
  --roles "analyst,read" \
  --project "case-2024-001" \
  --expires-in-days 30
```

### 3. API Key Authentication

```bash
# Generate API keys
export MCP_AUTH_ENABLED=true
export MCP_API_KEYS="key1_abc123,key2_def456,key3_ghi789"

# Use in requests
curl -H "Authorization: Bearer key1_abc123" \
  http://localhost:3002/mcp
```

### 4. Network Security

**Firewall Rules (UFW):**

```bash
# Allow only necessary ports
sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw allow 22/tcp   # SSH
sudo ufw allow 443/tcp  # HTTPS
sudo ufw enable
```

**Docker Network Isolation:**

```yaml
networks:
  public:
    driver: bridge
  internal:
    driver: bridge
    internal: true  # No external access
```

### 5. File System Permissions

```bash
# Restrict image directory to read-only
chmod 0750 /var/lib/totalimage/images
chown totalimage:totalimage /var/lib/totalimage/images

# Cache directory read-write
chmod 0770 /var/lib/totalimage/cache
chown totalimage:totalimage /var/lib/totalimage/cache
```

---

## Monitoring

### Prometheus Metrics

**Endpoints:**
- MCP Server: `http://localhost:3002/metrics`
- Fire Marshal: `http://localhost:3001/metrics`
- PYRO Worker: Metrics via `getMetrics()` API

**Key Metrics:**

```promql
# Tool execution latency
histogram_quantile(0.95,
  rate(totalimage_mcp_tool_duration_seconds_bucket[5m]))

# Tool call success rate
rate(totalimage_mcp_tool_calls_total{status="success"}[5m])
/ rate(totalimage_mcp_tool_calls_total[5m])

# Cache hit rate
rate(totalimage_mcp_cache_operations_total{operation="hit"}[5m])
/ rate(totalimage_mcp_cache_operations_total[5m])

# Active requests
totalimage_mcp_active_requests

# Worker queue depth
totalimage_worker_queue_size{state="waiting"}
```

### Prometheus Configuration

```yaml
# prometheus.yml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'totalimage-mcp'
    static_configs:
      - targets: ['localhost:3002']

  - job_name: 'fire-marshal'
    static_configs:
      - targets: ['localhost:3001']
```

### Grafana Dashboards

**Import Dashboard ID:** (TODO: Create and publish)

**Key Panels:**
- Request rate (req/s)
- Error rate (%)
- P95 latency (ms)
- Cache hit rate (%)
- Active connections
- Disk I/O (MB/s)

### Alerting Rules

```yaml
# alerts.yml
groups:
  - name: totalimage
    rules:
      - alert: HighErrorRate
        expr: rate(totalimage_mcp_tool_calls_total{status="error"}[5m]) > 0.1
        for: 5m
        annotations:
          summary: "High error rate detected"

      - alert: HighLatency
        expr: histogram_quantile(0.95, totalimage_mcp_tool_duration_seconds_bucket) > 10
        for: 5m
        annotations:
          summary: "P95 latency exceeds 10 seconds"

      - alert: LowCacheHitRate
        expr: rate(totalimage_mcp_cache_operations_total{operation="hit"}[5m]) / rate(totalimage_mcp_cache_operations_total[5m]) < 0.5
        for: 10m
        annotations:
          summary: "Cache hit rate below 50%"
```

---

## Scaling

### Horizontal Scaling

**Load Balancer Configuration:**

```nginx
upstream totalimage_backend {
    least_conn;
    server 10.0.1.10:3000;
    server 10.0.1.11:3000;
    server 10.0.1.12:3000;
}

server {
    listen 443 ssl http2;
    location / {
        proxy_pass http://totalimage_backend;
    }
}
```

### Vertical Scaling

**Resource Recommendations:**

| Component | CPU | Memory | Disk I/O | Notes |
|-----------|-----|--------|----------|-------|
| Web API | 2 cores | 2 GB | Medium | Primarily I/O bound |
| MCP Server | 2 cores | 2 GB | Medium | Caching reduces load |
| Fire Marshal | 1 core | 512 MB | Low | Orchestration only |
| PYRO Worker | 4 cores | 4 GB | High | CPU + I/O intensive |

### Cache Optimization

**Shared Cache (Multi-instance):**

```bash
# Use NFS or distributed filesystem
mount -t nfs server:/totalimage-cache /var/lib/totalimage/cache

# Or use Redis for metadata cache
# (requires code modification)
```

---

## Troubleshooting

### High Memory Usage

**Check cache size:**

```bash
# View cache statistics
totalimage cache stats

# If cache is too large, reduce TTL
export CACHE_TTL_DAYS=7
```

**Monitor with:**

```bash
# Container memory
docker stats totalimage-web

# Process memory
ps aux | grep totalimage
```

### Slow Response Times

**Check disk I/O:**

```bash
# Monitor I/O
iostat -x 1

# Check if images are on slow storage
hdparm -t /dev/sda
```

**Enable query logging:**

```bash
export RUST_LOG=totalimage=debug
```

### Connection Timeouts

**Increase timeout:**

```bash
# Web API - edit source or use reverse proxy
# nginx.conf
proxy_read_timeout 60s;
proxy_connect_timeout 60s;
```

### Authentication Failures

**Verify JWT configuration:**

```bash
# Check secret is set
echo $MCP_JWT_SECRET

# Verify token
totalimage-mcp auth verify-token <token>
```

---

## Appendix

### A. Production Checklist

- [ ] TLS/HTTPS configured
- [ ] Rate limiting enabled
- [ ] Authentication configured
- [ ] Monitoring dashboards set up
- [ ] Alerting rules configured
- [ ] Backup strategy defined
- [ ] Disaster recovery plan documented
- [ ] Security audit completed
- [ ] Load testing performed
- [ ] Documentation reviewed

### B. Performance Benchmarks

**Expected Performance (single instance):**

- **Throughput:** 100-500 req/s (depending on image size)
- **Latency P50:** < 100ms (cached), < 500ms (uncached)
- **Latency P95:** < 500ms (cached), < 2s (uncached)
- **Latency P99:** < 1s (cached), < 5s (uncached)

### C. Support

- **Issues:** https://github.com/Ununp3ntium115/TotalImage/issues
- **Documentation:** https://github.com/Ununp3ntium115/TotalImage/wiki
- **Security:** security@totalimage.org

---

**Document Version:** 1.0.0
**Last Updated:** 2025-12-02
**Status:** ✅ Production Ready
