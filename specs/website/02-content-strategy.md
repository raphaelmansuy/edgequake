# SPEC-WEBSITE-02: Content Strategy

> **Status**: DRAFT  
> **Created**: 2026-03-21  
> **Parent**: [00-overview.md](./00-overview.md)  
> **Related**: [01-information-architecture.md](./01-information-architecture.md) · [03-page-specifications.md](./03-page-specifications.md) · [05-seo-strategy.md](./05-seo-strategy.md)

---

## 1. WHY → WHAT → HOW Framework

The entire website follows the Golden Circle communication model. Every section, every page, and every component respects this hierarchy:

```
┌──────────────────────────────────────────────────────────────┐
│                                                              │
│  ┌─────────┐    ┌──────────────┐    ┌────────────────────┐  │
│  │         │    │              │    │                    │  │
│  │   WHY   │───►│    WHAT      │───►│       HOW          │  │
│  │         │    │              │    │                    │  │
│  │ Purpose │    │  Capability  │    │  Implementation    │  │
│  │ Emotion │    │  Features    │    │  Code Examples     │  │
│  │ Pain    │    │  Comparison  │    │  API Docs          │  │
│  │         │    │              │    │                    │  │
│  └─────────┘    └──────────────┘    └────────────────────┘  │
│                                                              │
│  SCROLL DIRECTION ──────────────────────────────────────►    │
│  (top to bottom on homepage, left to right in nav)           │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

---

## 2. Core Messaging

### 2.1 Value Proposition Statement

> **EdgeQuake**: The knowledge graph-powered RAG framework built in Rust.  
> 10x faster than Python. 6 query modes. Production-ready.

### 2.2 Tagline Variants (for different contexts)

| Context | Tagline |
|---------|---------|
| **Hero (homepage)** | "Graph-RAG Built for Speed" |
| **Hero subtitle** | "Turn documents into knowledge graphs. Query with 6 modes. 10x faster than Python RAG." |
| **OpenGraph / Twitter** | "EdgeQuake — High-performance Graph-RAG in Rust. 10x faster, 6 query modes, production-ready." |
| **GitHub description** | "High-performance Graph-RAG framework in Rust with 6 query modes, knowledge graph extraction, and production-ready API" |
| **One-line elevator** | "EdgeQuake is what happens when you rewrite LightRAG in Rust and add multi-tenancy, streaming, and MCP support." |

### 2.3 Key Messages by Audience

| Audience | Primary Message | Supporting Evidence |
|----------|----------------|-------------------|
| **AI/ML Engineers** | "Stop losing relationships between concepts. Your RAG needs a knowledge graph." | Entity extraction demo, query mode comparison |
| **CTOs / Architects** | "Production-grade Graph-RAG with multi-tenant isolation, audit logging, and streaming." | Architecture diagram, enterprise features list |
| **Data Engineers** | "Ingest 1000+ docs/min. PDF vision pipeline. PostgreSQL backend." | Benchmark charts, deployment guide |
| **OSS Contributors** | "10 modular Rust crates. Well-documented. Apache 2.0." | Crate architecture, contributing guide |

---

## 3. Content Sections — WHY Layer (Emotional / Problem)

### 3.1 "RAG is Broken" Problem Section

**Goal**: Create urgency by showing the viewer's current pain.

```
┌─────────────────────────────────────────────────────────────────┐
│                     THE PROBLEM                                  │
│            "Traditional RAG Loses Knowledge"                     │
│                                                                  │
│  ┌─────────────────┐  ┌─────────────────┐  ┌────────────────┐  │
│  │ 🔍 Lost         │  │ 🐌 Slow         │  │ 📦 Single      │  │
│  │ Relationships   │  │ at Scale        │  │ Query Mode     │  │
│  │                 │  │                 │  │                │  │
│  │ Vectors capture │  │ Python RAG      │  │ Vector         │  │
│  │ similarity but  │  │ processes ~100  │  │ similarity ≠   │  │
│  │ lose structure. │  │ docs/min. Real  │  │ the right      │  │
│  │ "Sarah works at │  │ workloads need  │  │ retrieval for  │  │
│  │ Acme" becomes   │  │ 10x more.       │  │ every question │  │
│  │ two unrelated   │  │                 │  │ type.          │  │
│  │ embeddings.     │  │                 │  │                │  │
│  └─────────────────┘  └─────────────────┘  └────────────────┘  │
│                                                                  │
│  ┌─────────────────┐  ┌─────────────────┐  ┌────────────────┐  │
│  │ 🏗️ No Multi-    │  │ 📄 PDF = Pain   │  │ 💸 Cloud-Only  │  │
│  │ Tenancy         │  │                 │  │ Cost           │  │
│  │                 │  │ Most RAG tools  │  │                │  │
│  │ Sharing one RAG │  │ choke on tables,│  │ Forced to use  │  │
│  │ across teams    │  │ multi-column    │  │ expensive APIs │  │
│  │ means data      │  │ layouts, and    │  │ for embeddings │  │
│  │ leaks.          │  │ scanned docs.   │  │ and queries.   │  │
│  └─────────────────┘  └─────────────────┘  └────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

**Copy for each pain card:**

| Pain | Headline | Body (≤ 30 words) |
|------|----------|-------------------|
| Lost Relationships | "Vectors Lose Structure" | Embeddings capture semantic similarity but destroy structural relationships. "Sarah works at Acme" becomes two unrelated vectors. |
| Slow at Scale | "Python Hits a Wall at Scale" | Python RAG processes ~100 docs/min. When you have 10,000+ documents, you need 10x throughput — or wait days. |
| Single Query Mode | "One Size Doesn't Fit All" | Vector cosine similarity is the only retrieval mode. But "who works at Acme?" needs graph traversal, not embedding distance. |
| No Multi-Tenancy | "Shared RAG = Data Leaks" | Running one RAG pipeline for multiple teams without workspace isolation means sensitive data crosses boundaries. |
| PDF = Pain | "PDFs Weren't Built for AI" | Tables become unstructured text. Multi-column layouts scramble reading order. Scanned documents need vision LLM support. |
| Cloud-Only Cost | "Locked Into Expensive APIs" | Can't mix local embeddings with cloud LLM. Every request costs money even during development and testing. |

### 3.2 Social Proof Quote (if available)

```
"If the data isn't structured properly, your RAG system will never
 retrieve accurate answers. Knowledge graphs change everything."
```

---

## 4. Content Sections — WHAT Layer (Features / Capabilities)

### 4.1 Feature Grid (6 Core Capabilities)

```
┌─────────────────────────────────────────────────────────────────┐
│                 THE SOLUTION: EdgeQuake                           │
│          "Graph-RAG That Actually Works in Production"            │
│                                                                  │
│  ┌─────────────────┐  ┌─────────────────┐  ┌────────────────┐  │
│  │ 🧠 Knowledge    │  │ ⚡ 10x Faster   │  │ 🔀 6 Query     │  │
│  │ Graph Engine    │  │                 │  │ Modes          │  │
│  │                 │  │ Rust core       │  │                │  │
│  │ LLM extracts   │  │ processes       │  │ Naive, Local,  │  │
│  │ entities &     │  │ 1000+ docs/min  │  │ Global, Hybrid │  │
│  │ relationships  │  │ with 200MB RAM  │  │ Mix, Bypass    │  │
│  │ into Apache    │  │ per core.       │  │                │  │
│  │ AGE graphs.    │  │                 │  │ Right retrieval│  │
│  │                 │  │                 │  │ for every      │  │
│  │ [Learn more →] │  │ [Benchmarks →]  │  │ question type. │  │
│  └─────────────────┘  └─────────────────┘  └────────────────┘  │
│                                                                  │
│  ┌─────────────────┐  ┌─────────────────┐  ┌────────────────┐  │
│  │ 🏢 Multi-Tenant │  │ 📄 PDF Vision   │  │ 🤖 MCP         │  │
│  │ Workspaces     │  │ Pipeline        │  │ Integration    │  │
│  │                 │  │                 │  │                │  │
│  │ Workspace      │  │ Text + Vision   │  │ 18 tools for   │  │
│  │ isolation,     │  │ LLM modes.      │  │ Claude, VS     │  │
│  │ RBAC, audit    │  │ Handles tables, │  │ Code, Cursor.  │  │
│  │ logging, rate  │  │ scanned docs,   │  │ Use EdgeQuake  │  │
│  │ limiting.      │  │ multi-column.   │  │ as persistent  │  │
│  │                 │  │                 │  │ AI memory.     │  │
│  │ [Enterprise →] │  │ [PDF Guide →]   │  │ [MCP Docs →]   │  │
│  └─────────────────┘  └─────────────────┘  └────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### 4.2 Feature Detail Copy

| Feature ID | Feature | Headline | Body (≤ 40 words) | CTA |
|-----------|---------|----------|-------------------|-----|
| F1 | Knowledge Graph | "Documents → Knowledge Graphs" | LLM-powered entity extraction identifies 7 entity types (Person, Org, Location, Concept, Event, Tech, Product) and their relationships. Stored in Apache AGE on PostgreSQL. | Learn more → |
| F2 | 10x Performance | "Rust Speed, Python Ease" | Process 1000+ documents/minute. 200-400MB RAM per core. No GIL bottleneck. Async I/O throughout. 10x faster than Python Graph-RAG implementations. | See benchmarks → |
| F3 | 6 Query Modes | "The Right Retrieval for Every Question" | Naive (fast vector), Local (entity neighborhood), Global (community summary), Hybrid (vector + graph), Mix (all signals), Bypass (direct LLM). | Query modes guide → |
| F4 | Multi-Tenancy | "Enterprise Isolation Built In" | Workspace-level data isolation. JWT + API key auth. Role-based access control. Audit logging. Rate limiting per tenant. | Enterprise features → |
| F5 | PDF Vision | "PDFs That Actually Parse" | Dual-mode pipeline: text extraction for clean PDFs, vision LLM for scanned documents. Handles tables, multi-column layouts, embedded images. | PDF guide → |
| F6 | MCP Integration | "Persistent Memory for AI Agents" | 18 MCP tools expose query, document management, workspace ops, and graph exploration to Claude Desktop, VS Code, and Cursor. | MCP docs → |

---

## 5. Content Sections — HOW Layer (Technical / Getting Started)

### 5.1 Quick Start Code Blocks

**Tab 1: Rust (cargo)**
```rust
// Add EdgeQuake to your project
// Cargo.toml
[dependencies]
edgequake-core = "0.7"
edgequake-llm = "0.3"

// Ingest a document
let pipeline = Pipeline::new(config).await?;
pipeline.ingest("document.pdf").await?;

// Query with hybrid mode
let result = engine.query("Who works at Acme?", QueryMode::Hybrid).await?;
```

**Tab 2: Docker**
```bash
# Start EdgeQuake with Docker
docker compose up -d

# Upload a document
curl -X POST http://localhost:8080/api/v1/documents \
  -F "file=@document.pdf"

# Query
curl http://localhost:8080/api/v1/query \
  -d '{"query": "Who works at Acme?", "mode": "hybrid"}'
```

**Tab 3: REST API**
```bash
# Query with streaming
curl -N http://localhost:8080/api/v1/query/stream \
  -H "Content-Type: application/json" \
  -d '{"query": "Summarize key relationships", "mode": "global"}'
```

### 5.2 Architecture Diagram Content

```
┌──────────────────────────────────────────────────────────────────┐
│                    EdgeQuake Architecture                          │
│                                                                   │
│   Documents                REST API (Axum)           Clients      │
│   ─────────               ──────────────────         ────────     │
│   PDF ──────┐             /api/v1/documents    ┌──── WebUI       │
│   Text ─────┤    ┌───────►/api/v1/query   ─────┤     MCP        │
│   Markdown ─┘    │       /api/v1/workspaces    └──── cURL       │
│        │         │        │                 │                     │
│        ▼         │        ▼                 ▼                     │
│   ┌──────────┐   │   ┌──────────┐    ┌──────────┐               │
│   │ Pipeline │   │   │  Query   │    │  Auth    │               │
│   │          │   │   │  Engine  │    │  + RBAC  │               │
│   │ Chunk    │   │   │          │    └──────────┘               │
│   │ Extract  │   │   │ 6 Modes │                                │
│   │ Embed    │   │   │ Stream  │                                │
│   │ Merge    │   │   └────┬─────┘                               │
│   └────┬─────┘   │        │                                      │
│        │         │        │                                      │
│        ▼         │        ▼                                      │
│   ┌────────────────────────────────────┐                         │
│   │         PostgreSQL                  │                         │
│   │  ┌──────────┐  ┌───────────────┐   │                         │
│   │  │ pgvector │  │  Apache AGE   │   │                         │
│   │  │ (vectors)│  │  (knowledge   │   │                         │
│   │  │          │  │   graph)      │   │                         │
│   │  └──────────┘  └───────────────┘   │                         │
│   └────────────────────────────────────┘                         │
│                                                                   │
│   LLM Providers: OpenAI · Ollama · LM Studio · Mock              │
└──────────────────────────────────────────────────────────────────┘
```

---

## 6. Benchmarks Content

### 6.1 Performance Comparison Data

| Metric | EdgeQuake (Rust) | LightRAG (Python) | Microsoft GraphRAG | Traditional RAG |
|--------|-----------------|-------------------|-------------------|----------------|
| **Throughput** | ~1000 docs/min | ~100 docs/min | ~50 docs/min | ~200 docs/min |
| **Memory/Core** | 200-400 MB | 2-4 GB | 1-2 GB | 500 MB-1 GB |
| **Query Modes** | 6 | 3 | 2 | 1 |
| **Streaming** | ✅ SSE | ❌ | ❌ | ❌ |
| **Multi-Tenant** | ✅ Built-in | ❌ | ❌ | ❌ |
| **Entity Types** | 7 configurable | 3 fixed | 3 fixed | N/A |
| **PDF Vision** | ✅ Text + LLM | ❌ Text only | ❌ Text only | ❌ Text only |
| **Graph DB** | PostgreSQL AGE | Neo4j/Kuzu | Custom | None |

### 6.2 Chart Specifications

| Chart | Type | Data Source |
|-------|------|------------|
| Throughput Comparison | Horizontal bar chart | Internal benchmarks |
| Memory Usage | Grouped bar chart | Profile data |
| Query Mode Coverage | Radar/spider chart | Feature matrix |
| Entity Extraction Quality | Metric cards | Test results (20 entities → 12 unique) |

---

## 7. Ecosystem Content

### 7.1 Crate Cards

Each crate gets a card with:

```
┌─────────────────────────────────────┐
│ 📦 edgequake-core                   │
│ ─────────────────────────────       │
│ Orchestration layer with pipeline   │
│ management, domain types, and       │
│ workspace services.                 │
│                                     │
│ Key: Pipeline · Types · Workspaces  │
│                                     │
│ [Docs →]  [API →]  [Source →]       │
└─────────────────────────────────────┘
```

| Crate | One-Line Description | Category |
|-------|---------------------|----------|
| edgequake-core | Orchestration, domain types, pipeline management | Core |
| edgequake-api | REST API server with OpenAPI docs and SSE streaming | API |
| edgequake-storage | Vector, graph, and KV storage adapters (PostgreSQL) | Storage |
| edgequake-pipeline | Document ingestion: chunking, extraction, embedding | Pipeline |
| edgequake-query | 6-mode query engine with token budgeting | Query |
| edgequake-llm | LLM provider abstraction (OpenAI, Ollama, LM Studio) | LLM |
| edgequake-pdf2md | PDF→Markdown with embedded pdfium + vision LLM | PDF |
| edgequake-auth | JWT, API keys, RBAC, multi-tenancy | Security |
| edgequake-audit | Compliance audit logging with PostgreSQL storage | Compliance |
| edgequake-tasks | Background task queue with retry and progress tracking | Tasks |

### 7.2 Integration Cards

| Integration | Description | Status |
|------------|-------------|--------|
| MCP Server | 18 tools for AI agent integration | ✅ Stable |
| Docker | Single-container deployment | ✅ Stable |
| Kubernetes | Health probes, HPA ready | ✅ Stable |
| Open WebUI | Chat interface integration | ✅ Documented |
| LangChain | Agent integration guide | ✅ Documented |

---

## 8. Enterprise Content

### 8.1 Enterprise Feature List

| Feature | Description |
|---------|------------|
| **Multi-Tenant Workspaces** | Complete data isolation between teams/customers |
| **RBAC & JWT Auth** | Fine-grained access control with role-based permissions |
| **Audit Logging** | Every operation logged for compliance (SOC 2, GDPR) |
| **Rate Limiting** | Per-tenant rate limits to prevent abuse |
| **Background Tasks** | Async processing with retry, backoff, and progress tracking |
| **Hybrid Provider Mode** | Mix local embeddings + cloud LLM for cost optimization |
| **SQL-Level Filtering** | 90% fewer vector scans with metadata pre-filtering |
| **SSE Streaming** | Real-time token-by-token response delivery |

### 8.2 Enterprise CTA Copy

> **Need EdgeQuake for your organization?**  
> Get dedicated support, custom integrations, and architecture consulting from the team behind EdgeQuake.  
> [Contact Us →] contact@elitizon.com

---

## 9. Content Voice & Tone

| Attribute | Guideline | Example |
|-----------|----------|---------|
| **Confidence** | State benefits assertively | "10x faster" not "potentially faster" |
| **Technical** | Use precise terminology | "Apache AGE graph" not "graph database" |
| **Concise** | Every word earns its place | Max 30 words per feature description |
| **Developer-first** | Code > prose | Show `cargo add` before explaining what it does |
| **Honest** | Acknowledge limitations | "Mock provider for testing" shows maturity |
| **Action-oriented** | Every section has a CTA | "Get Started →", "See Benchmarks →" |

### Vocabulary

| Prefer | Avoid |
|--------|-------|
| Knowledge graph | Graph database |
| Entity extraction | NER |
| Query modes | Search modes |
| Ingest / Ingestion | Upload / Import |
| Workspace | Tenant / Namespace |
| Provider | Backend / Driver |
| Streaming | Real-time |

---

## 10. Content Maintenance Plan

| Content Type | Update Frequency | Source of Truth |
|-------------|-----------------|----------------|
| Feature descriptions | Per release | CHANGELOG.md |
| Benchmarks | Per major release | Internal benchmarks |
| API reference | Per release | OpenAPI spec |
| Quick start code | Per major release | examples/ directory |
| Enterprise features | Quarterly | Product roadmap |
| Crate descriptions | Per release | Cargo.toml descriptions |

---

*Previous: [01-information-architecture.md](./01-information-architecture.md) · Next: [03-page-specifications.md](./03-page-specifications.md)*
