---
title: "Installation Guide"
---

# Installation Guide

> Get EdgeQuake running on your machine in 5 minutes

---

## Prerequisites Checklist

Before installing, ensure you have:

| Requirement | Version    | Check Command      | Purpose                                  |
| ----------- | ---------- | ------------------ | ---------------------------------------- |
| **Rust**    | 1.95.x     | `rustc --version`  | Build backend with the pinned toolchain  |
| **Cargo**   | via rustup | `cargo --version`  | Package manager and workspace tooling    |
| **Docker**  | 24+        | `docker --version` | Recommended path for required PostgreSQL |
| **Node.js** | 20+        | `node --version`   | WebUI and Playwright                     |
| **pnpm**    | 10+        | `pnpm --version`   | Frontend package manager                 |

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
       make dev         make dev-bg       Docker Compose
       (interactive)    (background)      (see Deployment)
```

---

## Installation Options

### Option 1: Full Stack with Make (Recommended)

```bash
# Clone the repository
git clone https://github.com/raphaelmansuy/edgequake.git
cd edgequake

# Start everything (PostgreSQL + Backend + Frontend)
make dev
```

**What happens**:

1. Ensures PostgreSQL is reachable on port 5432
2. Runs database migrations
3. Builds and starts the Rust backend on port 8080
4. Starts the Next.js frontend on port 3000 by default and automatically shifts only if that port is already in use

**Verify**:

```bash
# In a new terminal
curl http://localhost:8080/health
# Expected: JSON containing "status":"healthy"

# Open WebUI (default local Make-based port)
open <http://localhost:3000>
```

> If another stack is already using 3000, run `make status` to see the exact frontend URL that was selected.

---

### Option 2: Backend Only (For API Development)

```bash
# Clone and enter
git clone https://github.com/raphaelmansuy/edgequake.git
cd edgequake

# Start backend with PostgreSQL (required since v0.4.0)
make backend-bg
```

> **Note**: Starting with v0.4.0, `DATABASE_URL` is required for all server modes.
> In-memory storage was removed to ensure reliable, production-grade behavior.
> Use the Docker-based PostgreSQL setup above (`make dev`) for the fastest path.

**Verify**:

```bash
curl http://localhost:8080/health
```

---

### Option 3: Build from Source

```bash
# Clone
git clone https://github.com/raphaelmansuy/edgequake.git
cd edgequake

# Build release binary
cd edgequake
cargo build --release

# Binary location
ls target/release/edgequake

# Run directly (PostgreSQL is required)
export DATABASE_URL="postgresql://edgequake:edgequake_secret@localhost:5432/edgequake"
./target/release/edgequake
```

---

### Option 4: Development Mode (Watch + Hot Reload)

> Principle: prefer explicit health checks and the repository-pinned toolchain over ad-hoc local variations. That keeps local behavior consistent with CI.

```bash
# Terminal 1: Start PostgreSQL
make db-start

# Terminal 2: Run backend with cargo-watch
cd edgequake
cargo watch -x run

# Terminal 3: Run frontend with hot reload
cd edgequake_webui
pnpm dev
```

---

## LLM Provider Configuration

EdgeQuake supports multiple LLM providers:

### Ollama (Free, Local) — Default

```bash
# Install Ollama
brew install ollama  # macOS
# or: curl -fsSL https://ollama.com/install.sh | sh

# Pull models (Makefile defaults when no OPENAI_API_KEY)
ollama pull gemma4:latest
ollama pull embeddinggemma:latest

# Start Ollama (if not running)
ollama serve

# Start EdgeQuake (auto-detects Ollama)
make dev
```

### OpenAI (Paid, Cloud)

```bash
# Set API key
export OPENAI_API_KEY="sk-your-key"

# Start EdgeQuake (auto-selects OpenAI when key is present)
make dev
```

### Google Vertex AI (Enterprise)

Vertex AI uses **IAM identity auth** (ADC or service account), not a static API key. Do not confuse with the Gemini Developer API (`GEMINI_API_KEY`).

```bash
gcloud auth application-default login
export GOOGLE_CLOUD_PROJECT=your-gcp-project
export GOOGLE_CLOUD_REGION=europe-west1   # optional

# Use bundled catalog if ~/.edgequake/models.toml omits vertexai:
export EDGEQUAKE_MODELS_CONFIG=edgequake/models.toml

make dev
```

Verify in Settings → Provider Status Hub: Vertex AI should show **Identity (ADC)** when configured. See [Configuration — Vertex AI](/docs/operations/configuration#google-vertex-ai-enterprise).

### Provider Switching at Runtime

Once running, you can switch providers via API:

```bash
# Check current effective LLM configuration
curl http://localhost:8080/api/v1/config/effective | jq '.llm'

# Provider is auto-selected based on OPENAI_API_KEY
```

---

## Storage Configuration

EdgeQuake uses PostgreSQL as its storage backend for all modes (since v0.4.0):

```
┌─────────────────────────────────────────────────────────────┐
│                     Storage (PostgreSQL)                    │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│         ┌─────────────────────────────────────┐            │
│         │          PostgreSQL 15+              │            │
│         │                                     │            │
│         │  ┌──────────┐  ┌──────────────────┐ │            │
│         │  │ pgvector  │  │   Apache AGE     │ │            │
│         │  │ (vectors) │  │   (graph DB)     │ │            │
│         │  └──────────┘  └──────────────────┘ │            │
│         │                                     │            │
│         └─────────────────────────────────────┘            │
│                                                             │
│  DATABASE_URL required for all server modes.               │
│  Use Docker for the easiest PostgreSQL setup.              │
└─────────────────────────────────────────────────────────────┘
```

### PostgreSQL Setup

```bash
# Using Docker (recommended)
docker run -d \
  --name edgequake-postgres \
  -e POSTGRES_PASSWORD=edgequake \
  -e POSTGRES_DB=edgequake \
  -p 5432:5432 \
  ghcr.io/raphaelmansuy/edgequake-postgres:latest

# Set connection string
export DATABASE_URL="postgresql://edgequake:edgequake_secret@localhost:5432/edgequake"

# Run migrations
cd edgequake && sqlx database setup
```

---

## Verification Checklist

Run these commands to verify your installation:

```bash
# 1. Confirm the pinned compiler is active
cd edgequake
cargo --version
rustc --version

# 2. Check backend health
curl -s http://localhost:8080/health | jq
# ✅ Expected: JSON containing "status":"healthy" and "storage_mode":"postgresql"

# 3. Check API docs
curl -s http://localhost:8080/api-docs/openapi.json | jq .info.title
# ✅ Expected: "EdgeQuake API"

# 4. Check Ollama if using the local provider
curl -s http://localhost:11434/api/tags | jq

# 5. Let the repo verify itself
cargo fmt --all --check
cargo clippy --workspace --lib -- -D warnings
cargo test --workspace --lib --no-fail-fast
```

### No-flake local workflow

For the most reproducible local setup:

```bash
cd edgequake
rustup show active-toolchain
make status
```

If PostgreSQL is unavailable, EdgeQuake now exits with a clear startup error instead of crashing later in a harder-to-debug state.

---

## Troubleshooting

### Docker Issues

```bash
# Problem: Docker not running
docker info
# Solution: Start Docker Desktop or systemctl start docker

# Problem: Port 5432 in use
lsof -i :5432
# Solution: Stop conflicting service or use different port
```

### Rust Build Issues

```bash
# Problem: Rust version too old
rustup update stable

# Problem: Missing dependencies on Linux
sudo apt-get install pkg-config libssl-dev libpq-dev

# Problem: Slow compilation
# Solution: Use faster linker
# In .cargo/config.toml:
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=lld"]
```

### LLM Issues

```bash
# Problem: Ollama not responding
ollama serve  # Start if not running
ollama list   # Check available models

# Problem: OpenAI rate limit
# Solution: Check your API usage at platform.openai.com
```

---

## Next Steps

Now that EdgeQuake is running:

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
