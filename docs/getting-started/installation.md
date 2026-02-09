# Installation Guide

> Get EdgeQuake running on your machine in 5 minutes

---

## Prerequisites Checklist

Before installing, ensure you have:

| Requirement | Version | Check Command      | Purpose               |
| ----------- | ------- | ------------------ | --------------------- |
| **Rust**    | 1.78+   | `rustc --version`  | Build backend         |
| **Cargo**   | Latest  | `cargo --version`  | Package manager       |
| **Docker**  | 24+     | `docker --version` | PostgreSQL (optional) |
| **Node.js** | 20+     | `node --version`   | WebUI (optional)      |
| **pnpm**    | 8+      | `pnpm --version`   | Package manager       |

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

1. Starts PostgreSQL in Docker (port 5432)
2. Runs database migrations
3. Builds and starts Rust backend (port 8080)
4. Starts Next.js frontend (port 3000)

**Verify**:

```bash
# In a new terminal
curl http://localhost:8080/health
# Expected: {"status":"ok","version":"..."}

# Open WebUI
open http://localhost:3000
```

---

### Option 2: Backend Only (For API Development)

```bash
# Clone and enter
git clone https://github.com/raphaelmansuy/edgequake.git
cd edgequake

# Start backend with PostgreSQL
make backend-bg

# Or: Start backend with in-memory storage (no persistence)
make backend-memory
```

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

# Run directly
./target/release/edgequake
```

---

### Option 4: Development Mode (Watch + Hot Reload)

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

# Pull models
ollama pull llama3.2
ollama pull nomic-embed-text

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

### Provider Switching at Runtime

Once running, you can switch providers via API:

```bash
# Check current provider
curl http://localhost:8080/api/v1/config | jq .llm_provider

# Provider is auto-selected based on OPENAI_API_KEY
```

---

## Storage Configuration

```
┌─────────────────────────────────────────────────────────────┐
│                     Storage Options                         │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────────────┐          ┌──────────────────┐         │
│  │   PostgreSQL     │          │    In-Memory     │         │
│  │  + pgvector      │          │                  │         │
│  │  + Apache AGE    │          │   (No Docker)    │         │
│  │                  │          │   (No persist)   │         │
│  │  [Production]    │          │   [Development]  │         │
│  └──────────────────┘          └──────────────────┘         │
│                                                             │
│  DATABASE_URL set?  ────▶ Yes: Use PostgreSQL               │
│                     ────▶ No:  Use In-Memory                │
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
export DATABASE_URL="postgresql://postgres:edgequake@localhost:5432/edgequake"

# Run migrations
cd edgequake && sqlx database setup
```

---

## Verification Checklist

Run these commands to verify your installation:

```bash
# 1. Check backend health
curl -s http://localhost:8080/health | jq
# ✅ Expected: {"status":"ok","version":"0.1.0",...}

# 2. Check API docs
curl -s http://localhost:8080/api-docs/openapi.json | jq .info.title
# ✅ Expected: "EdgeQuake API"

# 3. Check LLM provider
curl -s http://localhost:8080/api/v1/config | jq .llm_provider
# ✅ Expected: "ollama" or "openai"

# 4. Test document ingestion
curl -X POST http://localhost:8080/api/v1/documents \
  -H "Content-Type: application/json" \
  -d '{"content":"Marie Curie discovered radium in 1898.","title":"Test"}'
# ✅ Expected: {"document_id":"...","entities_extracted":...}

# 5. Test query
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{"query":"Who discovered radium?"}'
# ✅ Expected: {"response":"Marie Curie...","sources":[...]}
```

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

1. **[Quick Start](quick-start.md)** — Ingest your first document
2. **[Architecture Overview](../architecture/overview.md)** — Understand the system
3. **[API Reference](../api-reference/rest-api.md)** — Explore endpoints

---

## System Requirements

| Component | Minimum                      | Recommended  |
| --------- | ---------------------------- | ------------ |
| **RAM**   | 4 GB                         | 16 GB        |
| **CPU**   | 2 cores                      | 8 cores      |
| **Disk**  | 10 GB                        | 50 GB        |
| **OS**    | Linux, macOS, Windows (WSL2) | Linux, macOS |
