# TotalImage Kubernetes Deployment Guide

Complete guide for deploying TotalImage to Kubernetes in production.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Quick Start](#quick-start)
- [Architecture](#architecture)
- [Storage Configuration](#storage-configuration)
- [Security](#security)
- [High Availability](#high-availability)
- [Monitoring](#monitoring)
- [Troubleshooting](#troubleshooting)

---

## Prerequisites

### Required

- Kubernetes cluster (1.23+)
- kubectl configured
- Container registry access
- Persistent storage (NFS recommended for shared disk images)

### Optional but Recommended

- Ingress controller (nginx, traefik, etc.)
- Prometheus Operator (for monitoring)
- cert-manager (for TLS certificates)
- Horizontal Pod Autoscaler enabled (metrics-server)

---

## Quick Start

### 1. Build and Push Container Image

```bash
# Build multi-arch image
docker buildx build --platform linux/amd64,linux/arm64 \
  -t your-registry.com/totalimage:latest \
  --push .

# Or build for single architecture
docker build -t your-registry.com/totalimage:latest .
docker push your-registry.com/totalimage:latest
```

### 2. Update Image References

Edit `k8s/deployment.yaml` and replace `totalimage:latest` with your image:

```yaml
image: your-registry.com/totalimage:latest
```

### 3. Configure Secrets

```bash
# Generate a secure JWT secret (minimum 32 characters)
JWT_SECRET=$(openssl rand -base64 32)

# Create secret
kubectl create secret generic totalimage-secrets \
  --from-literal=MCP_JWT_SECRET="$JWT_SECRET"

# Optional: Add API keys
kubectl create secret generic totalimage-secrets \
  --from-literal=MCP_JWT_SECRET="$JWT_SECRET" \
  --from-literal=MCP_API_KEYS="key1,key2,key3" \
  --dry-run=client -o yaml | kubectl apply -f -
```

### 4. Configure Storage

**Option A: NFS (Recommended for shared disk images)**

Create a PersistentVolume backed by NFS:

```yaml
apiVersion: v1
kind: PersistentVolume
metadata:
  name: totalimage-images-pv
spec:
  capacity:
    storage: 1Ti
  accessModes:
    - ReadOnlyMany
  nfs:
    server: nfs.example.com
    path: /exports/forensic-images
  mountOptions:
    - ro
    - nfsvers=4.1
```

**Option B: Cloud Storage (AWS EFS, Azure Files, GCP Filestore)**

```yaml
apiVersion: v1
kind: PersistentVolume
metadata:
  name: totalimage-images-pv
spec:
  capacity:
    storage: 1Ti
  accessModes:
    - ReadOnlyMany
  csi:
    driver: efs.csi.aws.com
    volumeHandle: fs-12345678
```

Apply the PV:

```bash
kubectl apply -f pv-images.yaml
```

### 5. Deploy TotalImage

```bash
# Deploy all components
kubectl apply -f k8s/

# Watch deployment progress
kubectl rollout status deployment/totalimage-web
kubectl rollout status deployment/totalimage-mcp
kubectl rollout status deployment/fire-marshal
```

### 6. Verify Deployment

```bash
# Check pods
kubectl get pods -l app=totalimage

# Check services
kubectl get svc -l app=totalimage

# Test health endpoints
kubectl port-forward svc/totalimage-web 8080:80
curl http://localhost:8080/health
```

### 7. Configure Ingress (Optional)

Edit `k8s/ingress.yaml` and update hostnames:

```yaml
- host: totalimage.your-domain.com
```

Enable TLS:

```yaml
tls:
  - hosts:
      - totalimage.your-domain.com
      - mcp.totalimage.your-domain.com
    secretName: totalimage-tls
```

Apply:

```bash
kubectl apply -f k8s/ingress.yaml
```

---

## Architecture

### Components

```
┌─────────────────────────────────────────────────────────┐
│                     Kubernetes Cluster                   │
├─────────────────────────────────────────────────────────┤
│                                                           │
│  ┌─────────────┐  ┌─────────────┐  ┌──────────────┐    │
│  │ Ingress     │  │ Ingress     │  │              │    │
│  │ Controller  │  │ Controller  │  │  Prometheus  │    │
│  │ (Web API)   │  │ (MCP API)   │  │  Operator    │    │
│  └──────┬──────┘  └──────┬──────┘  └──────┬───────┘    │
│         │                 │                 │            │
│  ┌──────▼──────┐   ┌─────▼────┐           │            │
│  │ totalimage  │   │totalimage│           │            │
│  │    -web     │   │   -mcp   │           │            │
│  │ (2-10 pods) │   │(2-8 pods)│           │            │
│  └──────┬──────┘   └─────┬────┘           │            │
│         │                 │                 │            │
│         └────────┬────────┘                 │            │
│                  │                          │            │
│           ┌──────▼───────┐          ┌──────▼───────┐   │
│           │fire-marshal  │          │ServiceMonitor│   │
│           │  (1 pod)     │          │   (metrics)  │   │
│           └──────┬───────┘          └──────────────┘   │
│                  │                                       │
│  ┌───────────────┴────────────────┐                     │
│  │  PersistentVolumeClaim         │                     │
│  │  - images (ReadOnlyMany)       │                     │
│  │  - fire-marshal-data (RWO)     │                     │
│  └────────────────────────────────┘                     │
│                                                           │
└─────────────────────────────────────────────────────────┘
```

### Deployments

| Component | Replicas | Resources | Purpose |
|-----------|----------|-----------|---------|
| `totalimage-web` | 2-10 (HPA) | 256Mi-1Gi, 100m-1000m | REST API server |
| `totalimage-mcp` | 2-8 (HPA) | 512Mi-2Gi, 200m-2000m | MCP protocol server for AI/Claude |
| `fire-marshal` | 1 | 256Mi-512Mi, 100m-500m | MCP tool registry |

### Services

| Service | Type | Port | Target |
|---------|------|------|--------|
| `totalimage-web` | ClusterIP | 80 | 3000 |
| `totalimage-mcp` | ClusterIP | 3002 | 3002 |
| `fire-marshal` | ClusterIP | 3001 | 3001 |

---

## Storage Configuration

### Disk Image Storage (ReadOnlyMany)

**Requirements:**
- Must support ReadOnlyMany access mode
- Should be mounted read-only for security
- Recommended: NFS, EFS, Azure Files, GCP Filestore

**Configuration:**

```yaml
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: totalimage-images
spec:
  accessModes:
    - ReadOnlyMany  # Multiple pods can read simultaneously
  resources:
    requests:
      storage: 100Gi  # Adjust based on image collection size
  storageClassName: nfs-client  # Your storage class
```

**Best Practices:**
- Mount as read-only to prevent accidental modifications
- Use dedicated storage for forensic images (separate from application data)
- Consider storage performance (IOPS, throughput) for large images
- Enable snapshots for backup/recovery

### Fire Marshal Storage (ReadWriteOnce)

**Requirements:**
- Standard block storage
- Only needs single-node access

```yaml
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: fire-marshal-data
spec:
  accessModes:
    - ReadWriteOnce
  resources:
    requests:
      storage: 10Gi
  storageClassName: standard
```

### Cache Storage (EmptyDir)

Each pod uses ephemeral cache storage:

```yaml
volumes:
  - name: cache
    emptyDir:
      sizeLimit: 1Gi  # Adjust based on workload
```

**Notes:**
- Cache is lost when pod restarts
- Size limit prevents disk exhaustion
- For persistent caching, use PVC instead

---

## Security

### Pod Security

All deployments use restrictive security contexts:

```yaml
securityContext:
  runAsNonRoot: true           # Prevents running as root
  runAsUser: 1000              # Specific non-root UID
  readOnlyRootFilesystem: true # Immutable filesystem
```

**Writable directories:**
- `/var/cache/totalimage` - Cache storage (emptyDir)
- `/tmp` - Temporary files (emptyDir implied)

### Network Policies

Network policies restrict pod-to-pod communication:

```bash
# Apply network policies
kubectl apply -f k8s/networkpolicy.yaml
```

**Traffic rules:**
- `totalimage-web`: Ingress from ingress controller, egress to fire-marshal
- `totalimage-mcp`: Ingress from ingress controller, egress to fire-marshal
- `fire-marshal`: Ingress only from totalimage pods, no external access

### Secrets Management

**Required secrets:**

```bash
kubectl create secret generic totalimage-secrets \
  --from-literal=MCP_JWT_SECRET="$(openssl rand -base64 32)"
```

**Optional secrets:**

```bash
# API keys for authentication
kubectl create secret generic totalimage-secrets \
  --from-literal=MCP_JWT_SECRET="..." \
  --from-literal=MCP_API_KEYS="key1,key2,key3" \
  --from-literal=TOTALIMAGE_WEB_API_KEYS="webkey1,webkey2"
```

**External secrets (recommended):**

Use external secrets operator to sync from AWS Secrets Manager, Azure Key Vault, etc:

```yaml
apiVersion: external-secrets.io/v1beta1
kind: ExternalSecret
metadata:
  name: totalimage-secrets
spec:
  secretStoreRef:
    name: aws-secrets-manager
  target:
    name: totalimage-secrets
  data:
    - secretKey: MCP_JWT_SECRET
      remoteRef:
        key: totalimage/jwt-secret
```

### TLS Configuration

**Option A: cert-manager (Recommended)**

```yaml
apiVersion: cert-manager.io/v1
kind: Certificate
metadata:
  name: totalimage-tls
spec:
  secretName: totalimage-tls
  issuer: letsencrypt-prod
  dnsNames:
    - totalimage.your-domain.com
    - mcp.totalimage.your-domain.com
```

**Option B: Manual certificate**

```bash
# Create TLS secret
kubectl create secret tls totalimage-tls \
  --cert=path/to/tls.crt \
  --key=path/to/tls.key
```

---

## High Availability

### Horizontal Pod Autoscaling

Auto-scale based on CPU/memory:

```bash
# Apply HPA
kubectl apply -f k8s/hpa.yaml

# Monitor autoscaling
kubectl get hpa -w
```

**Configuration:**
- `totalimage-web`: 2-10 replicas, scale at 70% CPU
- `totalimage-mcp`: 2-8 replicas, scale at 75% CPU

**Custom metrics (advanced):**

Scale based on request rate:

```yaml
metrics:
  - type: Pods
    pods:
      metric:
        name: http_requests_per_second
      target:
        type: AverageValue
        averageValue: "1000"
```

### Pod Disruption Budgets

Ensure availability during node maintenance:

```bash
kubectl apply -f k8s/pdb.yaml
```

**Rules:**
- At least 1 pod of `totalimage-web` must be available
- At least 1 pod of `totalimage-mcp` must be available

**Test PDB:**

```bash
# Drain node (respects PDB)
kubectl drain node-1 --ignore-daemonsets

# Watch pods migrate
kubectl get pods -w
```

### Multi-Zone Deployment

Spread pods across availability zones:

```yaml
affinity:
  podAntiAffinity:
    preferredDuringSchedulingIgnoredDuringExecution:
      - weight: 100
        podAffinityTerm:
          labelSelector:
            matchLabels:
              app: totalimage
              component: web
          topologyKey: topology.kubernetes.io/zone
```

---

## Monitoring

### Prometheus Integration

**Prerequisites:**
- Prometheus Operator installed
- ServiceMonitor CRD available

**Deploy ServiceMonitors:**

```bash
kubectl apply -f k8s/servicemonitor.yaml
```

**Metrics exposed:**
- `/metrics` endpoint on each service
- Standard Rust metrics (requests, latency, errors)
- Custom TotalImage metrics (vault operations, extractions, etc.)

**Example queries:**

```promql
# Request rate
rate(http_requests_total[5m])

# Error rate
rate(http_requests_total{status=~"5.."}[5m])

# P95 latency
histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))

# Pod memory usage
container_memory_usage_bytes{pod=~"totalimage-.*"}

# Vault operations
rate(totalimage_vault_opens_total[5m])
```

### Grafana Dashboards

Import dashboard from `monitoring/grafana-dashboard.json`:

1. Open Grafana
2. Navigate to Dashboards → Import
3. Upload JSON file
4. Select Prometheus datasource

**Dashboard panels:**
- Request rate and latency
- Error rates by endpoint
- Resource utilization (CPU, memory)
- Vault operation metrics
- Pod status and replica counts

### Logging

**Structured JSON logging:**

All components log in JSON format when `RUST_LOG_FORMAT=json`:

```json
{
  "timestamp": "2025-12-05T10:30:45Z",
  "level": "INFO",
  "target": "totalimage_web",
  "message": "Vault opened successfully",
  "fields": {
    "path": "/data/evidence.vhd",
    "duration_ms": 150
  }
}
```

**Log aggregation options:**

**Option A: Loki (recommended)**

```bash
# Install Loki
helm install loki grafana/loki-stack \
  --set promtail.enabled=true

# Query logs in Grafana
{app="totalimage"} |= "error"
```

**Option B: ELK Stack**

Deploy Filebeat daemonset to ship logs to Elasticsearch.

**Option C: Cloud-native (EKS, AKS, GKE)**

Use CloudWatch, Azure Monitor, or Cloud Logging respectively.

### Alerting

**Prometheus AlertManager rules:**

```yaml
groups:
  - name: totalimage
    rules:
      - alert: TotalImageHighErrorRate
        expr: rate(http_requests_total{status=~"5..",app="totalimage"}[5m]) > 0.05
        for: 5m
        annotations:
          summary: "High error rate in TotalImage"

      - alert: TotalImagePodDown
        expr: up{app="totalimage"} == 0
        for: 2m
        annotations:
          summary: "TotalImage pod is down"
```

---

## Troubleshooting

### Common Issues

#### 1. Pods stuck in Pending

**Symptom:** Pods remain in Pending state

**Diagnosis:**

```bash
kubectl describe pod <pod-name>
```

**Common causes:**
- Insufficient resources (CPU/memory)
- PVC not bound (storage issue)
- Image pull failure

**Solutions:**

```bash
# Check node resources
kubectl top nodes

# Check PVC status
kubectl get pvc

# Check image pull
kubectl get events --field-selector involvedObject.name=<pod-name>
```

#### 2. Health check failures

**Symptom:** Pods crash-looping or marked unhealthy

**Diagnosis:**

```bash
# Check pod logs
kubectl logs <pod-name>

# Check health endpoint manually
kubectl port-forward <pod-name> 3000:3000
curl http://localhost:3000/health
```

**Common causes:**
- Startup time exceeds initialDelaySeconds
- Missing environment variables
- Storage not mounted

**Solutions:**

```bash
# Increase startup probe failure threshold
# Edit deployment.yaml:
startupProbe:
  failureThreshold: 60  # Allow up to 5 minutes (60 * 5s)
```

#### 3. "Permission denied" errors

**Symptom:** Cannot write to cache or temp directories

**Diagnosis:**

```bash
kubectl logs <pod-name> | grep "Permission denied"
```

**Cause:** Read-only root filesystem without writable volumes

**Solution:** Ensure emptyDir volumes are mounted:

```yaml
volumeMounts:
  - name: cache
    mountPath: /var/cache/totalimage
volumes:
  - name: cache
    emptyDir: {}
```

#### 4. Storage mount failures

**Symptom:** "Volume mount failed" in events

**Diagnosis:**

```bash
kubectl get pv
kubectl get pvc
kubectl describe pvc totalimage-images
```

**Common causes:**
- PV not created
- Access mode mismatch (need ReadOnlyMany for images)
- NFS server unreachable

**Solutions:**

```bash
# Test NFS connectivity from node
showmount -e nfs-server.example.com

# Create PV manually
kubectl apply -f pv-images.yaml
```

#### 5. High memory usage

**Symptom:** Pods OOMKilled or high memory usage

**Diagnosis:**

```bash
kubectl top pods
kubectl describe pod <pod-name> | grep -A 5 State
```

**Solutions:**

```bash
# Increase memory limits
# Edit deployment.yaml:
resources:
  limits:
    memory: "2Gi"  # Increase from 1Gi

# Check for memory leaks in logs
kubectl logs <pod-name> | grep -i memory
```

### Debug Commands

```bash
# Get all TotalImage resources
kubectl get all -l app=totalimage

# Describe deployment
kubectl describe deployment totalimage-web

# View recent events
kubectl get events --sort-by='.lastTimestamp' | grep totalimage

# Execute into pod
kubectl exec -it <pod-name> -- /bin/sh

# Check disk usage in pod
kubectl exec <pod-name> -- df -h

# Test network connectivity between pods
kubectl exec <pod-name> -- wget -O- http://fire-marshal:3001/health

# View config applied to pod
kubectl get pod <pod-name> -o yaml

# Restart deployment
kubectl rollout restart deployment/totalimage-web
```

### Performance Tuning

**Increase replica count:**

```bash
kubectl scale deployment totalimage-web --replicas=5
```

**Adjust resource limits:**

```yaml
resources:
  requests:
    memory: "512Mi"  # Guaranteed allocation
    cpu: "250m"
  limits:
    memory: "2Gi"    # Max allocation
    cpu: "2000m"
```

**Enable HTTP/2 in ingress:**

```yaml
metadata:
  annotations:
    nginx.ingress.kubernetes.io/backend-protocol: "HTTP2"
```

**Tune cache size:**

```yaml
env:
  - name: TOTALIMAGE_MAX_CACHE_ENTRIES
    value: "5000"  # Increase from default 1000
```

---

## Production Checklist

- [ ] Container image built and pushed to registry
- [ ] Secrets created (JWT secret, API keys)
- [ ] Storage configured (NFS/EFS for images)
- [ ] Ingress controller installed
- [ ] TLS certificates configured
- [ ] Network policies applied
- [ ] Resource requests/limits tuned
- [ ] HPA configured and tested
- [ ] PDB configured
- [ ] Monitoring (Prometheus + Grafana) deployed
- [ ] Logging aggregation configured
- [ ] AlertManager rules configured
- [ ] Backup strategy for fire-marshal data
- [ ] Disaster recovery plan documented
- [ ] Load testing performed
- [ ] Security scan completed (Trivy, Snyk, etc.)

---

## Additional Resources

- [TotalImage Architecture](../docs/ARCHITECTURE.md)
- [Streaming Guide](../docs/STREAMING.md)
- [API Documentation](../docs/API.md)
- [Production Deployment Guide](../steering/PRODUCTION-DEPLOYMENT.md)
- [Kubernetes Documentation](https://kubernetes.io/docs/)
- [Prometheus Operator](https://prometheus-operator.dev/)
