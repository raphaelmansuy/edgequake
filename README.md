# EdgeQuake

<a href="https://trendshift.io/repositories/20893" target="_blank"><img src="https://trendshift.io/api/badge/repositories/20893" alt="raphaelmansuy%2Fedgequake | Trendshift" style="width: 250px; height: 55px;" width="250" height="55"/></a>

> **High-Performance Graph-RAG Framework in Rust**  
> Transform documents into intelligent knowledge graphs for superior retrieval and generation

[![Version](https://img.shields.io/badge/version-0.11.0-blue.svg?style=flat)](CHANGELOG.md)
[![Rust](https://img.shields.io/badge/rust-1.95+-orange.svg?style=flat&logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg?style=flat)](LICENSE)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg?style=flat)](https://github.com/raphaelmansuy/edgequake)
[![Documentation](https://img.shields.io/badge/docs-available-blue.svg?style=flat)](docs/README.md)

> **v0.11.0** — Mistral La Plateforme is now a first-class citizen: chat (`mistral-small-latest`), vision PDF ingestion (`pixtral-large-latest`), and embeddings (`mistral-embed`, 1024 dims) all work out of the box. Set `MISTRAL_API_KEY` and `make dev` — no other config needed. See [CHANGELOG](CHANGELOG.md) for full details.

---

![Screenshot of EdgeQuake Frontend](docs/assets/01-screenshot.png)

## Why EdgeQuake?

Traditional RAG systems retrieve document chunks using vector similarity alone. This works for simple lookups but fails on multi-hop reasoning ("How does X relate to Y through Z?"), thematic questions ("What are the major themes?"), and relationship queries. The core problem: **vectors capture semantic similarity but lose structural relationships between concepts**.

**EdgeQuake** solves this by implementing the [LightRAG algorithm](https://arxiv.org/abs/2410.05779) in Rust: documents are not just chunked and embedded — they are decomposed into a **knowledge graph** of entities and relationships. At query time, the system traverses both the vector space and the graph structure, combining the speed of vector search with the reasoning power of graph traversal.

### What Sets EdgeQuake Apart

- **Knowledge Graphs**: LLM-powered entity extraction and relationship mapping create a structured understanding of your documents — not just keyword matching
- **6 Query Modes**: From fast naive vector search to graph-traversing hybrid queries, each mode optimizes for different question types
- **Rust Performance**: Async-first Tokio architecture with zero-copy operations — handles thousands of concurrent requests
- **PDF LLM Vision Pipeline ✅ NEW in 0.4.0**: Multimodal LLMs (GPT-4o, Claude, Gemini) read PDF pages as images — handles scanned documents, complex tables, and multi-column layouts out of the box
- **Production Ready**: OpenAPI 3.0 REST API, SSE streaming, health checks, fail-closed multi-tenant workspace isolation for query/delete flows, and workspace-scoped destructive/recovery operations
- **Modern Frontend**: React 19 with interactive Sigma.js graph visualizations

### Performance Benchmarks

| Metric                 | EdgeQuake        | Traditional RAG | Improvement |
| ---------------------- | ---------------- | --------------- | ----------- |
| Entity Extraction      | ~2-3x more       | Baseline        | 3x          |
| Query Latency (hybrid) | < 200ms          | ~1000ms         | 5x faster   |
| Document Processing    | 25s (10k tokens) | ~60s            | 2.4x faster |
| Concurrent Users       | 1000+            | ~100            | 10x         |
| Memory Usage (per doc) | 2MB              | ~8MB            | 4x better   |

> **v0.4.0 — PDF is now Production Ready**: The PDF pipeline ships with embedded pdfium (zero-config) and an opt-in LLM vision mode. Text-mode extraction works for all standard PDFs; enable `use_vision_llm = true` (or send `X-Use-Vision: true`) to route pages through your vision-capable LLM for scanned documents and complex layouts.

> **v0.4.0 Update**: PDF processing is now **production-ready** with embedded pdfium via `edgequake-pdf2md v0.4.1`. No external library setup required — just upload your PDFs!

---

## Features

### 🚀 High Performance

- **Async-First**: Tokio-based runtime for maximum concurrency
- **Zero-Copy**: Efficient memory management with Rust ownership
- **Parallel Processing**: Multi-threaded entity extraction and embeddings
- **Fast Storage**: PostgreSQL AGE for graph + pgvector for embeddings
- **SQL Pre-Filtering**: Metadata filters (tenant, workspace, document) pushed to SQL WHERE clauses with GIN + B-tree indexes — up to 90% fewer wasted vector scans at scale

### 💉 Knowledge Injection ✨ **NEW in 0.8.0** ([#131](https://github.com/raphaelmansuy/edgequake/issues/131))

- **Domain Glossaries**: Inject acronym definitions (OEE, NLP, ML) and synonym mappings that expand query terms automatically
- **Invisible Citations**: Injection entries enrich the knowledge graph but are **never shown as source citations** in query results
- **Full CRUD API**: `PUT`, `GET`, `PATCH`, `DELETE` per workspace — `POST /api/v1/workspaces/:id/injection/upload` for file-based injection
- **Text & File Input**: Accept plain-text definitions or upload `.txt`/`.md` glossary files
- **Status Polling**: Background processing with entity count tracking and `completed`/`failed`/`processing` status
- **Dedicated UI**: `/knowledge` page with list view, inline editing, delete confirmation, and entity count display
- **Query Expansion**: Automatically expands queries using injected synonyms before vector search

### 🏷️ Custom Entity Configuration ✨ **NEW in 0.9.0** ([#85](https://github.com/raphaelmansuy/edgequake/issues/85))

- **Domain Presets**: Choose from 6 curated presets — General, Manufacturing, Healthcare, Legal, Research, Finance
- **Up to 50 Entity Types**: Configure up to 50 domain-specific types per workspace (raised from 20)
- **Custom Types**: Add any UPPERCASE_UNDERSCORED entity type beyond the preset (e.g. `BEARING_TYPE`, `VIBRATION_ANOMALY`)
- **Per-Workspace Config**: Entity types are set at workspace creation and stored in workspace metadata
- **Auto-Normalization**: Input is trimmed, uppercased, and deduplicated before storage
- **Live UI Selector**: Collapsible entity-type section in workspace creation dialog with chip display and count badge
- **Backward Compatible**: Existing workspaces without custom config automatically use the 9 default general types

### Knowledge Graph

- **Entity Extraction**: Automatic detection of people, organizations, locations, concepts, events, technologies, and products (7 configurable types)
- **Relationship Mapping**: LLM-powered relationship identification with keyword tagging
- **Gleaning**: Multi-pass extraction catches 15-25% more entities than single-pass
- **Community Detection**: Louvain modularity optimization clusters related entities for thematic queries
- **Graph Visualization**: Interactive Sigma.js-powered frontend with zoom/pan

### 📄 PDF Processing (Production Ready in v0.4.0)

- **Text Mode**: Fast pdfium-based extraction for standard PDFs (default, zero-config)
- **Vision Mode** ✨: LLM reads each page as an image — GPT-4o, Claude 3.5+, Gemini 2.5 supported
- **Automatic Fallback**: Vision failures gracefully fall back to text extraction (BR1010)
- **Safe Large-PDF Guardrails**: Adaptive DPI/concurrency limits and early byte release reduce memory spikes and make local-model ingestion more reliable for large files
- **Restart-Safe Recovery**: Deleted or cancelled PDFs do not reappear from stale background jobs after a server restart
- **Fail-Closed Query Isolation**: Invalid or missing explicit workspace selectors are rejected instead of being silently remapped to defaults
- **Safer Dev Service Checks**: Health/status flows now rely on lightweight database port checks first, which is less disruptive when Docker/OrbStack is unavailable
- **Table Reconstruction**: Vision mode recovers complex tables that text parsers mangle
- **Multi-Column Layout**: LLM understands reading order across multi-column pages
- **Embedded pdfium**: No `PDFIUM_DYNAMIC_LIB_PATH` env var needed — binary ships inside the binary

### 🔍 6 Query Modes

1. **Naive**: Simple vector similarity — fastest for keyword-like lookups (~100-300ms)
2. **Local**: Entity-centric with local graph neighborhood — best for specific relationships (~200-500ms)
3. **Global**: Community-based semantic search — best for thematic/high-level questions (~300-800ms)
4. **Hybrid** _(default)_: Combines local + global for balanced, comprehensive results (~400-1000ms)
5. **Mix**: Weighted combination of naive + graph results with configurable ratios
6. **Bypass**: Direct LLM query without RAG retrieval — useful for general questions

### 🌐 REST API

- **OpenAPI 3.0**: Full Swagger documentation at `/swagger-ui`
- **Streaming**: Server-Sent Events (SSE) for real-time responses
- **Versioned**: `/api/v1/*` with backward compatibility
- **Health Checks**: Kubernetes-ready `/health`, `/ready`, `/live`
- **Safer Local Startup**: Make-based development uses the standard UI port 3000 when available and auto-selects the next free port only if another local stack is already using it
- **Runtime Auth Hardening** ✨: prebuilt WebUI images now consume runtime API/auth config, and protected dashboard routes fail closed when authentication is enabled

### 🎯 React 19 Frontend

- **Real-Time Streaming**: Token-by-token generation display
- **Graph Visualization**: Interactive network graph with zoom/pan
- **Document Upload**: Drag-and-drop with progress tracking
- **Configuration UI**: Visual PDF processing config builder

### 🔌 MCP (Model Context Protocol)

- **Agent Integration**: Expose EdgeQuake capabilities to AI agents via [MCP](https://modelcontextprotocol.io/)
- **Tool Discovery**: Agents can query, upload, and explore knowledge graphs programmatically
- **Interoperability**: Works with Claude, Cursor, and other MCP-compatible clients

See [mcp/](mcp/) for server implementation details.

---

## Quick Start

### ⚡ Option 1 — One Command (Docker, ~30s, no build required)

> **Zero prerequisites beyond Docker.**  
> No Rust, no Node.js, no `cargo build`, no `npm install`.

```bash
# Download and run the interactive setup wizard
curl -fsSL https://raw.githubusercontent.com/raphaelmansuy/edgequake/edgequake-main/quickstart.sh | sh
```

The wizard guides you through:
1. **Provider selection** — choose OpenAI or Ollama (never guessed from environment variables)
2. **Model selection** — pick your LLM + embedding model from a curated, priced menu
3. **Validation** — API key check (OpenAI) or Ollama ping + model-availability check
4. **Stack startup** — pulls images, starts services, and polls health for up to 90 seconds
5. **Re-run aware** — detects running/stopped containers and existing data volumes; offers "Update & Reconfigure" or safe "Fresh Start" (requires typing `DELETE`)

> For local Make-based development, the UI uses port 3000 by default and automatically moves to a safe free port if 3000 or 8080 are already occupied.

Or with `docker compose` directly (pipe to compose):

```bash
curl -fsSL https://raw.githubusercontent.com/raphaelmansuy/edgequake/edgequake-main/docker-compose.quickstart.yml \
  | docker compose -f - up -d
```

Or download the compose file first, then start:

```bash
curl -fsSL https://raw.githubusercontent.com/raphaelmansuy/edgequake/edgequake-main/docker-compose.quickstart.yml \
  -o docker-compose.quickstart.yml
docker compose -f docker-compose.quickstart.yml up -d
```

**Then open:** http://localhost:3000

| Service | URL                              |
| ------- | -------------------------------- |
| Web UI  | http://localhost:3000            |
| API     | http://localhost:8080            |
| Swagger | http://localhost:8080/swagger-ui |
| Health  | http://localhost:8080/health     |

**Headless / CI install (no interactive terminal):**

```bash
# OpenAI
EDGEQUAKE_LLM_PROVIDER=openai \
  OPENAI_API_KEY=sk-... \
  docker compose -f docker-compose.quickstart.yml up -d

# Mistral La Plateforme ✨ new in v0.11.0
MISTRAL_API_KEY=... \
  docker compose -f docker-compose.quickstart.yml up -d
```

**Management:**

```bash
docker compose -f docker-compose.quickstart.yml logs -f    # tail logs
docker compose -f docker-compose.quickstart.yml ps         # check status
docker compose -f docker-compose.quickstart.yml down       # stop
```

> **Pinned version:** `EDGEQUAKE_VERSION=0.10.8 sh quickstart.sh` to use a specific release.

> Production auth/runtime deployment guidance is available in [docs/operations/runtime-auth-hardening.md](docs/operations/runtime-auth-hardening.md).

---

### 🛠️ Option 2 — Development Setup (Rust toolchain required)

#### Prerequisites

- **Rust**: 1.95 or later ([Install Rust](https://rustup.rs))
- **Node.js**: 18+ or Bun 1.0+ ([Install Node](https://nodejs.org))
- **Docker**: For PostgreSQL ([Install Docker](https://www.docker.com/get-started))
- **Ollama**: For local LLM (optional, [Install Ollama](https://ollama.ai))

#### Installation (5 minutes)

```bash
# 1. Clone the repository
git clone https://github.com/raphaelmansuy/edgequake.git
cd edgequake

# 2. Install dependencies
make install

# 3. Configure the frontend environment
cp edgequake_webui/.env.local.example edgequake_webui/.env.local

# 4. Start the full stack in the default local mode (no authentication)
make dev

# Optional: start the same stack with authentication enabled
make dev-auth
```

**That's it!** 🎉

- **Backend**: http://localhost:8080
- **Frontend**: http://localhost:3000 by default, or the next free port if 3000 is busy
- **Swagger UI**: http://localhost:8080/swagger-ui
- **Provider**: Ollama or OpenAI depending on your environment
- **Auth**: disabled in make dev, enabled in make dev-auth

### First Document Upload

```bash
# Upload a file (PDF, TXT, MD, etc.)
curl -X POST http://localhost:8080/api/v1/documents/upload \
  -F "file=@your-document.pdf"
```

**Response**:

```json
{
  "id": "doc-123",
  "status": "completed",
  "chunk_count": 15,
  "entity_count": 12,
  "relationship_count": 8,
  "processing_time_ms": 2500
}
```

### First Query

```bash
# Query the knowledge graph
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{
    "query": "What are the main concepts?",
    "mode": "hybrid"
  }'
```

**Response**:

```json
{
  "answer": "The main concepts are: knowledge graphs, entity extraction, and hybrid retrieval...",
  "sources": [
    { "chunk_id": "chunk-1", "similarity": 0.92 },
    { "chunk_id": "chunk-5", "similarity": 0.87 }
  ],
  "entities": ["KNOWLEDGE_GRAPH", "ENTITY_EXTRACTION"],
  "relationships": [
    {
      "source": "KNOWLEDGE_GRAPH",
      "target": "ENTITY_EXTRACTION",
      "type": "ENABLES"
    }
  ]
}
```

---

## Architecture

```
┌────────────────────────────────────────────────────────────────────────────┐
│                              EdgeQuake System                              │
└────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│  Frontend (React 19 + TypeScript)                                           │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │  Document    │  │    Query     │  │    Graph     │  │   Settings   │     │
│  │   Upload     │  │  Interface   │  │ Visualization│  │   Config     │     │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘     │
│         │                 │                 │                 │             │
│         └─────────────────┴─────────────────┴─────────────────┘             │
│                                    │                                        │
│                                    ▼                                        │
│  ┌────────────────────────────────────────────────────────────────────┐     │
│  │                         REST API (Axum)                            │     │
│  │  /api/v1/documents  •  /api/v1/query  •  /api/v1/graph             │     │
│  │  OpenAPI 3.0 Spec  •  SSE Streaming  •  Health Checks              │     │
│  └────────────────────────────────────────────────────────────────────┘     │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  Backend (Rust - 11 Crates)                                                 │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │  edgequake-core          │  Orchestration & Pipeline                 │   │
│  │  edgequake-llm           │  OpenAI, Anthropic, MiniMax, Ollama, etc. │   │
│  │  edgequake-storage       │  PostgreSQL AGE, Memory adapters          │   │
│  │  edgequake-api           │  REST API server                          │   │
│  │  edgequake-pipeline      │  Document ingestion pipeline              │   │
│  │  edgequake-query         │  Query engine (6 modes)                   │   │
│  │  edgequake-pdf           │  PDF extraction (vision / edgeparse)      │   │
│  │  edgequake-auth          │  Authentication & authorization           │   │
│  │  edgequake-audit         │  Compliance & audit logging               │   │
│  │  edgequake-tasks         │  Background job processing                │   │
│  │  edgequake-rate-limiter  │  Rate limiting middleware                 │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                    │                                        │
│                    ┌───────────────┴───────────────┐                        │
│                    ▼                               ▼                        │
│  ┌─────────────────────────────┐   ┌──────────────────────────────────┐     │
│  │   LLM Providers             │   │   Storage Backends               │     │
│  │  • OpenAI (gpt-4.1-nano)    │   │  • PostgreSQL 15+ (AGE + vector) │     │
│  │  • Anthropic (Claude)       │   │  • In-Memory (dev/testing)       │     │
│  │  • Mistral (mistral-small)  │   │  • Graph: Property graph model   │     │
│  │  • MiniMax (MiniMax-M2.7)   │   │  • Vector: pgvector embeddings   │     │
│  │  • Ollama (gemma3:12b)      │   │                                  │     │
│  │  • LM Studio, xAI, Gemini   │   │                                  │     │
│  │  Auto-detection via env     │   │                                  │     │
│  └─────────────────────────────┘   └──────────────────────────────────┘     │
└─────────────────────────────────────────────────────────────────────────────┘

                    Data Flow: Document → Chunks → Entities → Graph
                    Query Flow: Question → Graph Traversal → LLM → Answer
```

### How the Algorithm Works

EdgeQuake implements the [LightRAG algorithm](https://arxiv.org/abs/2410.05779) in Rust. The core insight: **extract a knowledge graph during indexing, then traverse it during querying**.

**Indexing Pipeline** (per document):

1. **Chunk** — Split document into ~1200-token segments with 100-token overlap
2. **Extract** — LLM parses each chunk into `(entity, type, description)` and `(source, target, keywords, description)` tuples
3. **Glean** — Optional second pass catches missed entities (improves recall by ~18%)
4. **Normalize** — Deduplicate entities via case normalization and description merging (reduces duplicates by ~36-40%)
5. **Embed** — Generate vector embeddings for chunks and entities
6. **Store** — Write to PostgreSQL: chunks to pgvector, entities/relationships to Apache AGE graph

**Query Flow** (6 modes):

- **Naive** — Vector similarity on chunks only (fast, no graph)
- **Local** — Find relevant entities via vector search, then traverse their local graph neighborhood
- **Global** — Use Louvain community detection to find thematic clusters, retrieve community summaries
- **Hybrid** _(default)_ — Combine local entity context + global community context
- **Mix** — Weighted blend of naive vector results and graph-enhanced results
- **Bypass** — Skip retrieval entirely, pass question directly to LLM

See [LightRAG Algorithm Deep Dive](docs/deep-dives/lightrag-algorithm.md) for the complete technical explanation.

---

## Documentation

### 📚 Complete Documentation Index

Explore the full documentation at [docs/README.md](docs/README.md)

### 📦 SDKs

EdgeQuake provides official SDKs for multiple languages:

- [Python SDK](sdks/python/README.md) ([Changelog](sdks/python/CHANGELOG.md))
- [TypeScript SDK](sdks/typescript/README.md) ([Changelog](sdks/typescript/CHANGELOG.md))
- [Rust SDK](sdks/rust/README.md)
- [Other SDKs](sdks/) for C#, Go, Java, Kotlin, PHP, Ruby, Swift

See the [CHANGELOG.md](CHANGELOG.md) for SDK and core updates.

### 🚀 Getting Started (15 minutes)

| Guide                                                      | Description                | Time   |
| ---------------------------------------------------------- | -------------------------- | ------ |
| [Installation](docs/getting-started/installation.md)       | Prerequisites and setup    | 5 min  |
| [Quick Start](docs/getting-started/quick-start.md)         | First ingestion and query  | 10 min |
| [First Ingestion](docs/getting-started/first-ingestion.md) | Understanding the pipeline | 15 min |

### 📖 Tutorials (Hands-On)

| Tutorial                                                             | Description                     |
| -------------------------------------------------------------------- | ------------------------------- |
| [Building Your First RAG App](docs/tutorials/first-rag-app.md)       | End-to-end tutorial             |
| [PDF Ingestion](docs/tutorials/pdf-ingestion.md)                     | PDF upload and configuration    |
| [Multi-Tenant Setup](docs/tutorials/multi-tenant.md)                 | Workspace isolation             |
| [Document Ingestion](docs/tutorials/document-ingestion.md)           | Upload and processing workflows |
| [Migration from LightRAG](docs/tutorials/migration-from-lightrag.md) | Python to Rust migration guide  |

### 🏗️ Architecture (How It Works)

| Document                                     | Description                           |
| -------------------------------------------- | ------------------------------------- |
| [Overview](docs/architecture/overview.md)    | System design and components          |
| [Data Flow](docs/architecture/data-flow.md)  | How documents flow through the system |
| [Crate Reference](docs/architecture/crates/) | 11 Rust crates explained              |

### 💡 Core Concepts (Theory)

| Concept                                                 | Description                       |
| ------------------------------------------------------- | --------------------------------- |
| [Graph-RAG](docs/concepts/graph-rag.md)                 | Why knowledge graphs enhance RAG  |
| [Entity Extraction](docs/concepts/entity-extraction.md) | LLM-based entity recognition      |
| [Knowledge Graph](docs/concepts/knowledge-graph.md)     | Nodes, edges, and communities     |
| [Hybrid Retrieval](docs/concepts/hybrid-retrieval.md)   | Combining vector and graph search |

### Deep Dives (Advanced)

| Article                                                         | Description                                  |
| --------------------------------------------------------------- | -------------------------------------------- |
| [LightRAG Algorithm](docs/deep-dives/lightrag-algorithm.md)     | Core algorithm: extraction, graph, retrieval |
| [Query Modes](docs/deep-dives/query-modes.md)                   | 6 modes explained with trade-offs            |
| [Entity Normalization](docs/deep-dives/entity-normalization.md) | Deduplication and description merging        |
| [Gleaning](docs/deep-dives/gleaning.md)                         | Multi-pass extraction for completeness       |
| [Community Detection](docs/deep-dives/community-detection.md)   | Louvain clustering for global queries        |
| [Chunking Strategies](docs/deep-dives/chunking-strategies.md)   | Token-based segmentation with overlap        |
| [Embedding Models](docs/deep-dives/embedding-models.md)         | Model selection and dimension trade-offs     |
| [Graph Storage](docs/deep-dives/graph-storage.md)               | Apache AGE property graph backend            |
| [Vector Storage](docs/deep-dives/vector-storage.md)             | pgvector HNSW indexing and search            |
| [PDF Processing](docs/deep-dives/pdf-processing.md)             | Vision and EdgeParse extraction pipeline     |
| [Cost Tracking](docs/deep-dives/cost-tracking.md)               | LLM cost monitoring per operation            |
| [Pipeline Progress](docs/deep-dives/pipeline-progress.md)       | Real-time progress tracking                  |

### 📊 Comparisons

| Comparison                                                     | Key Insights                       |
| -------------------------------------------------------------- | ---------------------------------- |
| [vs LightRAG (Python)](docs/comparisons/vs-lightrag-python.md) | Performance and design differences |
| [vs GraphRAG](docs/comparisons/vs-graphrag.md)                 | Microsoft's approach comparison    |
| [vs Traditional RAG](docs/comparisons/vs-traditional-rag.md)   | Why graphs matter                  |

### API Reference

| API                                                | Description           |
| -------------------------------------------------- | --------------------- |
| [REST API](docs/api-reference/rest-api.md)         | HTTP endpoints        |
| [Extended API](docs/api-reference/extended-api.md) | Advanced API features |

### Operations (Production)

| Guide                                                       | Description           |
| ----------------------------------------------------------- | --------------------- |
| [Deployment](docs/operations/deployment.md)                 | Production deployment |
| [Configuration](docs/operations/configuration.md)           | All config options    |
| [Monitoring](docs/operations/monitoring.md)                 | Observability setup   |
| [Performance Tuning](docs/operations/performance-tuning.md) | Optimization guide    |

### 🐛 Troubleshooting

| Guide                                                    | Description                  |
| -------------------------------------------------------- | ---------------------------- |
| [Common Issues](docs/troubleshooting/common-issues.md)   | Debugging guide              |
| [PDF Extraction](docs/troubleshooting/pdf-extraction.md) | PDF-specific troubleshooting |

### 🔗 Integrations

| Integration                                           | Description                          |
| ----------------------------------------------------- | ------------------------------------ |
| [MCP Server](mcp/)                                    | Model Context Protocol for AI agents |
| [OpenWebUI](docs/integrations/open-webui.md)          | Chat interface with Ollama emulation |
| [LangChain](docs/integrations/langchain.md)           | Retriever and agent integration      |
| [Custom Clients](docs/integrations/custom-clients.md) | Python, TypeScript, Rust, Go clients |

### 📓 More Resources

- [FAQ](docs/faq.md) - Frequently asked questions
- [Cookbook](docs/cookbook.md) - Practical recipes
- [Security](docs/security/) - Security best practices

---

## Docker Deployment

EdgeQuake ships a production-ready multi-arch Docker image published to **GitHub Container Registry (GHCR)** on every tagged release. Supports `linux/amd64` (x86 servers, Intel Macs) and `linux/arm64` (Apple Silicon, AWS Graviton) out of the box — no QEMU emulation needed.

```bash
# Pull latest release — architecture is selected automatically
docker pull ghcr.io/raphaelmansuy/edgequake:latest

# Pin to a specific version
docker pull ghcr.io/raphaelmansuy/edgequake:0.10.8
```

> **First-time package visibility:** After the first CI/CD publish, you may need to set the GHCR package visibility to **Public** under [GitHub → Your Profile → Packages → edgequake → Package Settings → Change Visibility](https://github.com/raphaelmansuy?tab=packages). Once public, `docker pull` works without authentication.

Three deployment options are available depending on your setup:

---

### Option A — API only (bring your own PostgreSQL)

Fastest start if you already have PostgreSQL (with `pgvector` + `apache_age` extensions).
No Rust toolchain, no local build — just pull and run.

```bash
# One-liner
docker run -d \
  --name edgequake \
  -p 8080:8080 \
  -e DATABASE_URL="postgres://user:password@your-db-host:5432/edgequake" \
  -e EDGEQUAKE_LLM_PROVIDER=openai \
  -e OPENAI_API_KEY="sk-..." \
  ghcr.io/raphaelmansuy/edgequake:latest

# Verify
curl http://localhost:8080/health
```

Or with `docker compose` (recommended for persistent config):

```bash
cd edgequake/docker
cp .env.example .env        # set DATABASE_URL and provider key(s)
docker compose -f docker-compose.api-only.yml up -d
curl http://localhost:8080/health
```

---

### Option B — Full stack using prebuilt images *(fastest, no Rust needed)*

Pulls the EdgeQuake API, frontend, and PostgreSQL images directly from GHCR. No Rust toolchain, Node.js toolchain, or local Docker build is required.

```bash
cd edgequake/docker
cp .env.example .env        # set EDGEQUAKE_LLM_PROVIDER and your API key
docker compose -f docker-compose.prebuilt.yml up -d
```

Services started:

| Service         | Port | Image                                             |
| --------------- | ---- | ------------------------------------------------- |
| `edgequake` API | 8080 | `ghcr.io/raphaelmansuy/edgequake:latest`          |
| `frontend`      | 3000 | `ghcr.io/raphaelmansuy/edgequake-frontend:latest` |
| `postgres`      | 5432 | `ghcr.io/raphaelmansuy/edgequake-postgres:latest` |

```bash
# Use a specific API version
EDGEQUAKE_VERSION=0.10.8 docker compose -f docker-compose.prebuilt.yml up -d

# Logs
docker compose -f docker-compose.prebuilt.yml logs -f edgequake

# Health check
curl http://localhost:8080/health

# Stop
docker compose -f docker-compose.prebuilt.yml down
```

---

### Option C — Full stack, build from source (includes frontend)

Builds everything locally including the Next.js web UI. Requires Docker BuildKit and sufficient disk space (~4 GB for the Rust build cache).

```bash
cd edgequake/docker
docker compose up -d         # builds API, frontend, and postgres
```

Services started:

| Service              | Port | Description                           |
| -------------------- | ---- | ------------------------------------- |
| `edgequake` API      | 8080 | REST API + document processing        |
| `frontend` (Next.js) | 3000 | Web UI                                |
| `postgres`           | 5432 | PostgreSQL with pgvector + Apache AGE |

```bash
# Follow logs
docker compose logs -f edgequake

# Check health
curl http://localhost:8080/health

# Stop
docker compose down
```

---

### Environment Variables

All compose files read from a `.env` file placed in the same directory. Copy `edgequake/docker/.env.example` to get started.

| Variable                       | Default                             | Description                                                                             |
| ------------------------------ | ----------------------------------- | --------------------------------------------------------------------------------------- |
| `DATABASE_URL`                 | *(set by compose in full-stack)*    | PostgreSQL connection string                                                            |
| `EDGEQUAKE_LLM_PROVIDER`       | `ollama`                            | LLM provider: `openai`, `anthropic`, `gemini`, `mistral`, `azure`, `vertexai`, `ollama` |
| `EDGEQUAKE_EMBEDDING_PROVIDER` | *(same as LLM)*                     | Separate embedding provider for hybrid mode                                             |
| `OPENAI_API_KEY`               | —                                   | Required for `openai` / `azure`                                                         |
| `ANTHROPIC_API_KEY`            | —                                   | Required for `anthropic`                                                                |
| `GEMINI_API_KEY`               | —                                   | Required for `gemini`                                                                   |
| `MISTRAL_API_KEY`              | —                                   | Required for `mistral`                                                                  |
| `AZURE_OPENAI_API_KEY`         | —                                   | Required for `azure`                                                                    |
| `AZURE_OPENAI_ENDPOINT`        | —                                   | Azure resource endpoint URL                                                             |
| `GOOGLE_CLOUD_PROJECT`         | —                                   | Required for `vertexai`                                                                 |
| `XAI_API_KEY`                  | —                                   | Required for `xai`                                                                      |
| `OLLAMA_HOST`                  | `http://host.docker.internal:11434` | Ollama server URL (host machine via gateway)                                            |
| `EDGEQUAKE_VERSION`            | `latest`                            | GHCR image tag (Option B only)                                                          |
| `RUST_LOG`                     | `info`                              | Log level (`debug`, `info`, `warn`, `error`)                                            |

> **Tip — Ollama on the host:** When running EdgeQuake inside Docker and Ollama on your machine, leave `OLLAMA_HOST` at its default (`http://host.docker.internal:11434`). On Linux, the `extra_hosts: host.docker.internal:host-gateway` entry in the compose file resolves this automatically.

---

### Building the Image Locally

The Dockerfile lives at `edgequake/docker/Dockerfile` and uses a two-stage build (Rust builder → Debian slim runtime). pdfium is embedded at compile time via `pdfium-auto` — no external shared library is needed.

```bash
# Build for host architecture
docker build -f edgequake/docker/Dockerfile edgequake -t edgequake:local

# Multi-platform build (requires docker buildx)
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -f edgequake/docker/Dockerfile edgequake \
  -t edgequake:local --load
```

---

### CI/CD — Automated Releases

Docker images are built and published automatically via GitHub Actions (`.github/workflows/release-docker.yml`) when a version tag is pushed:

```bash
# Tag a release — triggers multi-arch docker build + publish to ghcr.io
git tag v0.10.8 && git push origin v0.10.8
```

Both `linux/amd64` (ubuntu-latest runner) and `linux/arm64` (native ARM64 runner — no QEMU) are built in parallel and merged into a single multi-arch manifest. The same image tag (`ghcr.io/raphaelmansuy/edgequake:0.10.8`) works on x86 servers, Apple Silicon Macs, and AWS Graviton instances.

You can also trigger a manual Docker build + publish without a tag via the `workflow_dispatch` input on GitHub Actions (`Actions → Release — Docker (GHCR) → Run workflow`).

---

## Development

### Building and Testing

```bash
# Build backend
cd edgequake && cargo build --release

# Run tests
cargo test

# Lint and format
cargo clippy
cargo fmt

# Build frontend
cd edgequake_webui
bun run build
```

### Make Commands

EdgeQuake uses a unified Makefile for all development tasks:

```bash
# Full development stack
make dev              # Start all services (PostgreSQL + Backend + Frontend)
make dev-bg           # Start in background (for agents/automation)
make dev-memory       # Start with in-memory storage (testing only)
make stop             # Stop all services
make status           # Check service status

# Backend only
make backend-dev      # Run backend with PostgreSQL
make backend-memory   # Run backend with in-memory storage
make backend-bg       # Run backend in background
make backend-test     # Run backend tests

# Frontend only
make frontend-dev     # Start frontend dev server
make frontend-build   # Build frontend for production

# Database
make db-start         # Start PostgreSQL container
make db-stop          # Stop PostgreSQL container
make db-wait          # Wait for database to be ready

# Quality checks
make test             # Run all tests
make lint             # Lint all code
make format           # Format all code
make clean            # Clean build artifacts
```

### Agent Workflow

EdgeQuake development follows a **Specification-Driven Development** approach using the `edgecode` SOTA coding agent.

- **AGENTS.md**: Comprehensive agent guidelines and workflow
- **specs/**: All development specifications
- **OODA Loop**: Iterative development cycles (Observe, Orient, Decide, Act)

See [AGENTS.md](AGENTS.md) for detailed agent workflow documentation.

---

## Contributing

EdgeQuake is developed using the **edgecode** SOTA coding agent created by **Raphaël MANSUY**. The project follows a **Specification-Driven Development** approach where all changes are specified in the `specs/` directory before implementation.

**Current Status**: `edgecode` is not yet public but will be released soon.

**For now, contributions should go through Raphaël MANSUY directly:**

- **GitHub Issues**: Report bugs and request features
- **GitHub Discussions**: Ask questions and share ideas
- **Direct Contact**: For major contributions, contact [@raphaelmansuy](https://github.com/raphaelmansuy)

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed contribution guidelines.

---

## Community & Support

### Code of Conduct

We are committed to providing a welcoming and inclusive environment. Please read our [Code of Conduct](CODE_OF_CONDUCT.md).

### Support Channels

- **GitHub Issues**: Bug reports and feature requests
- **GitHub Discussions**: Questions and community help
- **LinkedIn**: [@raphaelmansuy](https://www.linkedin.com/in/raphaelmansuy)
- **Twitter/X**: [@raphaelmansuy](https://twitter.com/raphaelmansuy)

### Founder

**Raphaël MANSUY** 🇫🇷 - 🇭🇰🇨🇳 — Permanent Resident of Hong Kong, building the future of intelligent document retrieval systems and context graph systems.

---

## License

Licensed under the Apache License, Version 2.0 (the "License").  
You may obtain a copy of the License at:

http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the [LICENSE](LICENSE) file for the specific language governing permissions and limitations.

**Copyright © 2024-2026 Raphaël MANSUY**

---

## Acknowledgments

EdgeQuake is inspired by and builds upon the excellent work of:

- **LightRAG Research Paper** ([arxiv.org/abs/2410.05779](https://arxiv.org/abs/2410.05779)): We are grateful to the authors of the foundational LightRAG algorithm which powers the core knowledge graph extraction and retrieval capabilities in EdgeQuake. Their innovative approach to entity extraction, relationship mapping, and hybrid retrieval has been instrumental in our framework's design.

  **Special thanks to the LightRAG authors:**
  - [Zirui Guo](https://arxiv.org/search/cs?searchtype=author&query=Guo,+Z)
  - [Lianghao Xia](https://arxiv.org/search/cs?searchtype=author&query=Xia,+L)
  - [Yanhua Yu](https://arxiv.org/search/cs?searchtype=author&query=Yu,+Y)
  - [Tu Ao](https://arxiv.org/search/cs?searchtype=author&query=Ao,+T)
  - [Chao Huang](https://arxiv.org/search/cs?searchtype=author&query=Huang,+C)

- **GraphRAG** ([arxiv.org/abs/2404.16130](https://arxiv.org/abs/2404.16130)): Microsoft's "From Local to Global" knowledge graph approach to query-focused summarization.
  - [Shuai Wang](https://www.microsoft.com/en-us/research/people/shuaiw/)
  - [Yingqiang Ge](https://www.microsoft.com/en-us/research/people/yinge/)
  - [Ying Shen](https://www.microsoft.com/en-us/research/people/yingshen/)
  - [Jianfeng Gao](https://www.microsoft.com/en-us/research/people/jfgao/)
  - [Xiaodong Liu](https://www.microsoft.com/en-us/research/people/xiaodl/)
  - [Yelong Shen](https://www.microsoft.com/en-us/research/people/yelongshen/)
  - [Jianfeng Wang](https://www.microsoft.com/en-us/research/people/jianfw/)
  - [Ming Zhou](https://www.microsoft.com/en-us/research/people/zhou/)
- **Rust Community**: For the amazing async ecosystem (Tokio, Axum, SQLx) that enables EdgeQuake's high performance
- **React Community**: For React 19 and the modern frontend stack that powers our interactive UI

---

## Quick Links

| Resource             | URL                                                                              |
| -------------------- | -------------------------------------------------------------------------------- |
| 📚 Full Documentation | [docs/README.md](docs/README.md)                                                 |
| 🚀 Quick Start Guide  | [docs/getting-started/quick-start.md](docs/getting-started/quick-start.md)       |
| 📦 SDKs Overview      | [sdks/](sdks/)                                                                   |
| 🐍 Python SDK         | [sdks/python/README.md](sdks/python/README.md)                                   |
| 🦀 Rust SDK           | [sdks/rust/README.md](sdks/rust/README.md)                                       |
| 🟦 TypeScript SDK     | [sdks/typescript/README.md](sdks/typescript/README.md)                           |
| 📜 CHANGELOG          | [CHANGELOG.md](CHANGELOG.md)                                                     |
| 🔧 Agent Workflow     | [AGENTS.md](AGENTS.md)                                                           |
| 🤝 Contributing       | [CONTRIBUTING.md](CONTRIBUTING.md)                                               |
| 📜 Code of Conduct    | [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)                                         |
| 📄 License            | [LICENSE](LICENSE)                                                               |
| 🐛 Report Issues      | [GitHub Issues](https://github.com/raphaelmansuy/edgequake/issues)               |
| 💬 Discussions        | [GitHub Discussions](https://github.com/raphaelmansuy/edgequake/discussions)     |
| 🌐 Repository         | [github.com/raphaelmansuy/edgequake](https://github.com/raphaelmansuy/edgequake) |

---

**Ready to build intelligent document retrieval?** [Get started now!](docs/getting-started/quick-start.md)

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=raphaelmansuy/edgequake&type=date&legend=top-left)](https://www.star-history.com/#raphaelmansuy/edgequake&type=date&legend=top-left)
