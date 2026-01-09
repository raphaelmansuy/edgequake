# EdgeQuake Deployment Guide

> Production deployment for EdgeQuake API and WebUI

**Version**: 2.0.0 | **Last Updated**: January 2026

> **Implements**: [FEAT0025](features.md#feat0025) Production Deployment | [FEAT0026](features.md#feat0026) Monitoring
> **Business Rules**: [BR0020](business_rules.md#br0020) Health Checks | [BR0021](business_rules.md#br0021) High Availability
> **Code Reference**: See [edgequake/src/main.rs](../edgequake/src/main.rs) for server configuration and [edgequake/docker/](../edgequake/docker/) for Docker files

---

## Quick Reference

| I want to...                  | Go to                                         |
| ----------------------------- | --------------------------------------------- |
| Deploy with Docker            | [Docker Deployment](#docker-deployment)       |
| Set up Kubernetes             | [Kubernetes Deployment](#kubernetes-deployment) |
| Configure monitoring          | [Monitoring & Health](#monitoring--health)    |
| Check production readiness    | [Pre-deployment Checklist](#pre-deployment-checklist) |
| Troubleshoot issues           | [Troubleshooting](#troubleshooting)           |
| Set up backups                | [Backup & Recovery](#backup--recovery)        |

---

## Pre-deployment Checklist

Before going to production, verify all items:

### Infrastructure
- [ ] **PostgreSQL 15+** with extensions: `vector`, `age`, `pg_trgm`, `uuid-ossp`
- [ ] **SSL/TLS certificates** configured for HTTPS
- [ ] **Domain/DNS** configured for API and WebUI
- [ ] **Firewall rules** allow ports 80, 443, 5432 (internal only)
- [ ] **Load balancer** configured (if multi-instance)

### Configuration
- [ ] **DATABASE_URL** uses production credentials (not default `password`)
- [ ] **OPENAI_API_KEY** set with valid key (or alternative LLM configured)
- [ ] **CORS origins** restricted to production domains
- [ ] **Rate limiting** enabled at load balancer or API level
- [ ] **Log level** set to `info` (not `debug` in production)

### Security
- [ ] **Secrets** stored in vault/secret manager (not in code/env files)
- [ ] **Database** not exposed to public internet
- [ ] **API keys** rotated from development values
- [ ] **Audit logging** enabled for sensitive operations
- [ ] **Input validation** enabled (default)

### Monitoring
- [ ] **Health endpoints** responding at `/health`, `/live`, `/ready`
- [ ] **Prometheus metrics** scraped from `/metrics`
- [ ] **Alerting** configured for critical metrics
- [ ] **Log aggregation** set up (Loki, ELK, CloudWatch)
- [ ] **Error tracking** configured (Sentry, Honeybadger)

---

## Table of Contents

1. [Deployment Options](#deployment-options)
2. [Docker Deployment](#docker-deployment)
3. [Manual Deployment](#manual-deployment)
4. [Kubernetes Deployment](#kubernetes-deployment)
5. [Configuration](#configuration)
6. [Monitoring & Health](#monitoring--health)
7. [Security Considerations](#security-considerations)
8. [Troubleshooting](#troubleshooting)

---

## Deployment Options

| Option             | Best For                   | Complexity | Scalability |
| ------------------ | -------------------------- | ---------- | ----------- |
| **Docker Compose** | Small/Medium deployments   | Low        | Medium      |
| **Manual**         | Development, custom setups | Medium     | Low         |
| **Kubernetes**     | Large scale production     | High       | High        |

### Minimum Requirements

| Component     | CPU     | Memory | Storage |
| ------------- | ------- | ------ | ------- |
| EdgeQuake API | 2 cores | 2GB    | 1GB     |
| WebUI         | 1 core  | 512MB  | 100MB   |
| PostgreSQL    | 2 cores | 4GB    | 50GB+   |

---

## Docker Deployment

### Prerequisites

```bash
# Docker and Docker Compose
docker --version  # 20.10+
docker compose version  # 2.0+
```

### Quick Start

```bash
cd edgequake/docker

# Start all services
docker compose up -d

# View logs
docker compose logs -f edgequake-api
```

### Docker Compose Configuration

```yaml
# docker-compose.yml
version: "3.8"

services:
  edgequake-api:
    build:
      context: ..
      dockerfile: docker/Dockerfile
    ports:
      - "8080:8080"
    environment:
      - EDGEQUAKE_API_HOST=0.0.0.0
      - EDGEQUAKE_API_PORT=8080
      - DATABASE_URL=postgresql://edgequake:password@postgres:5432/edgequake
      - OPENAI_API_KEY=${OPENAI_API_KEY}
    depends_on:
      postgres:
        condition: service_healthy
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3

  postgres:
    image: postgres:16
    environment:
      POSTGRES_DB: edgequake
      POSTGRES_USER: edgequake
      POSTGRES_PASSWORD: password
    volumes:
      - postgres_data:/var/lib/postgresql/data
      - ./init.sql:/docker-entrypoint-initdb.d/init.sql
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U edgequake"]
      interval: 10s
      timeout: 5s
      retries: 5

  webui:
    build:
      context: ../edgequake_webui
      dockerfile: Dockerfile
    ports:
      - "3000:3000"
    environment:
      - NEXT_PUBLIC_API_URL=http://edgequake-api:8080
    depends_on:
      - edgequake-api

volumes:
  postgres_data:
```

### API Dockerfile

```dockerfile
# edgequake/docker/Dockerfile
FROM rust:1.83-slim-bookworm AS builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace
COPY . .

# Build release binary
RUN cargo build --release --package edgequake-api

# Runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/edgequake-api /usr/local/bin/

EXPOSE 8080

CMD ["edgequake-api"]
```

### WebUI Dockerfile

```dockerfile
# edgequake_webui/Dockerfile
FROM node:22-slim AS builder

WORKDIR /app

COPY package.json package-lock.json ./
RUN npm ci

COPY . .
RUN npm run build

# Runtime
FROM node:22-slim

WORKDIR /app

COPY --from=builder /app/.next ./.next
COPY --from=builder /app/node_modules ./node_modules
COPY --from=builder /app/package.json ./
COPY --from=builder /app/public ./public

EXPOSE 3000

CMD ["npm", "start"]
```

### PostgreSQL Extensions

```sql
-- docker/init.sql
-- Install required extensions for EdgeQuake

-- Vector similarity search
CREATE EXTENSION IF NOT EXISTS vector;

-- Graph queries (Apache AGE)
CREATE EXTENSION IF NOT EXISTS age;

-- Full-text search
CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- UUID generation
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
```

---

## Manual Deployment

### Build API Server

```bash
cd edgequake

# Install Rust toolchain
rustup install stable
rustup default stable

# Build release binary
cargo build --release --package edgequake-api

# Binary location
ls -la target/release/edgequake-api
```

### Build WebUI

```bash
cd edgequake_webui

# Install dependencies
npm ci

# Build production
npm run build

# Output in .next/
```

### Run API Server

```bash
# Set environment
export DATABASE_URL="postgresql://user:pass@localhost:5432/edgequake"
export OPENAI_API_KEY="sk-..."
export HOST="0.0.0.0"
export PORT="8080"

# Run (binary is named 'edgequake', not 'edgequake-api')
./target/release/edgequake
```

### Run WebUI

```bash
cd edgequake_webui

# Set API URL
export NEXT_PUBLIC_API_URL="http://localhost:8080"

# Start production server
npm start
```

### Systemd Service

```ini
# /etc/systemd/system/edgequake.service
[Unit]
Description=EdgeQuake API Server
After=network.target postgresql.service

[Service]
Type=simple
User=edgequake
Group=edgequake
WorkingDirectory=/opt/edgequake
ExecStart=/opt/edgequake/edgequake
Restart=on-failure
RestartSec=5

Environment=HOST=0.0.0.0
Environment=PORT=8080
EnvironmentFile=/etc/edgequake/config.env

[Install]
WantedBy=multi-user.target
```

```bash
# Enable and start
sudo systemctl enable edgequake
sudo systemctl start edgequake
sudo systemctl status edgequake
```

---

## Kubernetes Deployment

### Namespace and Secrets

```yaml
# k8s/namespace.yaml
apiVersion: v1
kind: Namespace
metadata:
  name: edgequake
---
# k8s/secrets.yaml
apiVersion: v1
kind: Secret
metadata:
  name: edgequake-secrets
  namespace: edgequake
type: Opaque
stringData:
  openai-api-key: "sk-your-key"
  database-url: "postgresql://user:pass@postgres:5432/edgequake"
```

### API Deployment

```yaml
# k8s/api-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: edgequake-api
  namespace: edgequake
spec:
  replicas: 3
  selector:
    matchLabels:
      app: edgequake-api
  template:
    metadata:
      labels:
        app: edgequake-api
    spec:
      containers:
        - name: api
          image: edgequake/api:latest
          ports:
            - containerPort: 8080
          env:
            - name: OPENAI_API_KEY
              valueFrom:
                secretKeyRef:
                  name: edgequake-secrets
                  key: openai-api-key
            - name: DATABASE_URL
              valueFrom:
                secretKeyRef:
                  name: edgequake-secrets
                  key: database-url
          resources:
            requests:
              cpu: "500m"
              memory: "512Mi"
            limits:
              cpu: "2000m"
              memory: "2Gi"
          livenessProbe:
            httpGet:
              path: /live
              port: 8080
            initialDelaySeconds: 10
            periodSeconds: 30
          readinessProbe:
            httpGet:
              path: /ready
              port: 8080
            initialDelaySeconds: 5
            periodSeconds: 10
---
apiVersion: v1
kind: Service
metadata:
  name: edgequake-api
  namespace: edgequake
spec:
  selector:
    app: edgequake-api
  ports:
    - port: 8080
      targetPort: 8080
  type: ClusterIP
```

### Ingress

```yaml
# k8s/ingress.yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: edgequake-ingress
  namespace: edgequake
  annotations:
    kubernetes.io/ingress.class: nginx
    cert-manager.io/cluster-issuer: letsencrypt-prod
spec:
  tls:
    - hosts:
        - api.edgequake.example.com
      secretName: edgequake-tls
  rules:
    - host: api.edgequake.example.com
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: edgequake-api
                port:
                  number: 8080
```

---

## Configuration

### Environment Variables

#### API Server

> **Note**: See [edgequake/src/main.rs](../edgequake/src/main.rs) for actual environment variable names

| Variable         | Required | Default   | Description              |
| ---------------- | -------- | --------- | ------------------------ |
| `HOST`           | No       | `0.0.0.0` | Listen address           |
| `PORT`           | No       | `8080`    | Listen port              |
| `DATABASE_URL`   | Prod     | -         | PostgreSQL connection    |
| `OPENAI_API_KEY` | Prod     | -         | OpenAI API key           |
| `WORKER_THREADS` | No       | CPU cores | Number of worker threads |
| `RUST_LOG`       | No       | `info`    | Log level                |

#### WebUI

| Variable              | Required | Default | Description       |
| --------------------- | -------- | ------- | ----------------- |
| `NEXT_PUBLIC_API_URL` | Yes      | -       | EdgeQuake API URL |
| `PORT`                | No       | `3000`  | Listen port       |

### Configuration File

```toml
# config.toml (optional)
[api]
host = "0.0.0.0"
port = 8080
cors_origins = ["http://localhost:3000"]

[storage]
type = "postgresql"

[storage.postgresql]
connection_string = "postgresql://user:pass@localhost:5432/edgequake"
pool_size = 10

[llm]
provider = "openai"
model = "gpt-4o-mini"
embedding_model = "text-embedding-3-small"
temperature = 0.0

[pipeline]
chunk_size = 1200
chunk_overlap = 100

[query]
top_k = 60
similarity_threshold = 0.5
```

---

## Monitoring & Health

### Health Endpoints

| Endpoint       | Purpose            | Expected Response       |
| -------------- | ------------------ | ----------------------- |
| `GET /health`  | Overall health     | `{"status": "healthy"}` |
| `GET /live`    | Liveness probe     | `{"live": true}`        |
| `GET /ready`   | Readiness probe    | `{"ready": true}`       |
| `GET /metrics` | Prometheus metrics | Prometheus format       |

### Prometheus Metrics

```yaml
# prometheus.yml
scrape_configs:
  - job_name: "edgequake"
    static_configs:
      - targets: ["edgequake-api:8080"]
    metrics_path: "/metrics"
```

### Key Metrics

| Metric                                | Type      | Description         |
| ------------------------------------- | --------- | ------------------- |
| `edgequake_requests_total`            | Counter   | Total HTTP requests |
| `edgequake_request_duration_seconds`  | Histogram | Request latency     |
| `edgequake_documents_processed_total` | Counter   | Documents ingested  |
| `edgequake_query_latency_seconds`     | Histogram | Query response time |
| `edgequake_llm_tokens_used`           | Counter   | LLM token usage     |

### Logging

```rust
// Structured JSON logging for production
// Set RUST_LOG=info or RUST_LOG=edgequake=debug
```

```bash
# Example log output
{"timestamp":"2025-01-15T10:30:00Z","level":"INFO","target":"edgequake_api","message":"Server listening","host":"0.0.0.0","port":8080}
{"timestamp":"2025-01-15T10:30:05Z","level":"INFO","target":"edgequake_api::handlers","message":"Document uploaded","doc_id":"abc123","chunks":5}
```

---

## Troubleshooting

### Quick Diagnosis Table

| Symptom | Likely Cause | Solution |
| ------- | ------------ | -------- |
| API returns 502 | Container/process not running | Check `docker ps` or `systemctl status edgequake` |
| Connection refused :8080 | Wrong host binding | Set `HOST=0.0.0.0` not `localhost` |
| "database connection failed" | Wrong DATABASE_URL | Verify credentials with `psql $DATABASE_URL -c "SELECT 1"` |
| "extension not found" | Missing PostgreSQL extensions | Run `CREATE EXTENSION IF NOT EXISTS vector;` etc. |
| "unauthorized" from OpenAI | Invalid/expired API key | Test key: `curl -H "Authorization: Bearer $OPENAI_API_KEY" https://api.openai.com/v1/models` |
| High latency (>5s) | Cold LLM or large queries | Check `top_k` setting, reduce to 30-40 for faster response |
| Out of memory | Too many concurrent requests | Increase memory limits or add replicas |
| "Too many connections" | Connection pool exhausted | Increase `pool_size` or use PgBouncer |
| Metrics not appearing | Wrong scrape path | Verify `/metrics` endpoint returns Prometheus format |
| SSL handshake failed | Certificate issues | Check cert chain, verify CN matches domain |

### Common Issues

#### API Won't Start

```bash
# Check port availability
lsof -i :8080

# Check database connection
psql $DATABASE_URL -c "SELECT 1"

# Check logs
docker compose logs edgequake-api
```

#### Database Connection Errors

```bash
# Verify extensions
psql $DATABASE_URL -c "\dx"

# Required extensions:
# - vector
# - age
# - pg_trgm
```

#### LLM API Errors

```bash
# Test API key
curl https://api.openai.com/v1/models \
  -H "Authorization: Bearer $OPENAI_API_KEY"

# Check rate limits
curl -I https://api.openai.com/v1/models \
  -H "Authorization: Bearer $OPENAI_API_KEY" 2>&1 | grep -i rate
```

### Debug Mode

For detailed troubleshooting, enable debug logging:

```bash
# Full debug output
export RUST_LOG=debug

# Module-specific debugging
export RUST_LOG=edgequake=debug,edgequake_api=trace

# Database query logging
export RUST_LOG=sqlx=debug

# LLM request/response logging
export RUST_LOG=edgequake_llm=debug
```

### Performance Tuning

```bash
# PostgreSQL tuning
ALTER SYSTEM SET shared_buffers = '2GB';
ALTER SYSTEM SET effective_cache_size = '6GB';
ALTER SYSTEM SET work_mem = '256MB';
ALTER SYSTEM SET maintenance_work_mem = '512MB';
SELECT pg_reload_conf();

# Connection pooling
# Use PgBouncer for high-concurrency deployments
```

---

## Security Considerations

### Network Security

```yaml
# Example: Restrict database to internal network only
# docker-compose.prod.yml
services:
  postgres:
    networks:
      - internal
    # NO ports exposed to host

  edgequake-api:
    networks:
      - internal
      - external
    ports:
      - "8080:8080"  # Only API exposed

networks:
  internal:
    internal: true  # No external access
  external:
```

### Secrets Management

**DO NOT** store secrets in:
- Docker images
- Git repositories
- Environment files committed to version control

**DO** use:
- Kubernetes Secrets (with encryption at rest)
- HashiCorp Vault
- AWS Secrets Manager / Azure Key Vault
- Docker Swarm secrets

```bash
# Example: Docker secrets
echo "sk-your-real-key" | docker secret create openai_api_key -

# Reference in compose
services:
  edgequake-api:
    secrets:
      - openai_api_key
    environment:
      - OPENAI_API_KEY_FILE=/run/secrets/openai_api_key
```

### API Security Checklist

- [ ] **HTTPS only** - Redirect HTTP to HTTPS
- [ ] **Rate limiting** - Prevent abuse (100 req/min default)
- [ ] **CORS restricted** - Only allow known origins
- [ ] **Input size limits** - Max 10MB document upload
- [ ] **Authentication** - Implement API keys or OAuth2
- [ ] **Audit logging** - Log all document operations

---

## Backup & Recovery

```bash
# Backup PostgreSQL
pg_dump $DATABASE_URL > backup.sql

# Restore
psql $DATABASE_URL < backup.sql

# Backup with pg_dump (compressed)
pg_dump -Fc $DATABASE_URL > backup.dump
pg_restore -d $DATABASE_URL backup.dump
```

---

## Next Steps

| Document | When to Read |
| -------- | ------------ |
| [Configuration Reference](0007-configuration-reference.md) | For all environment variables and config options |
| [API Reference](0003-api-reference.md) | To integrate with the API |
| [Storage Backends](0004-storage-backends.md) | For database tuning and migration |
| [Multi-Tenancy](0008-multi-tenancy.md) | For enterprise isolation patterns |
| [LLM Integration](0005-llm-integration.md) | To configure alternative LLM providers |

---

**Document Navigation**: [← Storage Backends](0004-storage-backends.md) | [README](README.md) | [Configuration Reference →](0007-configuration-reference.md)
