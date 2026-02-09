# Deployment Guide

> **Deploying EdgeQuake to Production**

This guide covers deploying EdgeQuake in production environments, from single-server setups to containerized deployments with PostgreSQL.

---

## Deployment Options

| Option              | Complexity | Best For                             |
| ------------------- | ---------- | ------------------------------------ |
| Binary + PostgreSQL | Low        | Single server, simple setups         |
| Docker Compose      | Medium     | Standard production deployments      |
| Kubernetes          | High       | Scale, high availability, enterprise |

---

## Prerequisites

### Required

- PostgreSQL 15+ with extensions:
  - `pgvector` 0.7+ (vector similarity search)
  - `age` 1.5+ (Apache AGE for graph storage)
- LLM provider access (OpenAI API key or Ollama running)

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

Install PostgreSQL 15+ and extensions:

```bash
# macOS with Homebrew
brew install postgresql@15
brew services start postgresql@15

# Build pgvector
git clone --branch v0.7.4 https://github.com/pgvector/pgvector.git
cd pgvector && make && make install

# Build Apache AGE
git clone --branch PG16/v1.6.0-rc0 https://github.com/apache/age.git
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

Create `.env` in project root:

```bash
# Database
POSTGRES_PASSWORD=your_secure_password_here

# LLM Provider (choose one)
OPENAI_API_KEY=sk-your-key
# OR for Ollama:
EDGEQUAKE_LLM_PROVIDER=ollama
OLLAMA_HOST=http://host.docker.internal:11434
OLLAMA_MODEL=gemma3:latest
OLLAMA_EMBEDDING_MODEL=nomic-embed-text

# Server (optional)
EDGEQUAKE_PORT=8080
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

### Helm Chart (Coming Soon)

For Kubernetes deployments, a Helm chart is in development. For now, use the following manifests as a starting point:

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
          image: edgequake/edgequake:latest
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
| `OLLAMA_MODEL`           | For Ollama     | `gemma3:latest`          | Ollama model for LLM         |
| `OLLAMA_EMBEDDING_MODEL` | For Ollama     | `nomic-embed-text`       | Ollama model for embeddings  |
| `HOST`                   | No             | `0.0.0.0`                | Server bind address          |
| `PORT`                   | No             | `8080`                   | Server port                  |
| `RUST_LOG`               | No             | `edgequake=debug`        | Log level                    |
| `WORKER_THREADS`         | No             | CPU count                | Background worker count      |

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
│       └── NO ──────▶ Memory Mode                                │
│                      • Ephemeral (data lost on restart)         │
│                      • For development/testing only             │
│                      • No external dependencies                 │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Health Checks

EdgeQuake provides health endpoints for monitoring:

| Endpoint            | Purpose         | Response             |
| ------------------- | --------------- | -------------------- |
| `GET /health`       | Basic health    | `{ "status": "ok" }` |
| `GET /health/ready` | Readiness check | Storage + LLM status |
| `GET /health/live`  | Liveness check  | Process alive        |

### Docker Healthcheck

```dockerfile
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1
```

### Kubernetes Probes

```yaml
livenessProbe:
  httpGet:
    path: /health/live
    port: 8080
  initialDelaySeconds: 10
  periodSeconds: 30

readinessProbe:
  httpGet:
    path: /health/ready
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

        # SSE support for streaming
        proxy_buffering off;
        proxy_cache off;
        proxy_read_timeout 86400;
    }
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

---

## Security Checklist

- [ ] Use strong PostgreSQL password
- [ ] Keep `OPENAI_API_KEY` in secrets manager
- [ ] Enable TLS termination at reverse proxy
- [ ] Set up firewall rules (only expose 443)
- [ ] Use non-root user in Docker
- [ ] Enable audit logging
- [ ] Set up backup for PostgreSQL
- [ ] Monitor rate limits on LLM providers

---

## See Also

- [Configuration Reference](configuration.md) - Detailed configuration options
- [Monitoring Guide](monitoring.md) - Observability setup
- [Quick Start](../getting-started/quick-start.md) - Development setup
- [Architecture Overview](../architecture/overview.md) - System design
