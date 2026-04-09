---
title: 'Docker Quickstart'
---

# Docker Quickstart — Full Stack in One Command

> **Zero prerequisites beyond Docker.**  
> No Rust, no Node.js, no `cargo build`, no `npm install`.  
> Cold start: **~30 seconds** on a fast connection (image layers cache after first pull).

---

## The One-Liner (no git clone required)

Copy and paste this into any terminal that has Docker:

```bash
curl -fsSL https://raw.githubusercontent.com/raphaelmansuy/edgequake/edgequake-main/docker-compose.quickstart.yml \
  | docker compose -f - up -d
```

That's it. Three versioned images (API, Web UI, PostgreSQL) are pulled from GitHub Container Registry and started.

**Then open:** http://localhost:3000

---

## Option A — Pure `docker compose` (download file first)

```bash
# 1. Download the compose file
curl -fsSL https://raw.githubusercontent.com/raphaelmansuy/edgequake/edgequake-main/docker-compose.quickstart.yml \
  -o docker-compose.quickstart.yml

# 2. Start the full stack
docker compose -f docker-compose.quickstart.yml up -d

# 3. Check health
curl http://localhost:8080/health
```

---

## Option B — With git clone + make

```bash
git clone https://github.com/raphaelmansuy/edgequake.git
cd edgequake
make stack
```

---

## Option C — Pinned version (production-stable)

```bash
# Download a specific version's compose file
EDGEQUAKE_VERSION=0.9.4
curl -fsSL "https://raw.githubusercontent.com/raphaelmansuy/edgequake/edgequake-main/docker-compose.quickstart.yml" \
  -o docker-compose.quickstart.yml

# Start with that version
EDGEQUAKE_VERSION=${EDGEQUAKE_VERSION} docker compose -f docker-compose.quickstart.yml up -d
```

---

## Access Points

| Service      | URL                              | Description                               |
| ------------ | -------------------------------- | ----------------------------------------- |
| 🌐 Web UI     | http://localhost:3000            | Upload documents, explore knowledge graph |
| 🔗 REST API   | http://localhost:8080            | Programmatic access                       |
| 📚 Swagger UI | http://localhost:8080/swagger-ui | Interactive API explorer                  |
| 🏥 Health     | http://localhost:8080/health     | JSON health status                        |

---

## LLM Providers

### Default: Ollama (free, local, no API key)

Requires [Ollama](https://ollama.ai) running on your machine:

```bash
# Install and run Ollama (once)
curl -fsSL https://ollama.ai/install.sh | sh
ollama serve &
ollama pull gemma3:latest

# Start EdgeQuake
docker compose -f docker-compose.quickstart.yml up -d
```

### OpenAI

```bash
EDGEQUAKE_LLM_PROVIDER=openai \
OPENAI_API_KEY=sk-... \
  docker compose -f docker-compose.quickstart.yml up -d
```

### Any OpenAI-compatible endpoint (LM Studio, vLLM, Azure, etc.)

```bash
EDGEQUAKE_LLM_PROVIDER=openai \
OPENAI_API_KEY=your-key \
OPENAI_BASE_URL=http://localhost:1234/v1 \
  docker compose -f docker-compose.quickstart.yml up -d
```

### Provider reference

| Variable                 | Default                             | Description                            |
| ------------------------ | ----------------------------------- | -------------------------------------- |
| `EDGEQUAKE_LLM_PROVIDER` | `ollama`                            | `ollama`, `openai`, `lmstudio`, `mock` |
| `EDGEQUAKE_LLM_MODEL`    | provider-specific default           | Main chat / extraction model           |
| `EDGEQUAKE_EMBEDDING_PROVIDER` | same as LLM                   | Override embedding provider            |
| `EDGEQUAKE_EMBEDDING_MODEL` | provider-specific default        | Embedding model override               |
| `OPENAI_API_KEY`         | _(empty)_                           | Required when provider is `openai`     |
| `OPENAI_BASE_URL`        | _(empty)_                           | Override OpenAI base URL               |
| `OLLAMA_HOST`            | `http://host.docker.internal:11434` | Ollama server address                  |
| `EDGEQUAKE_VERSION`      | `latest`                            | Pin to a specific release tag          |
| `EDGEQUAKE_PORT`         | `8080`                              | API port                               |
| `FRONTEND_PORT`          | `3000`                              | Web UI port                            |
| `POSTGRES_PASSWORD`      | `edgequake_secret`                  | PostgreSQL password                    |

### Migration aliases

If you are migrating from a LightRAG-style environment file, EdgeQuake also accepts:

| Alias | Canonical variable |
| --- | --- |
| `MODEL_PROVIDER` | `EDGEQUAKE_LLM_PROVIDER` |
| `CHAT_MODEL` | `EDGEQUAKE_LLM_MODEL` |
| `EMBEDDING_PROVIDER` | `EDGEQUAKE_EMBEDDING_PROVIDER` |
| `EMBEDDING_MODEL` | `EDGEQUAKE_EMBEDDING_MODEL` |
| `EMBEDDING_DIMENSION` | `EDGEQUAKE_EMBEDDING_DIMENSION` |

Canonical `EDGEQUAKE_*` variables always win when both are set.

---

## Managing the Stack

```bash
# Stop and remove containers (data persists in the edgequake-pg-data volume)
docker compose -f docker-compose.quickstart.yml down

# Stop and remove containers + data
docker compose -f docker-compose.quickstart.yml down -v

# Tail all logs
docker compose -f docker-compose.quickstart.yml logs -f

# Check container status
docker compose -f docker-compose.quickstart.yml ps

# Pull latest images (then restart)
docker compose -f docker-compose.quickstart.yml pull
docker compose -f docker-compose.quickstart.yml up -d

# Restart a single service
docker compose -f docker-compose.quickstart.yml restart api
```

---

## Images

All images are multi-arch (`linux/amd64`, `linux/arm64`) and published to GitHub Container Registry on every tagged release.

| Image                                      | Tag                | Description                        |
| ------------------------------------------ | ------------------ | ---------------------------------- |
| `ghcr.io/raphaelmansuy/edgequake`          | `latest` / `0.9.4` | Rust API server                    |
| `ghcr.io/raphaelmansuy/edgequake-frontend` | `latest` / `0.9.4` | Next.js Web UI                     |
| `ghcr.io/raphaelmansuy/edgequake-postgres` | `latest` / `0.9.4` | PostgreSQL + pgvector + Apache AGE |

Pull an image manually:
```bash
docker pull ghcr.io/raphaelmansuy/edgequake:latest
docker pull ghcr.io/raphaelmansuy/edgequake-frontend:latest
docker pull ghcr.io/raphaelmansuy/edgequake-postgres:latest
```

---

## Troubleshooting

### API never becomes healthy

```bash
# Check logs
docker compose -f docker-compose.quickstart.yml logs api

# Common cause: Ollama not running
curl http://localhost:11434/api/tags   # should return model list
ollama serve &                          # start if not running
```

### Port already in use

```bash
# Change ports
EDGEQUAKE_PORT=8081 FRONTEND_PORT=3001 \
  docker compose -f docker-compose.quickstart.yml up -d
```

### Reset all data

```bash
docker compose -f docker-compose.quickstart.yml down -v
docker compose -f docker-compose.quickstart.yml up -d
```

### Apple Silicon (M-series) / ARM64

Images include native `linux/arm64` layers — no QEMU emulation needed. Docker Desktop on macOS auto-selects the right architecture.

---

## Next Steps

- [REST API Reference](/docs/api-reference/) — upload documents, query, manage the graph
- [Configuration Guide](/docs/operations/configuration/) — tune LLM models, embedding dimensions, timeouts
- [Production Deployment](/docs/operations/deployment/) — TLS, secrets management, scaling
