# EdgeQuake Architecture Overview

> **Implements**: [FEAT0001](features.md#feat0001) Document Ingestion, [FEAT0002](features.md#feat0002) Knowledge Graph Query
>
> Technical deep-dive into EdgeQuake's Graph-Enhanced RAG system architecture

**Version**: 2.0.0 | **Last Updated**: January 2026 | **Language**: Rust

> **Code Reference**: Main crates in [edgequake/crates/](../edgequake/crates/)

---

## Quick Navigation

| Section                                       | What You'll Learn                               |
| --------------------------------------------- | ----------------------------------------------- |
| [System Overview](#system-overview)           | High-level architecture and key differentiators |
| [Crate Structure](#crate-structure)           | 11 Rust crates and their responsibilities       |
| [Data Flow](#data-flow)                       | How documents flow through the system           |
| [Query Pipeline](#query-pipeline)             | How queries are processed across 6 modes        |
| [Storage Architecture](#storage-architecture) | KV, Vector, and Graph storage patterns          |

---

## Table of Contents

1. [System Overview](#system-overview)
2. [Core Architecture](#core-architecture)
3. [Crate Structure](#crate-structure)
4. [Data Flow](#data-flow)
5. [Component Breakdown](#component-breakdown)
6. [Storage Architecture](#storage-architecture)
7. [Query Pipeline](#query-pipeline)

---

## System Overview

EdgeQuake is a **Graph-Enhanced Retrieval-Augmented Generation** framework implemented in Rust, combining knowledge graph construction with vector similarity search to provide contextually rich, accurate responses.

### Key Differentiators

| Feature     | Traditional RAG        | EdgeQuake                                      |
| ----------- | ---------------------- | ---------------------------------------------- |
| Language    | Python                 | Rust                                           |
| Retrieval   | Vector similarity only | Graph + Vector hybrid                          |
| Context     | Flat document chunks   | Entity-Relation aware                          |
| Query Modes | Single mode            | 6 modes (naive/local/global/hybrid/mix/bypass) |
| Knowledge   | Implicit in embeddings | Explicit knowledge graph                       |
| Performance | Standard               | High-performance, async                        |
| WebUI       | Basic                  | Next.js 16.1.0 + React 19.2.3                  |

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              EdgeQuake System                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐  │
│  │   Axum      │───▶│  EdgeQuake  │───▶│   Storage   │───▶│  Backends   │  │
│  │   API       │    │    Core     │    │   Traits    │    │             │  │
│  └─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘  │
│        │                  │                  │                  │          │
│        │                  ▼                  ▼                  ▼          │
│        │           ┌───────────┐      ┌───────────┐      ┌───────────┐    │
│        │           │ Pipeline  │      │ KV Store  │      │ PostgreSQL│    │
│        │           │ + Extract │      │ VectorDB  │      │ + pgvector│    │
│        │           │ + Merge   │      │ GraphDB   │      │ + AGE     │    │
│        │           └───────────┘      └───────────┘      └───────────┘    │
│        │                                                                   │
│        ▼                                                                   │
│  ┌─────────────────────────────────────────────────────────────────────┐  │
│  │                     LLM / Embedding Providers                        │  │
│  │                 OpenAI │ Ollama │ LM Studio (Compatible)            │  │
│  └─────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐  │
│  │                     Next.js WebUI (edgequake_webui)                  │  │
│  │            React 19 │ TypeScript │ Sigma.js Graph │ Zustand         │  │
│  └─────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Core Architecture

### Crate Dependency Graph

```
                    ┌─────────────────────┐
                    │   edgequake-api     │  ← REST API (Axum)
                    │   (handlers, routes)│
                    └──────────┬──────────┘
                               │
                               ▼
                    ┌─────────────────────┐
                    │   edgequake-core    │  ← Orchestration
                    │   (EdgeQuake class) │
                    └──────────┬──────────┘
                               │
          ┌────────────────────┼────────────────────┐
          │                    │                    │
          ▼                    ▼                    ▼
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│ edgequake-query │  │edgequake-pipeline│  │ edgequake-llm  │
│   (QueryEngine) │  │   (Pipeline)     │  │   (Providers)  │
└────────┬────────┘  └────────┬────────┘  └────────┬────────┘
         │                    │                    │
         └────────────────────┼────────────────────┘
                              │
                              ▼
                    ┌─────────────────────┐
                    │  edgequake-storage  │  ← Storage Traits
                    │  (KV, Vector, Graph)│
                    └─────────────────────┘
```

---

## Crate Structure

EdgeQuake consists of **11 specialized Rust crates**, each with a single responsibility:

### Core Crates (Business Logic)

| Crate                | Lines  | Responsibility               | Key Features                               |
| -------------------- | ------- | ---------------------------- | ------------------------------------------ |
| `edgequake-core`     | ~15,500 | Orchestration, types, config | EdgeQuake class, QueryParams, InsertResult |
| `edgequake-pipeline` | ~10,500 | Document processing          | Entity extraction, chunking, merging       |
| `edgequake-query`    | ~11,900 | Query engine                 | 6 query modes, context assembly            |

### Infrastructure Crates

| Crate               | Lines   | Responsibility   | Key Features                          |
| ------------------- | ------- | ---------------- | ------------------------------------- |
| `edgequake-api`     | ~37,400 | REST API         | Axum handlers, OpenAPI, SSE streaming |
| `edgequake-storage` | ~11,900 | Storage adapters | Memory, PostgreSQL, pgvector, AGE     |
| `edgequake-llm`     | ~8,500  | LLM providers    | OpenAI, Mock, streaming               |

### Specialized Crates

| Crate                    | Lines   | Responsibility   | Key Features                  |
| ------------------------ | ------- | ---------------- | ----------------------------- |
| `edgequake-pdf`          | ~26,000 | PDF extraction   | Text, tables, layout analysis |
| `edgequake-auth`         | ~2,900  | Authentication   | JWT, API keys, OAuth2         |
| `edgequake-audit`        | ~580    | Audit logging    | Compliance, event tracking    |
| `edgequake-tasks`        | ~3,400  | Background tasks | Async processing, job queue   |
| `edgequake-rate-limiter` | ~1,000  | Rate limiting    | Tenant quotas, throttling     |

> **Total Rust Code**: ~130,000 lines across 11 crates (as of January 2026)
>
> **Enforces**: [BR0003](business_rules.md#br0003) Modular Architecture, [BR0004](business_rules.md#br0004) Single Responsibility

### `edgequake-core` - Orchestration Layer

> **Code Reference**: [edgequake/crates/edgequake-core/src/orchestrator.rs](../edgequake/crates/edgequake-core/src/orchestrator.rs)

The central orchestrator that coordinates all RAG operations.

```rust
// Located: edgequake/crates/edgequake-core/src/orchestrator.rs

pub struct EdgeQuake {
    config: EdgeQuakeConfig,
    initialized: bool,

    // Storage backends
    kv_storage: Option<Arc<dyn KVStorage>>,
    vector_storage: Option<Arc<dyn VectorStorage>>,
    graph_storage: Option<Arc<dyn GraphStorage>>,

    // LLM providers
    llm_provider: Option<Arc<dyn LLMProvider>>,
    embedding_provider: Option<Arc<dyn EmbeddingProvider>>,

    // Processing components
    pipeline: Option<Arc<Pipeline>>,
    query_engine: Option<Arc<QueryEngine>>,
}

impl EdgeQuake {
    pub fn new(config: EdgeQuakeConfig) -> Self;
    pub fn with_storage_backends(...) -> Self;
    pub fn with_providers(...) -> Self;
    pub async fn initialize(&mut self) -> Result<()>;
    pub async fn insert(&self, content: &str, doc_id: Option<&str>) -> Result<InsertResult>;
    pub async fn query(&self, query: &str, params: Option<QueryParams>) -> Result<QueryResult>;
    pub async fn delete_document(&self, doc_id: &str) -> Result<bool>;
    pub async fn get_graph_stats(&self) -> Result<GraphStats>;
}
```

### `edgequake-api` - REST API

> **Code Reference**: [edgequake/crates/edgequake-api/src/routes.rs](../edgequake/crates/edgequake-api/src/routes.rs)

Axum-based REST API with OpenAPI documentation.

```rust
// Located: edgequake/crates/edgequake-api/src/routes.rs

pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Health endpoints
        .route("/health", get(handlers::health_check))
        .route("/ready", get(handlers::readiness_check))
        .route("/metrics", get(handlers::get_metrics))

        // API v1 endpoints
        .nest("/api/v1", api_v1_routes())
        .with_state(state)
}

fn api_v1_routes() -> Router<AppState> {
    Router::new()
        // Documents
        .route("/documents", post(handlers::upload_document))
        .route("/documents", get(handlers::list_documents))
        .route("/documents/upload", post(handlers::upload_file))
        .route("/documents/{document_id}", get(handlers::get_document))
        .route("/documents/{document_id}", delete(handlers::delete_document))

        // Query
        .route("/query", post(handlers::execute_query))
        .route("/query/stream", post(handlers::stream_query))

        // Graph
        .route("/graph", get(handlers::get_graph))
        .route("/graph/entities", post(handlers::create_entity))
        .route("/graph/entities/{entity_name}", get(handlers::get_entity))
        .route("/graph/relationships", post(handlers::create_relationship))

        // Tasks
        .route("/tasks", get(handlers::list_tasks))
        .route("/tasks/{track_id}", get(handlers::get_task))
}
```

### `edgequake-llm` - LLM Providers

> **Code Reference**: [edgequake/crates/edgequake-llm/src/traits.rs](../edgequake/crates/edgequake-llm/src/traits.rs)

```rust
// Located: edgequake/crates/edgequake-llm/src/traits.rs

#[async_trait]
pub trait LLMProvider: Send + Sync {
    fn name(&self) -> &str;
    fn model(&self) -> &str;
    fn max_context_length(&self) -> usize;

    async fn complete(&self, prompt: &str) -> Result<LLMResponse>;
    async fn complete_with_options(&self, prompt: &str, options: &CompletionOptions)
        -> Result<LLMResponse>;
    async fn chat(&self, messages: &[ChatMessage], options: Option<&CompletionOptions>)
        -> Result<LLMResponse>;
    async fn stream(&self, prompt: &str) -> Result<BoxStream<'static, Result<String>>>;

    fn supports_streaming(&self) -> bool;
    fn supports_json_mode(&self) -> bool;
}

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    fn name(&self) -> &str;
    fn model(&self) -> &str;
    fn dimension(&self) -> usize;
    fn max_tokens(&self) -> usize;
    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
    async fn embed_one(&self, text: &str) -> Result<Vec<f32>>;
}
```

**Implemented Providers:**

| Provider         | LLM | Embeddings | Notes                  |
| ---------------- | --- | ---------- | ---------------------- |
| `OpenAIProvider` | ✅  | ✅         | Production ready       |
| `MockProvider`   | ✅  | ✅         | Testing, deterministic |

### `edgequake-storage` - Storage Abstractions

> **Code Reference**: [edgequake/crates/edgequake-storage/src/traits/](../edgequake/crates/edgequake-storage/src/traits/)

```rust
// Located: edgequake/crates/edgequake-storage/src/traits/

#[async_trait]
pub trait KVStorage: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<serde_json::Value>>;
    async fn upsert(&self, items: &[(String, serde_json::Value)]) -> Result<()>;
    async fn delete(&self, keys: &[String]) -> Result<()>;
    async fn keys(&self) -> Result<Vec<String>>;
}

#[async_trait]
pub trait VectorStorage: Send + Sync {
    async fn upsert(&self, vectors: &[VectorEntry]) -> Result<()>;
    async fn search(&self, query: &[f32], top_k: usize) -> Result<Vec<SearchResult>>;
    async fn delete(&self, ids: &[String]) -> Result<()>;
}

#[async_trait]
pub trait GraphStorage: Send + Sync {
    async fn add_node(&self, node: GraphNode) -> Result<()>;
    async fn add_edge(&self, edge: GraphEdge) -> Result<()>;
    async fn get_node(&self, id: &str) -> Result<Option<GraphNode>>;
    async fn get_neighbors(&self, id: &str, depth: usize) -> Result<Vec<GraphNode>>;
    async fn get_knowledge_graph(&self, start: &str, depth: usize, limit: usize)
        -> Result<KnowledgeGraph>;
}
```

**Implemented Adapters:**

| Adapter           | KV  | Vector | Graph | Use Case            |
| ----------------- | --- | ------ | ----- | ------------------- |
| `MemoryStorage`   | ✅  | ✅     | ✅    | Development/Testing |
| `PostgresStorage` | ✅  | ✅     | ✅    | Production          |

### `edgequake-query` - Query Engine

> **Code Reference**: [edgequake/crates/edgequake-core/src/types/query.rs](../edgequake/crates/edgequake-core/src/types/query.rs) (canonical QueryMode)

```rust
// Located: edgequake/crates/edgequake-core/src/types/query.rs

#[derive(Debug, Clone, Copy)]
pub enum QueryMode {
    /// Simple vector similarity search
    Naive,

    /// Entity-centric local neighborhood search
    Local,

    /// Community-based global search
    Global,

    /// Combined local and global
    Hybrid,

    /// Weighted combination of all modes
    Mix,

    /// Skip retrieval, direct LLM query
    Bypass,
}

impl QueryMode {
    pub fn uses_vector_search(&self) -> bool {
        matches!(self, Self::Naive | Self::Local | Self::Mix)
    }

    pub fn uses_graph(&self) -> bool {
        matches!(self, Self::Local | Self::Global | Self::Hybrid | Self::Mix)
    }
}
```

### `edgequake-pipeline` - Document Processing

> **Code Reference**: [edgequake/crates/edgequake-pipeline/src/](../edgequake/crates/edgequake-pipeline/src/)

```rust
// Document processing pipeline stages:
// 1. Chunking - Split documents into chunks
// 2. Embedding - Generate vector embeddings
// 3. Extraction - Extract entities and relationships via LLM
// 4. Merging - Deduplicate and merge entities
// 5. Storage - Persist to backends

pub struct Pipeline {
    chunker: Chunker,
    extractor: LLMExtractor,
    merger: KnowledgeGraphMerger,
}

impl Pipeline {
    pub async fn process_document(&self, doc: &Document)
        -> Result<ProcessingResult> {
        // 1. Chunk the document
        let chunks = self.chunker.chunk(&doc.content)?;

        // 2. Extract entities and relationships
        let extractions = self.extractor.extract(&chunks).await?;

        // 3. Merge into knowledge graph
        let merged = self.merger.merge(extractions)?;

        Ok(ProcessingResult {
            chunks: chunks.len(),
            entities: merged.entities.len(),
            relationships: merged.relationships.len(),
        })
    }
}
```

### `edgequake-pdf` - PDF Extraction

> **Code Reference**: [edgequake/crates/edgequake-pdf/src/](../edgequake/crates/edgequake-pdf/src/)
>
> **Implements**: [FEAT1001-FEAT1025](features.md#advanced-pdf-features-feat10xx) | **Enforces**: [BR1001-BR1026](business_rules.md#pdf-processing-rules-br10xx)

Converts PDF documents to Markdown with structure preservation.

```rust
// PDF extraction pipeline architecture:
//
// ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
// │ SotaBackend │───▶│  Processor  │───▶│   Renderer  │───▶│  Markdown   │
// │ (parsing)   │    │   Chain     │    │ (Markdown)  │    │   Output    │
// └─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
//       │                  │
//       │                  ├─ LayoutProcessor
//       │                  ├─ TableDetectionProcessor
//       │                  ├─ HeaderDetectionProcessor
//       │                  ├─ StyleDetectionProcessor
//       │                  └─ PostProcessor
//       │
//       ├─ Font analysis
//       ├─ Text extraction (lopdf)
//       ├─ Image extraction
//       └─ Table detection (lattice/stream)

pub struct PdfExtractor {
    backend: SotaBackend,
    processor_chain: ProcessorChain,
    renderer: MarkdownRenderer,
}

impl PdfExtractor {
    pub async fn extract(&self, pdf_bytes: &[u8]) -> Result<ExtractionResult> {
        // 1. Parse PDF and extract raw content
        let document = self.backend.extract(pdf_bytes)?;

        // 2. Process through chain (layout, tables, headings)
        let processed = self.processor_chain.process(document)?;

        // 3. Render to Markdown
        let markdown = self.renderer.render(&processed)?;

        Ok(ExtractionResult { markdown, pages: processed.pages.len() })
    }
}
```

**Key Components:**

| Component             | Lines | Responsibility                  |
| --------------------- | ----- | ------------------------------- |
| `SotaBackend`         | ~3000 | PDF parsing, font/text analysis |
| `LatticeEngine`       | ~600  | Table detection via line grid   |
| `ProcessorChain`      | ~3000 | Content transformation pipeline |
| `MarkdownRenderer`    | ~800  | Final Markdown generation       |
| `ImageOcrProcessor`   | ~500  | Vision LLM image understanding  |

---

## Data Flow

### Document Ingestion Pipeline

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                        Document Ingestion Flow                               │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  1. INPUT                                                                    │
│     │   POST /api/v1/documents                                              │
│     │   { content: "...", title: "...", async_processing: bool }            │
│     ▼                                                                        │
│  ┌─────────────────────────────────────────────────────────────┐            │
│  │  2. CHUNKING                                                 │            │
│  │     • Token-based splitting (default: 1200 tokens)          │            │
│  │     • Overlap preservation (default: 100 tokens)            │            │
│  │     • Sentence boundary awareness                           │            │
│  └─────────────────────────────────────────────────────────────┘            │
│     │                                                                        │
│     ▼                                                                        │
│  ┌─────────────────────────────────────────────────────────────┐            │
│  │  3. EMBEDDING                                                │            │
│  │     • OpenAI text-embedding-3-small (1536 dims)             │            │
│  │     • Batch processing for efficiency                        │            │
│  │     • Store in VectorStorage                                 │            │
│  └─────────────────────────────────────────────────────────────┘            │
│     │                                                                        │
│     ▼                                                                        │
│  ┌─────────────────────────────────────────────────────────────┐            │
│  │  4. ENTITY EXTRACTION (LLM)                                  │            │
│  │     • GPT-4o-mini structured extraction                     │            │
│  │     • Entity types: PERSON, ORGANIZATION, LOCATION, etc.   │            │
│  │     • Relationship extraction: source → relation → target   │            │
│  └─────────────────────────────────────────────────────────────┘            │
│     │                                                                        │
│     ▼                                                                        │
│  ┌─────────────────────────────────────────────────────────────┐            │
│  │  5. ENTITY MERGING                                           │            │
│  │     • Name normalization: UPPERCASE_UNDERSCORE              │            │
│  │     • Duplicate detection and merging                        │            │
│  │     • Description aggregation                                │            │
│  └─────────────────────────────────────────────────────────────┘            │
│     │                                                                        │
│     ▼                                                                        │
│  ┌─────────────────────────────────────────────────────────────┐            │
│  │  6. GRAPH STORAGE                                            │            │
│  │     • Add nodes (entities) to GraphStorage                  │            │
│  │     • Add edges (relationships) to GraphStorage             │            │
│  │     • Update entity vectors in VectorStorage                │            │
│  └─────────────────────────────────────────────────────────────┘            │
│     │                                                                        │
│     ▼                                                                        │
│  OUTPUT                                                                      │
│     { document_id, status: "processed", entity_count, relationship_count }  │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

### Query Pipeline

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                           Query Pipeline                                     │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  1. INPUT                                                                    │
│     │   POST /api/v1/query                                                  │
│     │   { query: "...", mode: "hybrid" }                                    │
│     ▼                                                                        │
│  ┌─────────────────────────────────────────────────────────────┐            │
│  │  2. QUERY EMBEDDING                                          │            │
│  │     • Generate query vector embedding                        │            │
│  │     • Extract keywords for graph search                      │            │
│  └─────────────────────────────────────────────────────────────┘            │
│     │                                                                        │
│     ├────────────────────────────┬───────────────────────────┐              │
│     ▼                            ▼                           ▼              │
│  ┌───────────────┐    ┌───────────────┐    ┌───────────────┐               │
│  │ NAIVE MODE    │    │ LOCAL MODE    │    │ GLOBAL MODE   │               │
│  │ Vector search │    │ Entity match  │    │ Community     │               │
│  │ top-k chunks  │    │ + neighbors   │    │ summaries     │               │
│  └───────┬───────┘    └───────┬───────┘    └───────┬───────┘               │
│          │                    │                    │                        │
│          └────────────────────┼────────────────────┘                        │
│                               ▼                                              │
│  ┌─────────────────────────────────────────────────────────────┐            │
│  │  3. CONTEXT ASSEMBLY                                         │            │
│  │     • Merge chunks, entities, relationships                 │            │
│  │     • Apply token limits                                     │            │
│  │     • Rank by relevance                                      │            │
│  └─────────────────────────────────────────────────────────────┘            │
│     │                                                                        │
│     ▼                                                                        │
│  ┌─────────────────────────────────────────────────────────────┐            │
│  │  4. LLM GENERATION                                           │            │
│  │     • Build prompt with context                              │            │
│  │     • Generate answer (streaming supported)                  │            │
│  └─────────────────────────────────────────────────────────────┘            │
│     │                                                                        │
│     ▼                                                                        │
│  OUTPUT                                                                      │
│     { answer, mode, sources: [...], stats: { embedding_time, ... } }        │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## WebUI Architecture

### Technology Stack

| Component           | Technology            |
| ------------------- | --------------------- |
| Framework           | Next.js 16            |
| React               | React 19              |
| State               | Zustand               |
| Data Fetching       | TanStack Query        |
| Graph Visualization | Sigma.js + Graphology |
| Styling             | Tailwind CSS 4        |
| Components          | Radix UI              |

### Component Structure

```
edgequake_webui/src/
├── app/                    # Next.js App Router
│   ├── (auth)/            # Auth routes (login)
│   ├── (dashboard)/       # Dashboard routes
│   │   ├── page.tsx       # Main dashboard
│   │   ├── documents/     # Document management
│   │   ├── query/         # Query interface
│   │   ├── graph/         # Graph visualization
│   │   └── settings/      # Settings
│   └── layout.tsx         # Root layout
├── components/
│   ├── ui/                # shadcn/ui components
│   ├── graph/             # Graph visualization
│   ├── documents/         # Document components
│   ├── query/             # Query components
│   └── layout/            # Layout components
├── lib/
│   └── api/               # API client
│       ├── client.ts      # HTTP client
│       └── edgequake.ts   # API functions
├── stores/                # Zustand stores
│   ├── use-auth-store.ts          # Authentication state
│   ├── use-backend-store.ts       # Backend connection state
│   ├── use-conversation-store.ts  # Chat conversation history
│   ├── use-cost-store.ts          # Cost tracking
│   ├── use-graph-store.ts         # Graph visualization state
│   ├── use-ingestion-store.ts     # Document ingestion progress
│   ├── use-query-store.ts         # Query execution state
│   ├── use-query-ui-store.ts      # Query UI preferences
│   ├── use-settings-store.ts      # User settings
│   ├── use-tenant-store.ts        # Multi-tenant state
│   └── use-ui-preferences-store.ts # UI theme/layout
└── types/                 # TypeScript types
    └── index.ts
```

### State Management with Zustand

The WebUI uses Zustand for lightweight, performant state management. Each store manages a specific domain:

| Store                    | Responsibility                                     | Persisted |
| ------------------------ | -------------------------------------------------- | --------- |
| `use-auth-store`         | JWT tokens, user info, login/logout                | ✅        |
| `use-backend-store`      | Backend URL, connection status                     | ✅        |
| `use-conversation-store` | Chat history, message threading                    | ✅        |
| `use-cost-store`         | Token usage, estimated costs                       | ❌        |
| `use-graph-store`        | Sigma.js instance, graph layout, filters           | ❌        |
| `use-ingestion-store`    | Upload progress, processing status                 | ❌        |
| `use-query-store`        | Query mode, parameters, history                    | ✅        |
| `use-query-ui-store`     | UI toggles (sources panel, graph visibility)       | ✅        |
| `use-settings-store`     | LLM model, temperature, max tokens                 | ✅        |
| `use-tenant-store`       | Active tenant, workspace                           | ✅        |
| `use-ui-preferences-store`| Dark mode, sidebar collapsed, language             | ✅        |

```typescript
// Example: use-query-store.ts
import { create } from 'zustand';
import { persist } from 'zustand/middleware';

interface QueryStore {
  mode: 'naive' | 'local' | 'global' | 'hybrid' | 'mix' | 'bypass';
  topK: number;
  maxTokens: number;
  history: string[];
  setMode: (mode: string) => void;
  addToHistory: (query: string) => void;
}

export const useQueryStore = create<QueryStore>()(
  persist(
    (set) => ({
      mode: 'hybrid',
      topK: 10,
      maxTokens: 4000,
      history: [],
      setMode: (mode) => set({ mode }),
      addToHistory: (query) => set((state) => ({ 
        history: [query, ...state.history].slice(0, 50) 
      })),
    }),
    { name: 'query-store' }
  )
);
```

### WebUI Data Flow

```
┌────────────────────────────────────────────────────────────────────────────┐
│                          WebUI ↔ Backend Flow                              │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│  ┌─────────────────┐                    ┌─────────────────────────────┐   │
│  │   React 19      │                    │     Rust Backend (Axum)     │   │
│  │   Components    │                    │     localhost:8080          │   │
│  └────────┬────────┘                    └─────────────┬───────────────┘   │
│           │                                           │                    │
│           ▼                                           │                    │
│  ┌─────────────────┐                                  │                    │
│  │   Zustand       │  ◄──────────────────────────────┘                    │
│  │   Stores        │        State Updates                                  │
│  │  (auth, graph,  │                                                       │
│  │   query, docs)  │                                                       │
│  └────────┬────────┘                                                       │
│           │                                                                │
│           ▼                                                                │
│  ┌─────────────────┐    HTTP/SSE          ┌──────────────────────────┐   │
│  │  TanStack Query │ ───────────────────► │  REST API Endpoints      │   │
│  │  (Data Fetch)   │ ◄─────────────────── │  /api/v1/documents       │   │
│  └─────────────────┘    JSON Response     │  /api/v1/query           │   │
│                                            │  /api/v1/graph           │   │
│                         ┌──────────────── │  /api/v1/chat (SSE)      │   │
│                         ▼                  └──────────────────────────┘   │
│  ┌─────────────────────────────────────────────────────────────────────┐  │
│  │                    SSE Streaming (Query/Chat)                        │  │
│  │     text/event-stream: data: {"token": "Hello"}\n\n                 │  │
│  │     Rendered by StreamingMarkdownRenderer using marked.lexer()      │  │
│  └─────────────────────────────────────────────────────────────────────┘  │
│                                                                            │
└────────────────────────────────────────────────────────────────────────────┘
```

---

## Configuration

### EdgeQuake Configuration Structure

> **Code Reference**: [edgequake/crates/edgequake-core/src/config.rs](../edgequake/crates/edgequake-core/src/config.rs)

```rust
// Located: edgequake/crates/edgequake-core/src/config.rs

pub struct Config {
    pub storage: StorageConfig,
    pub llm: LlmConfig,
    pub pipeline: PipelineConfig,
    pub query: QueryConfig,
    pub api: ApiConfig,
}

pub struct StorageConfig {
    pub database_url: String,      // postgres://localhost/edgequake
    pub max_connections: u32,      // 10
    pub min_connections: u32,      // 1
    pub connect_timeout_secs: u64, // 30
    pub namespace: Option<String>, // Multi-tenant namespace
}

pub struct LlmConfig {
    pub provider: String,          // "openai"
    pub api_key: Option<String>,   // From OPENAI_API_KEY
    pub base_url: Option<String>,  // Custom endpoint
    pub model: String,             // "gpt-4o-mini"
    pub embedding_model: String,   // "text-embedding-3-small"
    pub embedding_dim: usize,      // 1536
    pub max_tokens: usize,         // 4096
    pub temperature: f32,          // 0.0
}

pub struct PipelineConfig {
    pub chunk_size: usize,         // 1200 tokens
    pub chunk_overlap: usize,      // 100 tokens
    pub entity_types: Vec<String>, // PERSON, ORG, LOCATION...
    pub concurrency: usize,        // 4 parallel tasks
}

pub struct QueryConfig {
    pub default_mode: QueryMode,   // Hybrid
    pub max_vector_results: usize, // 20
    pub max_graph_depth: usize,    // 3
    pub max_context_chunks: usize, // 20
}

pub struct ApiConfig {
    pub host: String,              // "0.0.0.0"
    pub port: u16,                 // 8080
    pub cors_enabled: bool,        // true
    pub body_limit: usize,         // 10MB
}
```

---

## Design Principles

EdgeQuake follows these core architectural principles:

| Principle                    | Implementation                                 | Rationale                                |
| ---------------------------- | ---------------------------------------------- | ---------------------------------------- |
| **Trait-based Abstraction**  | All storage and LLM providers implement traits | Enables easy swapping of implementations |
| **Async-first**              | All I/O operations are async with Tokio        | High concurrency without blocking        |
| **Zero-copy where possible** | Efficient buffer handling                      | Minimize memory allocations              |
| **Fail-fast validation**     | Input validation at API boundary               | Clear error messages, no silent failures |
| **Namespace isolation**      | All data scoped to tenant namespace            | Multi-tenancy without data leakage       |

> **Enforces**: [BR0001](business_rules.md#br0001) Tenant Isolation, [BR0005](business_rules.md#br0005) Async Operations

---

## Next Steps

| Your Goal                   | Next Document                                        |
| --------------------------- | ---------------------------------------------------- |
| Integrate via REST API      | [API Reference](0003-api-reference.md)               |
| Configure storage backends  | [Storage Backends](0004-storage-backends.md)         |
| Set up LLM providers        | [LLM Integration](0005-llm-integration.md)           |
| Deploy to production        | [Deployment Guide](0006-deployment-guide.md)         |
| Understand query algorithms | [Algorithms Reference](0009-algorithms-reference.md) |

> **See Also**: [Features Registry](features.md) for complete FEAT0001-XXXX catalog
