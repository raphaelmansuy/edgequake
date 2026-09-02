---
title: 'Deployment Guide'
---

> **Product: v0.26.5** · Contract: OpenAPI · Spec ops: [Ingestion cancel & fairness](../ingestion-cancel-and-fairness.md)

# Deployment Guide

> **Deploying EdgeQuake to Production**

This guide covers deploying EdgeQuake in production environments, from single-server setups to containerized deployments with PostgreSQL.

---

## ⚡ Quickstart — One Command (~30 seconds)

> **Fastest path:** no Rust toolchain, no Node.js, no `cargo build`.  
> Prebuilt multi-arch images (amd64 + arm64) are pulled from GitHub Container Registry.

```bash
# Clone repo (or just download the compose file)
git clone https://github.com/raphaelmansuy/edgequake.git
cd edgequake

# Pull images and start all services
make stack
```

Without `make`:
```bash
docker compose -f docker-compose.quickstart.yml up -d
```

**Access:**

| Service   | URL                              |
| --------- | -------------------------------- |
| 🌐 Web UI  | http://localhost:3000            |
| 🔗 API     | http://localhost:8080            |
| 📚 Swagger | http://localhost:8080/swagger-ui |
| 🏥 Health  | http://localhost:8080/health     |

**Stop:**
```bash
make stack-down
```

**Use OpenAI instead of Ollama:**
```bash
EDGEQUAKE_LLM_PROVIDER=openai OPENAI_API_KEY=sk-... make stack
```

**Pin to a specific version:**
```bash
EDGEQUAKE_VERSION=0.23.0 make stack
```

**Production auth** (auth is on by default; quickstart uses `EDGEQUAKE_DEV_MODE=true` for open API):
```bash
EDGEQUAKE_VERSION=0.23.0 \
EDGEQUAKE_DEV_MODE=false \
EDGEQUAKE_AUTH_ENABLED=true \
EDGEQUAKE_BOOTSTRAP_ADMIN_PASSWORD='ChangeMe123!' \
  docker compose -f docker-compose.quickstart.yml up -d
```

For full documentation see: [Docker Quickstart Guide](./docker-quickstart.md)

---

## Deployment Options

| Option                            | Complexity | Cold Start | Best For                                 |
| --------------------------------- | ---------- | ---------- | ---------------------------------------- |
| **`make stack`** (GHCR images)    | ⭐ Lowest   | ~30 s      | Local dev, demos, quick evaluation       |
| `make docker-prebuilt` (GHCR)     | Low        | ~45 s      | Staging / production with pinned version |
| `make docker-up` (build from src) | Medium     | 5–15 min   | Custom builds, self-hosted               |
| Binary + PostgreSQL               | Low        | N/A        | Bare metal / VMs                         |
| Kubernetes                        | High       | N/A        | Scale, HA, enterprise                    |

---

## Prerequisites

### Required

- PostgreSQL 16+ (recommended: PG18 via GHCR image) with extensions:
  - `pgvector` **0.8.3** (vector similarity search)
  - `age` **1.6.0** (PG16) or **1.7.0** (PG17/PG18) — Apache AGE for graph storage
- LLM provider access (OpenAI API key or Ollama running)

See [Release & CD](./release-and-cd.md#postgresql-version-tiers) for the triple-track pin matrix.

### Recommended

- 4+ CPU cores
- 8GB+ RAM (16GB for large corpora)
- SSD storage
- Docker (for containerized deployments)

---

## Option 1: Binary Deployment

### Step 1: Build Release Binary

```bash
cd edgequake
cargo build --release
```

The binary is at `target/release/edgequake` (~15MB).

### Step 2: Set Up PostgreSQL

Install PostgreSQL 16+ and extensions (pins match GHCR postgres image):

```bash
# macOS with Homebrew (example: PG17)
brew install postgresql@17
brew services start postgresql@17

# Build pgvector 0.8.3
git clone --branch v0.8.3 https://github.com/pgvector/pgvector.git
cd pgvector && make && make install

# Build Apache AGE (pick branch for your PG major)
# PG16 → RELEASE_1.6.0   PG17/PG18 → RELEASE_1.7.0
git clone --branch RELEASE_1.7.0 https://github.com/apache/age.git
cd age && make && make install
```

### Step 3: Create Database

```sql
-- Connect as superuser
psql -U postgres

-- Create user and database
CREATE USER edgequake WITH PASSWORD 'your_secure_password';
CREATE DATABASE edgequake OWNER edgequake;

-- Connect to database
\c edgequake

-- Enable extensions
CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS age;
LOAD 'age';
SET search_path = ag_catalog, "$user", public;
SELECT create_graph('edgequake_graph');
```

### Step 4: Configure and Run

```bash
# Set environment variables
export DATABASE_URL="postgresql://edgequake:your_secure_password@localhost:5432/edgequake"
export OPENAI_API_KEY="sk-your-key"  # Or use Ollama
export RUST_LOG="edgequake=info,tower_http=info"

# Run the server
./target/release/edgequake
```

### Step 5: Systemd Service (Linux)

Create `/etc/systemd/system/edgequake.service`:

```ini
[Unit]
Description=EdgeQuake RAG Server
After=network.target postgresql.service
Requires=postgresql.service

[Service]
Type=simple
User=edgequake
Group=edgequake
WorkingDirectory=/opt/edgequake
ExecStart=/opt/edgequake/edgequake
Restart=on-failure
RestartSec=5
Environment=DATABASE_URL=postgresql://edgequake:password@localhost:5432/edgequake
Environment=OPENAI_API_KEY=sk-your-key
Environment=RUST_LOG=edgequake=info,tower_http=info
Environment=HOST=0.0.0.0
Environment=PORT=8080

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl daemon-reload
sudo systemctl enable edgequake
sudo systemctl start edgequake
```

---

## Option 2: Docker Compose (Recommended)

### Step 1: Create Environment File

Create `.env` in project root (see `.env.example` for full reference):

```bash
# Database (quickstart default: edgequake_secret)
POSTGRES_PASSWORD=your_secure_password_here

# LLM Provider (choose one)
OPENAI_API_KEY=sk-your-key
# OR for Ollama:
EDGEQUAKE_LLM_PROVIDER=ollama
OLLAMA_HOST=http://host.docker.internal:11434
OLLAMA_MODEL=gemma4:latest
OLLAMA_EMBEDDING_MODEL=embeddinggemma:latest

# Server (optional)
EDGEQUAKE_PORT=8080
FRONTEND_PORT=3000
EDGEQUAKE_VERSION=0.23.0

# Auth (production — auth is ON by default; quickstart sets EDGEQUAKE_DEV_MODE=true)
EDGEQUAKE_DEV_MODE=false
EDGEQUAKE_AUTH_ENABLED=true
EDGEQUAKE_BOOTSTRAP_ADMIN_USERNAME=admin
EDGEQUAKE_BOOTSTRAP_ADMIN_PASSWORD=ChangeMe123!
JWT_SECRET=your-256-bit-secret-here

# Multi-replica (SPEC-057 — required when EDGEQUAKE_REPLICAS>1)
# EDGEQUAKE_REPLICAS=2
# EDGEQUAKE_TASK_DELIVERY=bridged
# EDGEQUAKE_TASK_LEASE_TTL_SECS=120
```

### Step 2: Start Services

```bash
cd edgequake/docker
docker compose up -d
```

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                   DOCKER COMPOSE STACK                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────┐         ┌─────────────────┐                │
│  │   edgequake     │ ──────▶ │   postgres      │                │
│  │   (API Server)  │         │   (pgvector+AGE)│                │
│  │   :8080         │         │   :5432         │                │
│  └─────────────────┘         └─────────────────┘                │
│          │                                                      │
│          ▼                                                      │
│  ┌─────────────────┐                                            │
│  │  External LLM   │                                            │
│  │  (OpenAI/Ollama)│                                            │
│  └─────────────────┘                                            │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Step 3: Verify Deployment

```bash
# Check service health
docker compose ps

# Test health endpoint
curl http://localhost:8080/health

# View logs
docker compose logs -f edgequake
```

### Step 4: Add Frontend (Optional)

For the full stack with frontend, create `docker-compose.full.yml`:

```yaml
services:
  edgequake:
    # ... (from docker-compose.yml)

  postgres:
    # ... (from docker-compose.yml)

  frontend:
    build:
      context: ../edgequake_webui
      dockerfile: Dockerfile
    container_name: edgequake-frontend
    ports:
      - "3000:3000"
    environment:
      - NEXT_PUBLIC_API_URL=http://edgequake:8080
    depends_on:
      - edgequake
    networks:
      - edgequake-network
```

---

## Option 3: Kubernetes

EdgeQuake ships Helm charts for Kubernetes deployments with optional in-cluster Langfuse v4 (OTLP trace observability).

**Operator guide (start here):** [deploy/kubernetes/README.md](../../deploy/kubernetes/README.md)  
**Spec pack:** [specs/138-kubernetes/README.md](../../specs/138-kubernetes/README.md)

### What gets deployed

- **edgequake** namespace: web + API + PostgreSQL (pgvector + AGE)
- **langfuse** namespace: Langfuse v4 (web, worker, bundled stores on kind)
- API exports **OTLP/HTTP traces** to Langfuse v4 (SPEC-124). Self-hosted Langfuse **3.1.x** has no OTLP path; default `EDGEQUAKE_LANGFUSE_API=auto` falls back to native ingestion. How-to: [langfuse-3.1.md](langfuse-3.1.md) · Helm: [Existing Langfuse 3.1.x](../../deploy/kubernetes/README.md#existing-langfuse-31x). Upgrade to ≥ 3.22 remains recommended.

### Quick start (kind / local)

```bash
make k8s-prereqs          # cert-manager + ClickHouse.com operator + nginx
make k8s-kind-up          # create kind cluster (edgequake-spec138)
make k8s-install          # Langfuse then EdgeQuake (includes migrate Job)
make k8s-status
```

Verify OTLP trace delivery:

```bash
make spec138-kubernetes-proof
```

### Important behavior (v0.26+)

| Topic | Behavior |
|-------|----------|
| **Migrations** | API **does not** auto-migrate at boot. Helm runs a `edgequake migrate` Job before the API Deployment serves traffic. |
| **Kind E2E LLM** | Uses `mock` provider with `EDGEQUAKE_ALLOW_MOCK_PROVIDER=1` — production must use a real provider. |
| **Langfuse memory** | Langfuse v4 web needs `NODE_OPTIONS=--max-old-space-size=1536` on kind (see `langfuse-values-kind.yaml`). |
| **Langfuse prereqs** | Langfuse Helm v2 requires the **ClickHouse.com** operator (`make k8s-prereqs`), not Altinity. |

See [deploy/kubernetes/README.md — Troubleshooting](../../deploy/kubernetes/README.md#troubleshooting) for common failure modes.

### Charts

| Chart | Path |
|-------|------|
| EdgeQuake app | `deploy/kubernetes/helm/edgequake/` |
| Stack wrapper | `deploy/kubernetes/helm/edgequake-stack/` |
| Langfuse values (kind) | `deploy/kubernetes/helm/langfuse-values-kind.yaml` |

Langfuse installs into namespace `langfuse`; EdgeQuake into namespace `edgequake` (separate Postgres instances).

### Reference manifests

The snippets below remain useful for understanding probe and env wiring. Prefer Helm for production installs.

### Namespace

```yaml
apiVersion: v1
kind: Namespace
metadata:
  name: edgequake
```

### ConfigMap

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: edgequake-config
  namespace: edgequake
data:
  RUST_LOG: "edgequake=info,tower_http=info"
  HOST: "0.0.0.0"
  PORT: "8080"
```

### Secret

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: edgequake-secrets
  namespace: edgequake
type: Opaque
stringData:
  DATABASE_URL: "postgresql://edgequake:password@postgres:5432/edgequake"
  OPENAI_API_KEY: "sk-your-key"
```

### Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: edgequake
  namespace: edgequake
spec:
  replicas: 2
  selector:
    matchLabels:
      app: edgequake
  template:
    metadata:
      labels:
        app: edgequake
    spec:
      containers:
        - name: edgequake
          image: ghcr.io/raphaelmansuy/edgequake:0.23.0
          ports:
            - containerPort: 8080
          envFrom:
            - configMapRef:
                name: edgequake-config
            - secretRef:
                name: edgequake-secrets
          resources:
            requests:
              cpu: "500m"
              memory: "512Mi"
            limits:
              cpu: "2000m"
              memory: "2Gi"
          livenessProbe:
            httpGet:
              path: /health
              port: 8080
            initialDelaySeconds: 10
            periodSeconds: 30
          readinessProbe:
            httpGet:
              path: /health
              port: 8080
            initialDelaySeconds: 5
            periodSeconds: 10
```

### Service

```yaml
apiVersion: v1
kind: Service
metadata:
  name: edgequake
  namespace: edgequake
spec:
  selector:
    app: edgequake
  ports:
    - port: 8080
      targetPort: 8080
  type: ClusterIP
```

---

## Environment Variables Reference

| Variable                 | Required       | Default                  | Description                  |
| ------------------------ | -------------- | ------------------------ | ---------------------------- |
| `DATABASE_URL`           | For PostgreSQL | None                     | PostgreSQL connection string |
| `OPENAI_API_KEY`         | For OpenAI     | None                     | OpenAI API key               |
| `OLLAMA_HOST`            | For Ollama     | `http://localhost:11434` | Ollama server URL            |
| `OLLAMA_MODEL`           | For Ollama     | `gemma4:latest`          | Ollama model for LLM         |
| `OLLAMA_EMBEDDING_MODEL` | For Ollama     | `embeddinggemma:latest`  | Ollama model for embeddings  |
| `HOST`                   | No             | `0.0.0.0`                | Server bind address          |
| `PORT`                   | No             | `8080`                   | Server port                  |
| `RUST_LOG`               | No             | `edgequake=debug`        | Log level                    |
| `WORKER_THREADS`         | No             | CPU count                | Background worker count      |
| `EDGEQUAKE_DEV_MODE`     | No             | `false` (product)        | Open API without login (quickstart: `true`) |
| `EDGEQUAKE_AUTH_ENABLED` | No             | `true`                   | Require JWT/API key on protected routes |
| `EDGEQUAKE_BOOTSTRAP_ADMIN_USERNAME` | When auth on | `admin` | First-run admin username |
| `EDGEQUAKE_BOOTSTRAP_ADMIN_PASSWORD` | When auth on | — | First-run admin password (required on fresh DB) |
| `JWT_SECRET`             | When auth on   | —                        | JWT signing secret (256-bit recommended) |
| `EDGEQUAKE_REPLICAS`     | No             | `1`                      | Intended API/worker process count |
| `EDGEQUAKE_TASK_DELIVERY`| No             | `local`                  | `local` \| `bridged` \| `notify_only` (required non-`local` when replicas > 1) |
| `EDGEQUAKE_TASK_LEASE_TTL_SECS` | No      | `120`                    | Task claim lease TTL (min 30; heartbeat every 60s) |

See [Ingestion cancel & fairness](../ingestion-cancel-and-fairness.md) for multi-replica delivery, lease, and restart semantics.

---

## Auth Bootstrap (SPEC-027)

Auth is **enabled by default** in v0.23.0. On a fresh PostgreSQL database, set bootstrap credentials before first boot:

```bash
EDGEQUAKE_AUTH_ENABLED=true
EDGEQUAKE_BOOTSTRAP_ADMIN_USERNAME=admin
EDGEQUAKE_BOOTSTRAP_ADMIN_PASSWORD='ChangeMe123!'
JWT_SECRET='your-256-bit-secret-here'
```

Quickstart compose (`docker-compose.quickstart.yml`) defaults to `EDGEQUAKE_DEV_MODE=true` and `EDGEQUAKE_AUTH_ENABLED=false` for frictionless demos. **Never use `EDGEQUAKE_DEV_MODE=true` in production.**

Upgrades from pre-v0.15 installs: legacy KV `auth:user:*` records are imported automatically on startup.

---

## Multi-Replica & Task Delivery (SPEC-057)

When running more than one API/worker process against shared PostgreSQL:

| Variable | Default | Notes |
| -------- | ------- | ----- |
| `EDGEQUAKE_REPLICAS` | `1` | Set to intended replica count |
| `EDGEQUAKE_TASK_DELIVERY` | `local` | Must be `bridged` or `notify_only` when replicas > 1 |
| `EDGEQUAKE_TASK_LEASE_TTL_SECS` | `120` | Claim lease TTL; workers refresh every 60s |

Boot **fails** if `EDGEQUAKE_REPLICAS>1` and delivery is `local`. Correctness is always `claim_next` + lease — bridged/notify_only are **wake modes only**; never process from a channel payload without claim.

---

## Storage Modes

EdgeQuake automatically selects storage based on `DATABASE_URL`:

```
┌─────────────────────────────────────────────────────────────────┐
│                   STORAGE MODE SELECTION                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  DATABASE_URL set?                                              │
│       │                                                         │
│       ├── YES ─────▶ PostgreSQL Mode                            │
│       │              • Persistent storage                       │
│       │              • pgvector for embeddings                  │
│       │              • Apache AGE for graph                     │
│       │              • Full multi-tenant support                │
│       │                                                         │
│       └── NO ──────▶ ❌ Error: DATABASE_URL required             │
│                      • Server exits with code 1                 │
│                      • Set DATABASE_URL to proceed              │
│                      • In-memory mode removed in v0.4.0         │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Health Checks

EdgeQuake provides health endpoints for monitoring:

| Endpoint      | Purpose         | Response                                                  |
| ------------- | --------------- | --------------------------------------------------------- |
| `GET /health` | Basic health    | `status`, `version`, `storage_mode`, `components`, `llm_provider_name`, `capabilities` |
| `GET /ready`  | Readiness check | Storage + LLM status                                      |
| `GET /live`   | Liveness check  | Process alive                                             |

### Docker Healthcheck

The API image is distroless (no `curl`/`wget`/`sh`). The binary probes `GET /live`:

```dockerfile
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD ["/usr/local/bin/edgequake", "healthcheck"]
```

### Kubernetes Probes

```yaml
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
```

---

## Reverse Proxy Configuration

### Nginx

```nginx
upstream edgequake {
    server localhost:8080;
    keepalive 32;
}

server {
    listen 443 ssl http2;
    server_name rag.yourdomain.com;

    ssl_certificate /etc/ssl/certs/your-cert.pem;
    ssl_certificate_key /etc/ssl/private/your-key.pem;

    location / {
        proxy_pass http://edgequake;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # SSE support for streaming — gzip MUST NOT apply to text/event-stream.
        # Next.js / nginx gzip buffers the whole body (one chunk at EOF).
        # Axum already skips SSE compression; reverse proxies must too.
        proxy_buffering off;
        proxy_cache off;
        proxy_read_timeout 86400;
        gzip off;
    }

    # JSON/static can still be compressed; keep SSE on a dedicated location if
    # you need gzip elsewhere:
    # location /api/v1/query/stream { gzip off; proxy_buffering off; ... }
}
```

### Caddy

```caddy
rag.yourdomain.com {
    reverse_proxy localhost:8080 {
        header_up X-Real-IP {remote_host}
        flush_interval -1
    }
}
```

Do not wrap the API with `encode gzip` on SSE paths (`/api/v1/query/stream`,
`/api/v1/chat/completions/stream`, `/api/v1/graph/stream`). Gzip on
`text/event-stream` buffers the stream the same way Next.js `compress: true` did.

### Traefik / Kubernetes Ingress

nginx ingress defaults to buffering. Set:

```yaml
nginx.ingress.kubernetes.io/proxy-buffering: "off"
nginx.ingress.kubernetes.io/proxy-read-timeout: "86400"
```

The Helm chart ships these annotations on `ingress.annotations`. Traefik
`compress` middleware must exclude `text/event-stream` (or be disabled on
stream routes). The API also sends `X-Accel-Buffering: no` on SSE responses.

---

## Security Checklist

- [ ] Use strong PostgreSQL password (not `edgequake_secret`)
- [ ] Set `EDGEQUAKE_DEV_MODE=false` and configure auth bootstrap credentials
- [ ] Keep `OPENAI_API_KEY` and `JWT_SECRET` in secrets manager
- [ ] Enable TLS termination at reverse proxy
- [ ] Set up firewall rules (only expose 443)
- [ ] Use non-root user in Docker
- [ ] Enable audit logging
- [ ] Set up backup for PostgreSQL
- [ ] Monitor rate limits on LLM providers
- [ ] Set `EDGEQUAKE_TASK_DELIVERY=bridged` (or `notify_only`) when `EDGEQUAKE_REPLICAS>1`

---

## See Also

- [Configuration Reference](/docs/operations/configuration/) - Detailed configuration options
- [Monitoring Guide](/docs/operations/monitoring/) - Observability setup
- [Quick Start](/docs/getting-started/quick-start/) - Development setup
- [Architecture Overview](/docs/architecture/overview/) - System design
