---
title: "Installation Guide"
---

> **Product: v0.23.0** · Contract: [OpenAPI snapshot](../../edgequake_webui/openapi/openapi.snapshot.json) · Spec ops: [Ingestion cancel & fairness](../ingestion-cancel-and-fairness.md)

# Installation Guide

> Get EdgeQuake running on your machine in 5 minutes

---

## Prerequisites Checklist

Before installing, ensure you have:

| Requirement | Version    | Check Command      | Purpose                                  |
| ----------- | ---------- | ------------------ | ---------------------------------------- |
| **Rust**    | 1.95+      | `rustc --version`  | Build backend with the pinned toolchain  |
| **Cargo**   | via rustup | `cargo --version`  | Package manager and workspace tooling    |
| **Docker**  | 24+        | `docker --version` | Recommended path for required PostgreSQL |
| **Node.js** | 20+        | `node --version`   | WebUI and Playwright                     |
| **pnpm**    | 10+        | `pnpm --version`   | Frontend package manager                 |

### PostgreSQL (required)

EdgeQuake requires PostgreSQL **16, 17, or 18** with **pgvector** and **Apache AGE**. The Makefile default profile is **PG18** (`EQ_POSTGRES_PROFILE=pg18`); override with `make dev-pg16`, `make dev-pg17`, or `EQ_POSTGRES_PROFILE=pg17 make dev`.

Credentials must match across Docker and `DATABASE_URL`:

| Variable            | Value              |
| ------------------- | ------------------ |
| `POSTGRES_USER`     | `edgequake`        |
| `POSTGRES_PASSWORD` | `edgequake_secret` |
| `POSTGRES_DB`       | `edgequake`        |

```bash
export DATABASE_URL="postgresql://edgequake:edgequake_secret@localhost:5432/edgequake?options=-c%20search_path%3Dpublic"
```

### Authentication (production vs local dev)

| Mode | Auth | Setup |
| ---- | ---- | ----- |
| **`make dev`** (default) | On — `admin` / `EdgeQuake1` | Credentials printed at startup and on `/login` |
| **`make dev-open`** | Off (open API) | Escape hatch for rare open-API proofs |
| **Production** | On (secure by default) | Set `JWT_SECRET`, bootstrap admin credentials, `NEXT_PUBLIC_DISABLE_DEMO_LOGIN=true` |

See [Runtime auth hardening](../operations/runtime-auth-hardening.md).

### Vision LLM (PDF ingestion)

PDF uploads require a **vision-capable** model. Set explicitly or let resolution fall back from your LLM provider:

```bash
# Cloud (recommended for PDF quality)
EDGEQUAKE_VISION_PROVIDER=openai
EDGEQUAKE_VISION_MODEL=gpt-4.1-nano
OPENAI_API_KEY=sk-...

# Local (Ollama — pull a vision model)
ollama pull gemma4:latest
EDGEQUAKE_VISION_PROVIDER=ollama
EDGEQUAKE_VISION_MODEL=gemma4:latest
```

Verify after start: `GET /api/v1/config/effective` → Vision area (check `has_mismatch`).

---

## Quick Install Decision Tree

```
                     ┌─────────────────────┐
                     │ What's your goal?   │
                     └──────────┬──────────┘
                                │
              ┌─────────────────┼─────────────────┐
              │                 │                 │
              ▼                 ▼                 ▼
       ┌──────────┐      ┌──────────┐      ┌──────────┐
       │ Try it   │      │ Develop  │      │ Deploy   │
       │ quickly  │      │ locally  │      │ to prod  │
       └────┬─────┘      └────┬─────┘      └────┬─────┘
            │                 │                 │
            ▼                 ▼                 ▼
       make dev         make dev-bg       docker-compose
       (interactive)    (background)      .quickstart.yml
```

---

## Installation Options

### Option 1: Full Stack with Make (Recommended)

```bash
git clone https://github.com/raphaelmansuy/edgequake.git
cd edgequake
make dev
```

**What happens**:

1. Starts PostgreSQL (profile `pg18` by default) with password `edgequake_secret`
2. Runs database migrations
3. Builds and starts the Rust backend on port **8080**
4. Starts the Next.js frontend on port **3000** (shifts only if 3000 is taken)

**Verify**:

```bash
curl http://localhost:8080/health
# Expected: JSON containing "status":"healthy"

open http://localhost:3000
```

> Run `make status` if another stack is using port 3000.

---

### Option 2: Prebuilt GHCR Stack (No Rust/Node toolchain)

```bash
git clone https://github.com/raphaelmansuy/edgequake.git
cd edgequake

EDGEQUAKE_VERSION=0.23.0 docker compose -f docker-compose.quickstart.yml up -d
```

| Service    | Image                                              | Port |
| ---------- | -------------------------------------------------- | ---- |
| API        | `ghcr.io/raphaelmansuy/edgequake:0.23.0`           | 8080 |
| WebUI      | `ghcr.io/raphaelmansuy/edgequake-frontend:0.23.0`  | 3000 |
| PostgreSQL | `ghcr.io/raphaelmansuy/edgequake-postgres:0.23.0-pg18` | 5432 |

Pin PostgreSQL major: `EDGEQUAKE_POSTGRES_TAG=0.23.0-pg16` (or `-pg17`, `-pg18`).

---

### Option 3: Backend Only (For API Development)

```bash
git clone https://github.com/raphaelmansuy/edgequake.git
cd edgequake
make backend-bg
```

> `DATABASE_URL` is required. `make backend-bg` sets it to `postgresql://edgequake:edgequake_secret@localhost:5432/edgequake`.

**Verify**:

```bash
curl http://localhost:8080/health
```

---

### Option 4: Build from Source

```bash
git clone https://github.com/raphaelmansuy/edgequake.git
cd edgequake/edgequake
cargo build --release

export DATABASE_URL="postgresql://edgequake:edgequake_secret@localhost:5432/edgequake?options=-c%20search_path%3Dpublic"
./target/release/edgequake
```

---

### Option 5: Development Mode (Watch + Hot Reload)

```bash
# Terminal 1: PostgreSQL
make db-start

# Terminal 2: Backend with cargo-watch
cd edgequake
cargo watch -x run

# Terminal 3: Frontend
cd edgequake_webui
pnpm dev
```

---

## LLM Provider Configuration

EdgeQuake supports multiple LLM providers. Set canonical `EDGEQUAKE_DEFAULT_*` vars (see `.env.example`).

### Ollama (Free, Local) — Default for `make dev`

```bash
brew install ollama   # macOS
ollama pull gemma4:latest
ollama pull embeddinggemma:latest
ollama serve
make dev
```

### OpenAI (Paid, Cloud)

```bash
export OPENAI_API_KEY="sk-your-key"
make dev
```

### Google Vertex AI (Enterprise)

Uses IAM identity (ADC or service account), not `GEMINI_API_KEY`:

```bash
gcloud auth application-default login
export GOOGLE_CLOUD_PROJECT=your-gcp-project
make dev
```

See [Configuration — Vertex AI](/docs/operations/configuration#google-vertex-ai-enterprise).

### Provider Switching at Runtime

```bash
curl http://localhost:8080/api/v1/config/effective | jq '.llm'
```

---

## Storage Configuration

EdgeQuake uses PostgreSQL for all storage modes (since v0.4.0):

```
┌─────────────────────────────────────────────────────────────┐
│                     Storage (PostgreSQL)                    │
├─────────────────────────────────────────────────────────────┤
│         ┌─────────────────────────────────────┐            │
│         │     PostgreSQL 16 / 17 / 18          │            │
│         │  ┌──────────┐  ┌──────────────────┐ │            │
│         │  │ pgvector  │  │   Apache AGE     │ │            │
│         │  └──────────┘  └──────────────────┘ │            │
│         └─────────────────────────────────────┘            │
│  DATABASE_URL required. Password: edgequake_secret         │
└─────────────────────────────────────────────────────────────┘
```

### PostgreSQL Setup (Docker)

```bash
docker run -d \
  --name edgequake-postgres \
  -e POSTGRES_USER=edgequake \
  -e POSTGRES_PASSWORD=edgequake_secret \
  -e POSTGRES_DB=edgequake \
  -p 5432:5432 \
  ghcr.io/raphaelmansuy/edgequake-postgres:0.23.0-pg18

export DATABASE_URL="postgresql://edgequake:edgequake_secret@localhost:5432/edgequake?options=-c%20search_path%3Dpublic"

cd edgequake && sqlx database setup
```

> **Important:** `POSTGRES_PASSWORD` must be `edgequake_secret` to match `DATABASE_URL` used by Make, docker-compose, and `.env.example`.

---

## Verification Checklist

```bash
# 1. Toolchain
cd edgequake && rustc --version   # 1.95+

# 2. Backend health
curl -s http://localhost:8080/health | jq

# 3. OpenAPI contract
curl -s http://localhost:8080/api-docs/openapi.json | jq .info.title

# 4. Ollama (if local provider)
curl -s http://localhost:11434/api/tags | jq

# 5. Repo checks
cargo fmt --all --check
cargo clippy --workspace --lib -- -D warnings
cargo test --workspace --lib --no-fail-fast
```

### No-flake local workflow

```bash
make status
rustup show active-toolchain
```

If PostgreSQL is unavailable, EdgeQuake exits at startup with a clear error instead of failing mid-request.

---

## Troubleshooting

### Docker Issues

```bash
docker info                    # Docker running?
lsof -i :5432                  # Port conflict?
lsof -i :8080                  # API port
lsof -i :3000                  # WebUI port
```

### Rust Build Issues

```bash
rustup update stable
# Linux deps:
sudo apt-get install pkg-config libssl-dev libpq-dev
```

### LLM / Vision Issues

```bash
ollama serve && ollama list
curl -s http://localhost:8080/api/v1/config/effective | jq '.areas[] | select(.name == "Vision")'
```

### Auth Issues

- **401 on API calls after deploy:** Auth is on by default — add `Authorization: Bearer …` or `X-API-Key`, or use `EDGEQUAKE_DEV_MODE=true` locally only.
- **No login on first start:** Set `EDGEQUAKE_BOOTSTRAP_ADMIN_*` env vars before boot (see [runtime auth hardening](../operations/runtime-auth-hardening.md)).

---

## Next Steps

1. **[Quick Start](/docs/getting-started/quick-start/)** — Ingest your first document
2. **[Architecture Overview](/docs/architecture/overview/)** — Understand the system
3. **[API Reference](/docs/api-reference/rest-api/)** — Explore endpoints

---

## System Requirements

| Component | Minimum                      | Recommended  |
| --------- | ---------------------------- | ------------ |
| **RAM**   | 4 GB                         | 16 GB        |
| **CPU**   | 2 cores                      | 8 cores      |
| **Disk**  | 10 GB                        | 50 GB        |
| **OS**    | Linux, macOS, Windows (WSL2) | Linux, macOS |
