# TLS/HTTPS Deployment Guide

**Status:** Production Recommendation (SEC-007)  
**Last Updated:** 2025-12-01

## Overview

TotalImage Web API should be deployed behind a TLS-terminating reverse proxy in production environments. This is industry best practice and provides better security, performance, and operational flexibility than embedding TLS in the application.

## Recommended Architecture

```
Internet
    ↓
[Reverse Proxy: nginx/Traefik/Caddy]
  - TLS termination (HTTPS → HTTP)
  - Rate limiting
  - Request logging
  - Load balancing
    ↓
[TotalImage Web API]
  - HTTP on localhost:3000
  - No TLS complexity
  - Focus on business logic
```

## Benefits of Reverse Proxy Approach

### Security
- **Centralized TLS Management**: One place to manage certificates, cipher suites, and protocols
- **Automatic Certificate Renewal**: Let's Encrypt integration (Caddy/Traefik)
- **DDoS Protection**: Rate limiting and connection pooling at proxy level
- **Request Filtering**: Block malicious requests before they reach the application

### Performance
- **Connection Pooling**: Reuse backend connections
- **Compression**: gzip/brotli at proxy level
- **Static File Serving**: Offload static assets
- **Caching**: HTTP cache headers and response caching

### Operations
- **Zero-Downtime Deploys**: Proxy handles connection draining
- **Health Checks**: Automatic backend monitoring
- **Logging**: Centralized access logs
- **Metrics**: Built-in Prometheus exporters

---

## Implementation: nginx

### 1. Install nginx

```bash
# Ubuntu/Debian
sudo apt install nginx certbot python3-certbot-nginx

# RHEL/CentOS
sudo yum install nginx certbot python3-certbot-nginx
```

### 2. Configure nginx

Create `/etc/nginx/sites-available/totalimage`:

```nginx
# TotalImage Web API - TLS Termination
upstream totalimage_backend {
    server 127.0.0.1:3000 max_fails=3 fail_timeout=30s;
}

# HTTP → HTTPS redirect
server {
    listen 80;
    listen [::]:80;
    server_name totalimage.example.com;
    
    location /.well-known/acme-challenge/ {
        root /var/www/html;
    }
    
    location / {
        return 301 https://$server_name$request_uri;
    }
}

# HTTPS server
server {
    listen 443 ssl http2;
    listen [::]:443 ssl http2;
    server_name totalimage.example.com;

    # TLS configuration
    ssl_certificate /etc/letsencrypt/live/totalimage.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/totalimage.example.com/privkey.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers 'ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256:ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-RSA-AES256-GCM-SHA384';
    ssl_prefer_server_ciphers off;
    
    # Security headers
    add_header Strict-Transport-Security "max-age=63072000" always;
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-XSS-Protection "1; mode=block" always;

    # Rate limiting (10 req/sec per IP)
    limit_req_zone $binary_remote_addr zone=api_limit:10m rate=10r/s;
    limit_req zone=api_limit burst=20 nodelay;

    # Proxy configuration
    location / {
        proxy_pass http://totalimage_backend;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        
        # Timeouts
        proxy_connect_timeout 30s;
        proxy_send_timeout 30s;
        proxy_read_timeout 30s;
        
        # Buffer sizes
        proxy_buffering on;
        proxy_buffer_size 4k;
        proxy_buffers 8 4k;
        proxy_busy_buffers_size 8k;
    }

    # Health check endpoint (no rate limiting)
    location /health {
        proxy_pass http://totalimage_backend;
        access_log off;
    }

    # Access log
    access_log /var/log/nginx/totalimage_access.log;
    error_log /var/log/nginx/totalimage_error.log;
}
```

### 3. Enable site and get certificate

```bash
# Enable configuration
sudo ln -s /etc/nginx/sites-available/totalimage /etc/nginx/sites-enabled/
sudo nginx -t
sudo systemctl reload nginx

# Get Let's Encrypt certificate
sudo certbot --nginx -d totalimage.example.com

# Test auto-renewal
sudo certbot renew --dry-run
```

---

## Implementation: Traefik

### 1. Docker Compose Setup

Create `docker-compose.yml`:

```yaml
version: '3.8'

services:
  traefik:
    image: traefik:v2.10
    command:
      - "--api.dashboard=true"
      - "--providers.docker=true"
      - "--providers.docker.exposedbydefault=false"
      - "--entrypoints.web.address=:80"
      - "--entrypoints.websecure.address=:443"
      - "--certificatesresolvers.letsencrypt.acme.httpchallenge=true"
      - "--certificatesresolvers.letsencrypt.acme.httpchallenge.entrypoint=web"
      - "--certificatesresolvers.letsencrypt.acme.email=admin@example.com"
      - "--certificatesresolvers.letsencrypt.acme.storage=/letsencrypt/acme.json"
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock:ro
      - ./letsencrypt:/letsencrypt
    networks:
      - traefik_network

  totalimage-web:
    image: totalimage-web:latest
    labels:
      - "traefik.enable=true"
      - "traefik.http.routers.totalimage.rule=Host(`totalimage.example.com`)"
      - "traefik.http.routers.totalimage.entrypoints=websecure"
      - "traefik.http.routers.totalimage.tls.certresolver=letsencrypt"
      - "traefik.http.services.totalimage.loadbalancer.server.port=3000"
      # HTTP → HTTPS redirect
      - "traefik.http.middlewares.redirect-to-https.redirectscheme.scheme=https"
      - "traefik.http.routers.totalimage-http.rule=Host(`totalimage.example.com`)"
      - "traefik.http.routers.totalimage-http.entrypoints=web"
      - "traefik.http.routers.totalimage-http.middlewares=redirect-to-https"
      # Rate limiting
      - "traefik.http.middlewares.ratelimit.ratelimit.average=10"
      - "traefik.http.middlewares.ratelimit.ratelimit.burst=20"
      - "traefik.http.routers.totalimage.middlewares=ratelimit"
    networks:
      - traefik_network

networks:
  traefik_network:
    driver: bridge
```

### 2. Deploy

```bash
docker-compose up -d
```

---

## Implementation: Caddy (Simplest)

### 1. Install Caddy

```bash
# Ubuntu/Debian
sudo apt install -y debian-keyring debian-archive-keyring apt-transport-https
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | sudo gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' | sudo tee /etc/apt/sources.list.d/caddy-stable.list
sudo apt update
sudo apt install caddy
```

### 2. Configure Caddy

Create `/etc/caddy/Caddyfile`:

```caddyfile
# TotalImage Web API
totalimage.example.com {
    # Automatic HTTPS (Let's Encrypt)
    # No manual certificate management required!
    
    # Rate limiting (10 req/sec)
    rate_limit {
        zone totalimage_api {
            key {remote_host}
            events 10
            window 1s
        }
    }

    # Reverse proxy to TotalImage
    reverse_proxy localhost:3000 {
        # Health check
        health_uri /health
        health_interval 10s
        health_timeout 5s
        
        # Load balancing (if multiple backends)
        lb_policy least_conn
        
        # Timeouts
        timeout 30s
    }

    # Security headers
    header {
        Strict-Transport-Security "max-age=63072000"
        X-Frame-Options "SAMEORIGIN"
        X-Content-Type-Options "nosniff"
        X-XSS-Protection "1; mode=block"
    }

    # Logging
    log {
        output file /var/log/caddy/totalimage_access.log
        format json
    }
}
```

### 3. Start Caddy

```bash
sudo systemctl enable caddy
sudo systemctl start caddy

# Caddy automatically gets Let's Encrypt certificates!
# No certbot needed
```

---

## Kubernetes Deployment

### Ingress with cert-manager

```yaml
apiVersion: cert-manager.io/v1
kind: Certificate
metadata:
  name: totalimage-tls
  namespace: default
spec:
  secretName: totalimage-tls-secret
  issuerRef:
    name: letsencrypt-prod
    kind: ClusterIssuer
  dnsNames:
    - totalimage.example.com
---
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: totalimage-ingress
  annotations:
    cert-manager.io/cluster-issuer: "letsencrypt-prod"
    nginx.ingress.kubernetes.io/rate-limit: "10"
    nginx.ingress.kubernetes.io/limit-rps: "10"
spec:
  ingressClassName: nginx
  tls:
    - hosts:
        - totalimage.example.com
      secretName: totalimage-tls-secret
  rules:
    - host: totalimage.example.com
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: totalimage-web
                port:
                  number: 3000
```

---

## Security Checklist

### TLS Configuration
- [ ] Use TLS 1.2+ only (disable 1.0 and 1.1)
- [ ] Use strong cipher suites (ECDHE preferred)
- [ ] Enable HSTS with long max-age (1+ year)
- [ ] Configure automatic certificate renewal
- [ ] Monitor certificate expiration

### Reverse Proxy
- [ ] Enable rate limiting (10-100 req/sec depending on load)
- [ ] Set request size limits (10 MB max)
- [ ] Configure timeouts (30s connect, 30s read)
- [ ] Add security headers (X-Frame-Options, CSP, etc.)
- [ ] Enable access logging
- [ ] Set up health checks

### Monitoring
- [ ] Monitor backend health status
- [ ] Alert on certificate expiration (< 7 days)
- [ ] Track rate limit violations
- [ ] Monitor response times
- [ ] Set up log aggregation

---

## Testing

### 1. Test HTTPS

```bash
curl -I https://totalimage.example.com/health
```

### 2. Test TLS Configuration

```bash
# Use SSL Labs
https://www.ssllabs.com/ssltest/analyze.html?d=totalimage.example.com

# Or testssl.sh
./testssl.sh https://totalimage.example.com
```

### 3. Test Rate Limiting

```bash
# Send 100 requests rapidly
for i in {1..100}; do
    curl -s -o /dev/null -w "%{http_code}\n" https://totalimage.example.com/health
done
# Should see 429 (Too Many Requests) after burst limit
```

---

## Troubleshooting

### Certificate Issues

```bash
# nginx: Check certificate validity
sudo openssl x509 -in /etc/letsencrypt/live/totalimage.example.com/cert.pem -text -noout

# Traefik: Check logs
docker-compose logs traefik | grep acme

# Caddy: Check logs
sudo journalctl -u caddy -f
```

### Connection Issues

```bash
# Test backend directly
curl http://localhost:3000/health

# Check proxy logs
sudo tail -f /var/log/nginx/totalimage_error.log

# Test DNS resolution
dig totalimage.example.com

# Test TLS handshake
openssl s_client -connect totalimage.example.com:443 -servername totalimage.example.com
```

---

## References

- [Mozilla SSL Configuration Generator](https://ssl-config.mozilla.org/)
- [OWASP TLS Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Transport_Layer_Protection_Cheat_Sheet.html)
- [nginx SSL/TLS Configuration](https://nginx.org/en/docs/http/configuring_https_servers.html)
- [Traefik Documentation](https://doc.traefik.io/traefik/)
- [Caddy Documentation](https://caddyserver.com/docs/)

---

**Status:** ✅ Complete  
**Security Impact:** HIGH - TLS deployment is required for production  
**Maintenance:** LOW - Automatic certificate renewal via Let's Encrypt
