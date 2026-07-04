# EdgeQuake

<a href="https://trendshift.io/repositories/20893" target="_blank"><img src="https://trendshift.io/api/badge/repositories/20893" alt="raphaelmansuy%2Fedgequake | Trendshift" style="width: 250px; height: 55px;" width="250" height="55"/></a>

> **High-Performance Graph-RAG Framework in Rust**  
> Transform documents into intelligent knowledge graphs for superior retrieval and generation

[![Version](https://img.shields.io/badge/version-0.14.1-blue.svg?style=flat)](CHANGELOG.md)
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
**Open** http://localhost:3000 **and you're in.**

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

| Service | URL |
|---------|-----|
| Web UI | http://localhost:3000 |
| REST API | http://localhost:8080 |
| Swagger | http://localhost:8080/swagger-ui |
| Health | http://localhost:8080/health |

**Verify:**

```bash
curl -s http://localhost:8080/health | python3 -m json.tool
```

> Pin a version: `EDGEQUAKE_VERSION=0.14.0 sh quickstart.sh`

---

## First Steps

**Upload a document** (PDF, TXT, MD):

```bash
curl -X POST http://localhost:8080/api/v1/documents/upload \
  -F "file=@your-document.pdf"
```

Or drag-and-drop in the Web UI at http://localhost:3000.

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
| API | 8080 | `ghcr.io/raphaelmansuy/edgequake:latest` |
| Frontend | 3000 | `ghcr.io/raphaelmansuy/edgequake-frontend:latest` |
| PostgreSQL | 5432 | `ghcr.io/raphaelmansuy/edgequake-postgres:latest` (PG18) |

Pin a PostgreSQL tier: `EDGEQUAKE_POSTGRES_TAG=latest-pg16` or `latest-pg17`.

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
| `OPENAI_API_KEY` | — | Required for `openai` |
| `ANTHROPIC_API_KEY` | — | Required for `anthropic` |
| `GEMINI_API_KEY` | — | Required for `gemini` |
| `MISTRAL_API_KEY` | — | Required for `mistral` |
| `OLLAMA_HOST` | `http://host.docker.internal:11434` | Ollama server URL |
| `EDGEQUAKE_VERSION` | `latest` | GHCR image tag |
| `EDGEQUAKE_CHUNK_TIMEOUT_SECS` | `180` | Per-chunk LLM timeout (seconds) |
| `EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS` | `16` | Max parallel LLM calls |
| `RUST_LOG` | `info` | Log level |

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
```

```bash
cargo test                      # Run tests
cargo clippy && cargo fmt       # Lint and format
make status                     # Check service health
make stop                       # Stop all services
```

See [AGENTS.md](AGENTS.md) for the full developer workflow and [Release & CD](docs/operations/release-and-cd.md) for the release process.

---

## Documentation

| Category | Links |
|----------|-------|
| Getting Started | [Installation](docs/getting-started/installation.md) · [Quick Start](docs/getting-started/quick-start.md) |
| Tutorials | [First RAG App](docs/tutorials/first-rag-app.md) · [PDF Ingestion](docs/tutorials/pdf-ingestion.md) · [Multi-Tenant](docs/tutorials/multi-tenant.md) |
| Architecture | [Overview](docs/architecture/overview.md) · [Data Flow](docs/architecture/data-flow.md) · [Crate Reference](docs/architecture/crates/) |
| Deep Dives | [LightRAG Algorithm](docs/deep-dives/lightrag-algorithm.md) · [Query Modes](docs/deep-dives/query-modes.md) · [PDF Processing](docs/deep-dives/pdf-processing.md) |
| Operations | [Deployment](docs/operations/deployment.md) · [Configuration](docs/operations/configuration.md) · [Monitoring](docs/operations/monitoring.md) |
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

[![Star History Chart](https://api.star-history.com/svg?repos=raphaelmansuy/edgequake&type=date&legend=top-left)](https://www.star-history.com/#raphaelmansuy/edgequake&type=date&legend=top-left)
