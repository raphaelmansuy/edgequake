# EdgeQuake

<a href="https://trendshift.io/repositories/20893" target="_blank"><img src="https://trendshift.io/api/badge/repositories/20893" alt="raphaelmansuy%2Fedgequake | Trendshift" style="width: 250px; height: 55px;" width="250" height="55"/></a>

> **High-Performance Graph-RAG Framework in Rust**  
> Transform documents into intelligent knowledge graphs for superior retrieval and generation

[![Version](https://img.shields.io/badge/version-0.26.5-blue.svg?style=flat)](CHANGELOG.md)
[![Rust](https://img.shields.io/badge/rust-1.95+-orange.svg?style=flat&logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg?style=flat)](LICENSE)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg?style=flat)](https://github.com/raphaelmansuy/edgequake)
[![Documentation](https://img.shields.io/badge/docs-available-blue.svg?style=flat)](docs/README.md)

![Screenshot of EdgeQuake Frontend](docs/assets/01-screenshot.png)

---

## Quick Start

> **No Rust, no Node.js, no build.** Just Docker.

```bash
curl -fsSL https://raw.githubusercontent.com/raphaelmansuy/edgequake/edgequake-main/quickstart.sh | sh
```

The wizard guides you through provider selection (OpenAI / Ollama), model choice, and starts the full stack.  
**Open** http://localhost:3000 **and you're in** — no login required (quickstart runs with open API via `EDGEQUAKE_DEV_MODE=true`).

> **Ports:** Docker quickstart maps the Web UI to **http://localhost:3000**. Local `make dev` defaults to **http://localhost:3010** (avoids collisions with other stacks).

<details>
<summary><strong>Alternative: docker compose directly</strong></summary>

```bash
curl -fsSL https://raw.githubusercontent.com/raphaelmansuy/edgequake/edgequake-main/docker-compose.quickstart.yml \
  -o docker-compose.quickstart.yml
docker compose -f docker-compose.quickstart.yml up -d
```

**Headless / CI (no interactive terminal):**

```bash
# OpenAI
EDGEQUAKE_LLM_PROVIDER=openai \
  OPENAI_API_KEY=sk-... \
  docker compose -f docker-compose.quickstart.yml up -d

# Ollama (on host)
EDGEQUAKE_LLM_PROVIDER=ollama \
  EDGEQUAKE_LLM_MODEL=gemma4:e4b \
  EDGEQUAKE_EMBEDDING_PROVIDER=ollama \
  OLLAMA_EMBEDDING_MODEL=embeddinggemma \
  docker compose -f docker-compose.quickstart.yml up -d
```

</details>

| Service | URL (Docker quickstart) | URL (`make dev`) |
|---------|-------------------------|------------------|
| Web UI | http://localhost:3000 | http://localhost:3010 |
| REST API | http://localhost:8080 | http://localhost:8090* |
| Swagger | http://localhost:8080/swagger-ui | http://localhost:8090/swagger-ui* |
| Health | http://localhost:8080/health | http://localhost:8090/health* |

\*Local `make dev` picks free ports starting at **8090** (API) / **3010** (UI); see `make status` for the bound ports.
**Verify:**

```bash
curl -s http://localhost:8080/health | python3 -m json.tool
```

> Pin a version: `EDGEQUAKE_VERSION=0.26.5 sh quickstart.sh`

### What's new in 0.26.4

Patch: SPEC-144 Next.js **16.3.3** Active LTS (August Critical RCEs) + proxy SSOT; SPEC-140/141 list completeness; SPEC-122 bulk-ingest honesty; health poll off by default; distroless API. **No new migration** (schema stays **149**). Pull GHCR `0.26.4` for the patched frontend image.

Upgrade: **[upgrade-to-0.26.4.md](docs/operations/upgrade-to-0.26.4.md)** · changelog: [CHANGELOG.md](CHANGELOG.md).

### What's new in 0.26.3

Patch: SPEC-139 mid-cutover engine (iw2 21000, W3 coverage-sum, KV remainder after 119-before-122); Langfuse 3.22/3.225 isolated OTLP stacks. **No new migration** (schema stays **149**). Pull GHCR `0.26.3` — do not stay on `0.26.1` for leftover DROP OLD copy.

Upgrade: **[upgrade-to-0.26.3.md](docs/operations/upgrade-to-0.26.3.md)** · changelog: [CHANGELOG.md](CHANGELOG.md).

### What's new in 0.26.2

Patch: Langfuse 3.1.x ingestion fallback (SPEC-124), Kubernetes Helm/kind (SPEC-138), SSE/conversation restore, workspace `include_stats`. **No new migration** (schema stays **149**). Pull GHCR `0.26.2`.

Upgrade: **[upgrade-to-0.26.2.md](docs/operations/upgrade-to-0.26.2.md)** · changelog: [CHANGELOG.md](CHANGELOG.md).

### What's new in 0.26.1

Patch: SPEC-137 migrate honesty (`--drop-confirm` alias, unknown apply flags fail-closed, classified DROP abort hints). **No new migration** (schema stays **149**). Pull GHCR `0.26.1` — the `0.26.0` image still has the old CLI.

Upgrade: **[upgrade-to-0.26.1.md](docs/operations/upgrade-to-0.26.1.md)** · leftover 091: [upgrade-to-0.26.0.md](docs/operations/upgrade-to-0.26.0.md) · changelog: [CHANGELOG.md](CHANGELOG.md).

### What's new in 0.26.0

Minor: PDF pack-to-budget (SPEC-135), manuscript page-as-unit convert (SPEC-134), Langfuse dev sibling (SPEC-124), wizard persist honesty (SPEC-101), and reliability (#377, #383–#386 + mig **149**). Crates.io: `edgequake-llm` **0.10.8**, `edgequake-pdf2md` **0.9.11**, `edgequake-sdk` **0.4.0**.

Upgrade: **[upgrade-to-0.26.0.md](docs/operations/upgrade-to-0.26.0.md)** · changelog: [CHANGELOG.md](CHANGELOG.md).

### What's new in 0.25.0

Minor: Langfuse OTLP/HTTP (SPEC-124), structure-aware markdown pack (SPEC-125), provider KV / prompt cache (SPEC-126), PDF layout overlay (SPEC-128 + mig **148**), omit-temperature / Responses API (SPEC-131 / #379), multi-PDF admit honesty (#378), document status CHECK SSOT (#381), fleet-mirror UUID + target-`->` parse (#380 / SPEC-133). Crates.io deps: `edgequake-llm` **0.10.8**, `edgequake-pdf2md` **0.9.11**.

Upgrade: **[upgrade-to-0.25.0.md](docs/operations/upgrade-to-0.25.0.md)** · changelog: [CHANGELOG.md](CHANGELOG.md).

### What's new in 0.24.0

#### Database migration (read this first)

**The API never migrates the database.** Schema changes are an explicit operator step.

| Situation | What to run |
|-----------|-------------|
| **Fresh install** | `edgequake migrate` once, then start the API (`make dev` does this for you) |
| **Upgrade from ≤ v0.22.0** | Backup → `migrate dry-run` → `migrate` → `migrate --confirm-drop` → `migrate` (applies deferred **142**) → start API |
| **Server exits 78** | Schema behind or newer than the binary — run migrate, then restart |

Irreversible drops (**125** KV, **126**/**131** vectors) need `--confirm-drop` and a backup; rollback after that is restore-only. Migration **142** asserts empty leftovers (aborts if rows remain; deferred while residue exists).

Full plain-language guide: **[Migrate to v0.23.0+](docs/operations/migrate-to-0.23.md)** · production soak: [SPEC-091 upgrade runbook](docs/operations/spec091-upgrade-from-v0.22.0.md).

#### Highlights

- **SPEC-104 production data-layer monitors** — StorageInspector uses `workspace_id` + `PostgresConfig` AGE graph SSOT; no `42703` / `42P01` probes; INV-03 dual-read; tenant create **201/200/409**.
- **SPEC-105 legacy cutover assert** — census SSOT; unknown `VECTOR_BACKEND` → typed; migration **142**; mid-upgrade deferral so expandables soft-exit while residue remains.
- **Schema** — migrations through **142** (0.23.0 stopped at **141**).

Also in **0.23.0**: SPEC-091 relational cutover (106–141), LD-15 boot gate, SPEC-094 parse API, SPEC-103 LLM cache, wizard/UX. **0.22.0**: SPEC-090 multi-pool + migrate CLI.
### Performance testing

Publish Acc is **medical-mid n=200** (`make bench`) — not smoke n=40. Latest publish pack (`medical-mid-20260815T110218Z`): Acc EQ **0.792** vs LR **0.786** (Δ Acc 95% CI **[-0.022, +0.034]** — statistical tie; L2 promote gates unmet — do **not** claim EQ beats LightRAG / Acc Beat). Fair cold latency ratio **1.02×** (`C1COLD_v1`). Smoke peers remain CI/ablation references only. Required local pre-tag gate: see [Release & CD § SPEC-001](docs/operations/release-and-cd.md#spec-001-lightrag-acc-before-tag).

- [EQ vs LightRAG Acc Bench](docs/comparisons/eq-vs-lightrag-acc-bench.md) — measured scorecard
- [BUSINESS_REPORT.md](specs/001-benchmark/e2e/artifacts/publish/latest/BUSINESS_REPORT.md) — regenerated by `make bench` (n=200)
- [EXEC_SUMMARY.txt](specs/001-benchmark/e2e/artifacts/publish/latest/EXEC_SUMMARY.txt)
- [peers.json](specs/001-benchmark/e2e/artifacts/peers.json)
- Protocol: [000-index](specs/001-benchmark/000-index.md) · [001-first-principles](specs/001-benchmark/001-first-principles.md)
- Context: [055](specs/001-benchmark/001-edgquake-improvements/055-post-acc-ceiling-first-principles.md) · [063](specs/001-benchmark/001-edgquake-improvements/063-why-lightrag-faster-cache-fairness.md)

### Authentication (v0.15+)

From **v0.15**, the API enables authentication **secure by default** (SPEC-027). Identity lives in PostgreSQL; login requires at least one user with a real password hash.

| Scenario | What to set |
|----------|-------------|
| **Quickstart / demo** | Nothing — compose defaults to `EDGEQUAKE_DEV_MODE=true` (open API, no login) |
| **Production with login** | Bootstrap admin **before first API start** (see below) |
| **Local dev from source** | `make dev` (auth off) or `make dev-auth` (auth on + demo login hidden) |

**Enable login on Docker / production:**

```bash
export EDGEQUAKE_DEV_MODE=false
export EDGEQUAKE_AUTH_ENABLED=true
export EDGEQUAKE_BOOTSTRAP_ADMIN_USERNAME=admin
export EDGEQUAKE_BOOTSTRAP_ADMIN_PASSWORD='ChangeMe123!'   # min 8 chars, mixed complexity
export EDGEQUAKE_BOOTSTRAP_ADMIN_EMAIL=admin@example.com   # optional
export NEXT_PUBLIC_AUTH_ENABLED=true
export NEXT_PUBLIC_DISABLE_DEMO_LOGIN=true
docker compose -f docker-compose.quickstart.yml up -d
```

The API creates the bootstrap admin on startup. Sign in at http://localhost:3000/login (Docker quickstart) or http://localhost:3010/login (`make dev`).

Upgrades from pre-v0.15: legacy KV `auth:user:*` records are imported into PostgreSQL automatically when present.

See [Runtime Auth Hardening](docs/operations/runtime-auth-hardening.md) for master API keys, OIDC, and troubleshooting ([GitHub #288](https://github.com/raphaelmansuy/edgequake/issues/288)).

### Ingestion cancel & restart (v0.19+)

From **v0.19**, cancel and restart are durable (SPEC-057): UI shows **Stopping…** until terminal **Cancelled** (not Failed); Pending tasks survive process restart via Postgres claim/lease (`FOR UPDATE SKIP LOCKED`). See [Ingestion cancel and fairness](docs/ingestion-cancel-and-fairness.md).

---

## First Steps

**Upload a document** (PDF, TXT, MD):

```bash
curl -X POST http://localhost:8080/api/v1/documents/upload \
  -F "file=@your-document.pdf"
```

Or drag-and-drop in the Web UI at http://localhost:3000 (Docker) or http://localhost:3010 (`make dev`).

**Query the knowledge graph:**

```bash
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{"query": "What are the main concepts?", "mode": "hybrid"}'
```

---

## Why EdgeQuake?

Traditional RAG retrieves document chunks by vector similarity alone. This works for keyword lookups but fails on multi-hop reasoning, thematic questions, and relationship queries. **Vectors capture similarity but lose structural relationships.**

EdgeQuake implements the [LightRAG algorithm](https://arxiv.org/abs/2410.05779) in Rust: documents are decomposed into a **knowledge graph** of entities and relationships. At query time, the system traverses both the vector space and the graph structure — combining the speed of embeddings with the reasoning power of graph traversal.

| Metric | EdgeQuake | Traditional RAG | Improvement |
|--------|-----------|----------------|-------------|
| Query Latency (hybrid) | < 200ms | ~1000ms | 5x faster |
| Entity Extraction | ~2-3x more | Baseline | 3x |
| Concurrent Users | 1000+ | ~100 | 10x |
| Memory per Document | 2MB | ~8MB | 4x |

---

## Features

### Knowledge Graph

- **Entity Extraction** — LLM-powered detection of people, organizations, locations, concepts, technologies, and products
- **Relationship Mapping** — Automatic identification of connections with keyword tagging
- **Multi-Pass Gleaning** — Second-pass extraction catches 15-25% more entities
- **Community Detection** — Louvain clustering groups related entities for thematic queries
- **Custom Entity Types** — 5 domain presets (General, Manufacturing, Healthcare, Legal, Research), up to 50 types per workspace
- **Knowledge Injection** — Domain glossaries, acronym definitions, and synonym mappings

### Query Engine — 6 Modes

| Mode | Best For | Latency |
|------|----------|---------|
| **Naive** | Keyword-like lookups | ~100-300ms |
| **Local** | Specific entity relationships | ~200-500ms |
| **Global** | Thematic / high-level questions | ~300-800ms |
| **Hybrid** *(default)* | Balanced, comprehensive results | ~400-1000ms |
| **Mix** | Weighted vector + graph blend | configurable |
| **Bypass** | Direct LLM (no RAG) | LLM-dependent |

### Hybrid RAG (v0.16 / SPEC-046)

Production Hybrid RAG with **fail-closed ops** and science-grade retrieval defaults:

- **PPR-default graph walk** — Personalized PageRank expands entity neighborhoods (`EDGEQUAKE_GRAPH_WALK=bfs` escape hatch)
- **Bipartite dual-node pick** — entity∪chunk adjacency for Local / Global / Mix chunk selection
- **HNSW fail-closed + `/ready`** — missing ANN index blocks traffic instead of silent degradation
- **Intent-gated Mix/Hybrid arms** — skip irrelevant retrieval arms; GenAI `rag.retrieval` spans
- **Failed-chunk retry → merge** — persist / list / retry extraction failures into the knowledge graph
- **Faithfulness sampling** — heuristic + optional LLM judge (`EDGEQUAKE_FAITHFULNESS_JUDGE`)
- **ACC CI gate** — `make spec046-acc` writes a deterministic AccReport JSON (no API key)

Ops runbooks: [specs/046-graphrag-study/13-OPS-RUNBOOKS.md](specs/046-graphrag-study/13-OPS-RUNBOOKS.md).

### PDF Vision Pipeline

- **Text Mode** — Fast pdfium-based extraction (default, zero-config, embedded in binary)
- **Vision Mode** — GPT-4o, Claude, Gemini read each page as an image
- **Table Reconstruction** — Recovers complex tables that text parsers mangle
- **Multi-Column Layout** — LLM understands reading order across columns
- **Automatic Fallback** — Vision failures gracefully fall back to text extraction

### Production Ready

- **REST API** — OpenAPI 3.0, SSE streaming, batch ingestion, health checks
- **Multi-Tenant** — Fail-closed workspace isolation for query, delete, and recovery
- **Auth & Audit** — Built-in authentication, authorization, and compliance logging
- **PostgreSQL 16/17/18** — Triple-track support with pgvector + Apache AGE
- **Multi-Arch Docker** — `linux/amd64` + `linux/arm64`, published to GHCR on every release
- **MCP Integration** — Expose capabilities to AI agents via [Model Context Protocol](mcp/)
- **React 19 Frontend** — Real-time streaming, interactive Sigma.js graph visualization, drag-and-drop upload

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│  Frontend (React 19 + TypeScript)                                   │
│  Document Upload · Query Interface · Graph Visualization · Config   │
└──────────────────────────────┬──────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│  REST API (Axum)                                                    │
│  /api/v1/documents · /api/v1/query · /api/v1/graph                  │
│  OpenAPI 3.0 · SSE Streaming · Health Checks                        │
└──────────────────────────────┬──────────────────────────────────────┘
                               │
              ┌────────────────┴────────────────┐
              ▼                                 ▼
┌──────────────────────────┐   ┌────────────────────────────────────┐
│  LLM Providers           │   │  Storage                           │
│  OpenAI · Anthropic      │   │  PostgreSQL 16 / 17 / 18           │ 
│  Gemini · Mistral        │   │  ├─ pgvector (embeddings)          │
│  Ollama · LM Studio      │   │  └─ Apache AGE (knowledge graph)   │
│  xAI · Azure · VertexAI  │   │                                    │
└──────────────────────────┘   └────────────────────────────────────┘
```

**Data flow:** Document → Chunks → Entity Extraction → Knowledge Graph → Vector + Graph Storage  
**Query flow:** Question → Graph Traversal + Vector Search → LLM → Answer with Sources

EdgeQuake is built from **11 Rust crates**: `edgequake-core`, `edgequake-storage`, `edgequake-api`, `edgequake-pipeline`, `edgequake-query`, `edgequake-pdf`, `edgequake-auth`, `edgequake-audit`, `edgequake-tasks`, `edgequake-rate-limiter`, `edgequake-observability`. LLM providers are handled by the external [`edgequake-llm`](https://crates.io/crates/edgequake-llm) crate.

See [Architecture Overview](docs/architecture/overview.md) and [LightRAG Algorithm Deep Dive](docs/deep-dives/lightrag-algorithm.md).

---

## Docker Deployment

Three options depending on your setup:

<details>
<summary><strong>Option A — API Only</strong> (bring your own PostgreSQL)</summary>

```bash
docker run -d --name edgequake -p 8080:8080 \
  -e DATABASE_URL="postgres://user:pass@your-db:5432/edgequake" \
  -e EDGEQUAKE_LLM_PROVIDER=openai \
  -e OPENAI_API_KEY="sk-..." \
  ghcr.io/raphaelmansuy/edgequake:latest
```

</details>

<details>
<summary><strong>Option B — Full Stack with Prebuilt Images</strong> (recommended)</summary>

```bash
cd edgequake/docker
cp .env.example .env
docker compose -f docker-compose.prebuilt.yml up -d
```

| Service | Port | Image |
|---------|------|-------|
| API | 8080 | `ghcr.io/raphaelmansuy/edgequake:0.21.0` (`:latest`) |
| Frontend | 3000 | `ghcr.io/raphaelmansuy/edgequake-frontend:0.21.0` (`:latest`) |
| PostgreSQL | 5432 | `ghcr.io/raphaelmansuy/edgequake-postgres:0.21.0` (**PG18** default) |

**PostgreSQL major tags (multi-arch amd64 + arm64):**

| Tag | PostgreSQL |
|-----|------------|
| `0.21.0` / `latest` / `0.21.0-pg18` / `latest-pg18` | PG18 |
| `0.21.0-pg17` / `latest-pg17` | PG17 |
| `0.21.0-pg16` / `latest-pg16` | PG16 |

```bash
# Pin full stack to this release
EDGEQUAKE_VERSION=0.26.5 docker compose -f docker-compose.quickstart.yml up -d

# Pin PostgreSQL major (optional; default tag follows EDGEQUAKE_VERSION → PG18)
EDGEQUAKE_VERSION=0.26.5 EDGEQUAKE_POSTGRES_TAG=0.21.0-pg16 \
  docker compose -f docker-compose.quickstart.yml up -d
```

Also works with `latest-pg16` / `latest-pg17` / `latest-pg18`.

</details>

<details>
<summary><strong>Option C — Build from Source</strong></summary>

```bash
cd edgequake/docker && docker compose up -d
```

</details>

<details>
<summary><strong>Environment Variables</strong></summary>

| Variable | Default | Description |
|----------|---------|-------------|
| `EDGEQUAKE_LLM_PROVIDER` | `ollama` | `openai`, `anthropic`, `gemini`, `mistral`, `ollama`, `azure`, `vertexai` |
| `EDGEQUAKE_EMBEDDING_PROVIDER` | *(same as LLM)* | Separate embedding provider for hybrid mode |
| `EDGEQUAKE_MODELS_CONFIG` | — | Path to custom `models.toml` (see bundled catalog in repo) |
| `OPENAI_API_KEY` | — | Required for `openai` |
| `ANTHROPIC_API_KEY` | — | Required for `anthropic` |
| `GEMINI_API_KEY` | — | Required for **Gemini Developer API** (`gemini` provider) |
| `GOOGLE_CLOUD_PROJECT` | — | Required for **Vertex AI** (`vertexai` provider) |
| `GOOGLE_CLOUD_REGION` | `us-central1` | Vertex AI regional endpoint |
| `GOOGLE_APPLICATION_CREDENTIALS` | — | Service account JSON path (Vertex identity auth) |
| `MISTRAL_API_KEY` | — | Required for `mistral` |
| `OLLAMA_HOST` | `http://host.docker.internal:11434` | Ollama server URL |
| `EDGEQUAKE_VERSION` | `latest` | GHCR image tag |
| `EDGEQUAKE_DEV_MODE` | `true` (quickstart) | Open API without login — **do not use in production** |
| `EDGEQUAKE_AUTH_ENABLED` | `false` (quickstart) | Require JWT/API key on protected routes |
| `EDGEQUAKE_BOOTSTRAP_ADMIN_USERNAME` | `admin` | First-run admin username when auth is on |
| `EDGEQUAKE_BOOTSTRAP_ADMIN_PASSWORD` | — | First-run admin password (required for login on fresh installs) |
| `EDGEQUAKE_MASTER_API_KEY` | — | Bootstrap key for `POST /api/v1/users` without JWT |
| `NEXT_PUBLIC_AUTH_ENABLED` | `false` (quickstart) | Web UI login gate + session handling |
| `NEXT_PUBLIC_DISABLE_DEMO_LOGIN` | `false` | Hide “Continue without login” on the login page |
| `EDGEQUAKE_CHUNK_TIMEOUT_SECS` | `180` | Per-chunk LLM timeout (seconds) |
| `EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS` | `16` | Max parallel LLM calls |
| `RUST_LOG` | `info` | Log level |

**Vertex AI vs Gemini:** `gemini` uses a static API key (`GEMINI_API_KEY`). `vertexai` uses **OAuth2 identity** (ADC or service account) — not an API key. Local setup:

```bash
gcloud auth application-default login   # not: gcloud auth login application-default
export GOOGLE_CLOUD_PROJECT=your-gcp-project
export GOOGLE_CLOUD_REGION=europe-west1   # optional; default us-central1
```

If `~/.edgequake/models.toml` omits `vertexai`, point at the bundled catalog: `export EDGEQUAKE_MODELS_CONFIG=edgequake/models.toml`. See [Configuration — Vertex AI](docs/operations/configuration.md#google-vertex-ai-enterprise).

</details>

---

## SDKs

| Language | Link |
|----------|------|
| Python | [sdks/python/](sdks/python/README.md) |
| TypeScript | [sdks/typescript/](sdks/typescript/README.md) |
| Rust | [sdks/rust/](sdks/rust/README.md) |
| Go, Java, Kotlin, C#, PHP, Ruby, Swift | [sdks/](sdks/) |

---

## Development

> For contributors building from source. Most users should use the [Quick Start](#quick-start) above.

```bash
git clone https://github.com/raphaelmansuy/edgequake.git && cd edgequake
make install
cp edgequake_webui/.env.local.example edgequake_webui/.env.local
make dev                        # Start full stack (PostgreSQL + Backend + Frontend)
# Web UI defaults to http://localhost:3010 — confirm with: make status
```

```bash
cd edgequake && cargo test --workspace --lib --locked   # Unit / lib suite
cargo clippy --workspace --lib --locked -- -D warnings
cargo fmt --all -- --check
cd .. && make status && make stop
```

### Pre-delivery checklist (v0.19+)

Run these **before** tagging a release. Prefer Makefile targets — they set required env vars.

```bash
# Fast local gates (mirrors CI first principles: fail cheap → compile once → proofs)
make ops17-smoke                # PG extension pin SSOT (pg16/17/18)
make spec046-acc                # SPEC-046 ACC + AccReport JSON
make release-gates              # fmt + workspace clippy + SPEC-006/018 + WebUI + version parity
make test-e2e-lint              # Playwright flake anti-patterns
# SPEC-001 LightRAG Acc (local mandatory before tag — not in CI / release_gates.sh):
make bench001-doctor
make bench                      # EQ vs LightRAG Acc n=200 + publish/latest
# Optional UI-only (no backend): make test-e2e-ui
```

| Gate | What it proves | CI workflow |
|------|----------------|-------------|
| Migration checksum | Immutable SQL lockfile | `CI` → migration-checksum-guard |
| fmt + clippy + lib tests | Code quality | `CI` → check / test (nextest) |
| SPEC-006 / SPEC-018 | Resource + observability proofs | `CI` + `Release Gates` |
| Invariants + test floor | Reliability floor (≥870 lib) | `Test Quality Gates` |
| SPEC-046 ACC | Hybrid RAG science ACC | `SPEC-046 ACC` |
| SPEC-001 LightRAG Acc | EQ vs LightRAG GraphRAG-Bench Acc (n=200) | **Local only** (`make bench`) |
| OPS-17 pins | pgvector/AGE pin matrix | `PostgreSQL Matrix Nightly` |

**CI speed principles** (see `.github/workflows/ci.yml`): shared cargo cache across jobs, `CARGO_INCREMENTAL=0` + sparse index, `--locked`, cancel-in-progress, no duplicate workspace lib suite in sibling workflows, release gates skip per-crate clippy / lib re-run when CI already owns them.

See [AGENTS.md](AGENTS.md) for the full developer workflow and [Release & CD](docs/operations/release-and-cd.md) for the release process.

---

## Documentation

| Category | Links |
|----------|-------|
| Getting Started | [Installation](docs/getting-started/installation.md) · [Quick Start](docs/getting-started/quick-start.md) |
| Tutorials | [First RAG App](docs/tutorials/first-rag-app.md) · [PDF Ingestion](docs/tutorials/pdf-ingestion.md) · [Multi-Tenant](docs/tutorials/multi-tenant.md) |
| Architecture | [Overview](docs/architecture/overview.md) · [Data Flow](docs/architecture/data-flow.md) · [Crate Reference](docs/architecture/crates/) |
| Deep Dives | [LightRAG Algorithm](docs/deep-dives/lightrag-algorithm.md) · [Query Modes](docs/deep-dives/query-modes.md) · [PDF Processing](docs/deep-dives/pdf-processing.md) |
| Operations | [Deployment](docs/operations/deployment.md) · [Configuration](docs/operations/configuration.md) · [Runtime Auth](docs/operations/runtime-auth-hardening.md) · [Monitoring](docs/operations/monitoring.md) |
| API Reference | [REST API](docs/api-reference/rest-api.md) · [Extended API](docs/api-reference/extended-api.md) |
| Integrations | [MCP Server](mcp/) · [OpenWebUI](docs/integrations/open-webui.md) · [LangChain](docs/integrations/langchain.md) |
| Release & CD | [Release Cycle](docs/operations/release-and-cd.md) · [CHANGELOG](CHANGELOG.md) |

Full index: [docs/README.md](docs/README.md)

---

## Contributing

EdgeQuake uses a **Specification-Driven Development** approach. See [CONTRIBUTING.md](CONTRIBUTING.md).

- [GitHub Issues](https://github.com/raphaelmansuy/edgequake/issues) — Bug reports and feature requests
- [GitHub Discussions](https://github.com/raphaelmansuy/edgequake/discussions) — Questions and community help

---

## Acknowledgments

EdgeQuake implements the [LightRAG algorithm](https://arxiv.org/abs/2410.05779) by Zirui Guo, Lianghao Xia, Yanhua Yu, Tu Ao, and Chao Huang. Also inspired by Microsoft's [GraphRAG](https://arxiv.org/abs/2404.16130).

## License

Apache License, Version 2.0 — see [LICENSE](LICENSE).  
**Copyright 2024-2026 Raphaël MANSUY**

---

## Star History

[![Star History Chart](https://star-history.dera.page/svg?repos=raphaelmansuy/edgequake&type=date&legend=top-left)](https://star-history.dera.page/#raphaelmansuy/edgequake&type=date&legend=top-left)
