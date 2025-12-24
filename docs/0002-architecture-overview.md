# EdgeQuake Architecture Overview

> Technical deep-dive into EdgeQuake's Graph-Enhanced RAG system architecture

**Version**: 0.1.0 | **Last Updated**: December 2025 | **Language**: Rust

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

| Feature | Traditional RAG | EdgeQuake |
|---------|----------------|-----------|
| Language | Python | Rust |
| Retrieval | Vector similarity only | Graph + Vector hybrid |
| Context | Flat document chunks | Entity-Relation aware |
| Query Modes | Single mode | 5 modes (naive/local/global/hybrid/mix) |
| Knowledge | Implicit in embeddings | Explicit knowledge graph |
| Performance | Standard | High-performance, async |
| WebUI | Basic | Next.js 16 + React 19 |

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

### `edgequake-core` - Orchestration Layer

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
    pub async fn insert(&self, text: &str) -> Result<InsertResult>;
    pub async fn query(&self, query: &str) -> Result<QueryResult>;
    pub async fn delete_document(&self, doc_id: &str) -> Result<()>;
    pub fn get_graph_stats(&self) -> Result<GraphStats>;
}
```

### `edgequake-api` - REST API

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

```rust
// Located: edgequake/crates/edgequake-llm/src/traits.rs

#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn complete(&self, messages: &[ChatMessage], options: CompletionOptions) 
        -> Result<LLMResponse>;
    
    async fn complete_stream(&self, messages: &[ChatMessage], options: CompletionOptions) 
        -> Result<impl Stream<Item = Result<String>>>;
    
    fn model_name(&self) -> &str;
    fn max_context_length(&self) -> usize;
}

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
    fn dimension(&self) -> usize;
    fn model_name(&self) -> &str;
}
```

**Implemented Providers:**

| Provider | LLM | Embeddings | Notes |
|----------|-----|------------|-------|
| `OpenAIProvider` | ✅ | ✅ | Production ready |
| `MockProvider` | ✅ | ✅ | Testing, deterministic |

### `edgequake-storage` - Storage Abstractions

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

| Adapter | KV | Vector | Graph | Use Case |
|---------|----|----|-------|----------|
| `MemoryStorage` | ✅ | ✅ | ✅ | Development/Testing |
| `PostgresStorage` | ✅ | ✅ | ✅ | Production |

### `edgequake-query` - Query Engine

```rust
// Located: edgequake/crates/edgequake-query/src/modes.rs

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
│     { document_id, status: "completed", entity_count, relationship_count }  │
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

| Component | Technology |
|-----------|------------|
| Framework | Next.js 16 |
| React | React 19 |
| State | Zustand |
| Data Fetching | TanStack Query |
| Graph Visualization | Sigma.js + Graphology |
| Styling | Tailwind CSS 4 |
| Components | Radix UI |

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
│   ├── use-auth-store.ts
│   ├── use-graph-store.ts
│   └── use-query-store.ts
└── types/                 # TypeScript types
    └── index.ts
```

---

## Configuration

### EdgeQuake Configuration Structure

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

## Next Steps

1. **[API Reference](0003-api-reference.md)** - Complete REST API documentation
2. **[Storage Backends](0004-storage-backends.md)** - Configure storage
3. **[LLM Integration](0005-llm-integration.md)** - LLM providers
4. **[Deployment Guide](0006-deployment-guide.md)** - Production deployment
