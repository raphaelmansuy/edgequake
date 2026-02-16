# Architecture: Crate Reference

> **Understanding EdgeQuake's Modular Rust Architecture**

EdgeQuake is organized into 11 focused Rust crates, each with a single responsibility. This guide explains each crate's purpose, dependencies, and key types.

---

## Crate Dependency Graph

```
┌─────────────────────────────────────────────────────────────────┐
│                    EDGEQUAKE CRATE HIERARCHY                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│                      ┌──────────────────┐                       │
│                      │  edgequake-api   │  ◀── HTTP Entry Point │
│                      │   (Axum server)  │                       │
│                      └────────┬─────────┘                       │
│                               │                                 │
│                               ▼                                 │
│                      ┌──────────────────┐                       │
│                      │  edgequake-core  │  ◀── Orchestration    │
│                      │  (EdgeQuake API) │                       │
│                      └────────┬─────────┘                       │
│                               │                                 │
│              ┌────────────────┼────────────────┐                │
│              │                │                │                │
│              ▼                ▼                ▼                │
│    ┌─────────────────┐  ┌────────────┐  ┌─────────────┐         │
│    │edgequake-pipeline│  │edgequake-  │  │edgequake-   │        │
│    │(Document proc.) │  │   query    │  │  storage    │         │
│    └────────┬────────┘  └─────┬──────┘  └──────┬──────┘         │
│             │                 │                 │               │
│             ▼                 │                 │               │
│    ┌─────────────────┐        │                 │               │
│    │  edgequake-pdf  │        │                 │               │
│    │ (PDF extraction)│        │                 │               │
│    └─────────────────┘        │                 │               │
│                               │                 │               │
│              ┌────────────────┴─────────────────┘               │
│              │                                                  │
│              ▼                                                  │
│    ┌─────────────────┐                                          │
│    │  edgequake-llm  │  ◀── LLM Abstraction                     │
│    │(OpenAI, Ollama) │                                          │
│    └─────────────────┘                                          │
│                                                                 │
│    Supporting Crates:                                           │
│    ┌────────────────┐ ┌────────────────┐ ┌────────────────┐     │
│    │edgequake-auth  │ │edgequake-tasks │ │edgequake-audit │     │
│    └────────────────┘ └────────────────┘ └────────────────┘     │
│    ┌────────────────┐                                           │
│    │edgequake-rate- │                                           │
│    │    limiter     │                                           │
│    └────────────────┘                                           │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Core Crates

### edgequake-core

**The orchestration layer and public API.**

| Attribute    | Value                         |
| ------------ | ----------------------------- |
| Path         | `crates/edgequake-core`       |
| Lines        | ~8,000                        |
| Dependencies | pipeline, query, storage, llm |

**Key Types:**

```rust
// Main entry point
pub struct EdgeQuake {
    orchestrator: Orchestrator,
    config: EdgeQuakeConfig,
}

// Configuration
pub struct EdgeQuakeConfig {
    pub storage_mode: StorageMode,
    pub llm_config: LLMConfig,
    pub pipeline_config: PipelineConfig,
}

// Orchestrator - coordinates all components
pub struct Orchestrator {
    pipeline: DocumentPipeline,
    query_engine: QueryEngine,
    storage: Arc<dyn GraphStorage>,
}
```

**Responsibilities:**

- Document ingestion orchestration
- Query processing coordination
- Configuration management
- Component lifecycle

---

### edgequake-api

**HTTP/REST API server built with Axum.**

| Attribute | Value                  |
| --------- | ---------------------- |
| Path      | `crates/edgequake-api` |
| Lines     | ~5,000                 |
| Framework | Axum 0.8               |

**Key Types:**

```rust
// Application state
pub struct AppState {
    edgequake: Arc<EdgeQuake>,
    config: ApiConfig,
}

// Route handlers
pub mod handlers {
    pub mod documents;   // Document upload, list, delete
    pub mod query;       // Query execution
    pub mod chat;        // Chat interface
    pub mod graph;       // Graph exploration
    pub mod ollama;      // Ollama API emulation
    pub mod workspaces;  // Multi-tenancy
}
```

**Endpoints:**

- `/health` - Health check
- `/api/v1/documents/*` - Document management
- `/api/v1/query` - Query execution
- `/api/v1/chat/*` - Chat interface
- `/api/v1/graph/*` - Graph exploration
- `/api/tags`, `/api/chat` - Ollama emulation

---

### edgequake-pipeline

**Document processing pipeline.**

| Attribute | Value                           |
| --------- | ------------------------------- |
| Path      | `crates/edgequake-pipeline`     |
| Lines     | ~12,000                         |
| Features  | Chunking, extraction, embedding |

**Key Types:**

```rust
// Document pipeline
pub struct DocumentPipeline {
    chunker: Chunker,
    extractor: Arc<dyn EntityExtractor>,
    embedder: Arc<dyn EmbeddingProvider>,
}

// Pipeline configuration
pub struct PipelineConfig {
    pub chunk_size: usize,
    pub chunk_overlap: usize,
    pub enable_entity_extraction: bool,
    pub enable_relationship_extraction: bool,
}

// Extraction result
pub struct ExtractionResult {
    pub entities: Vec<ExtractedEntity>,
    pub relationships: Vec<ExtractedRelationship>,
    pub source_chunk_id: String,
}
```

**Sub-modules:**

- `chunker` - Document chunking strategies
- `extractor` - Entity/relationship extraction
- `prompts` - LLM prompt templates
- `lineage` - Processing provenance tracking

---

### edgequake-query

**Query engine for knowledge graph retrieval.**

| Attribute | Value                    |
| --------- | ------------------------ |
| Path      | `crates/edgequake-query` |
| Lines     | ~6,000                   |
| Features  | Multi-mode retrieval     |

**Key Types:**

```rust
// Query engine
pub struct QueryEngine<V, G> {
    vector_storage: Arc<V>,
    graph_storage: Arc<G>,
    llm_provider: Arc<dyn LLMProvider>,
}

// Query modes
pub enum QueryMode {
    Naive,   // Vector only
    Local,   // Entity neighborhood
    Global,  // Relationship-focused
    Hybrid,  // Combined (default)
    Mix,     // Weighted blend
}

// Query context
pub struct QueryContext {
    pub chunks: Vec<RetrievedChunk>,
    pub entities: Vec<RetrievedEntity>,
    pub relationships: Vec<RetrievedRelationship>,
}
```

**Strategies:**

- `NaiveStrategy` - Pure vector search
- `LocalStrategy` - Entity + 1-hop neighborhood
- `GlobalStrategy` - Relationship-focused
- `HybridStrategy` - Local + Global combined
- `MixStrategy` - Weighted combination

---

### edgequake-storage

**Storage abstractions and implementations.**

| Attribute | Value                      |
| --------- | -------------------------- |
| Path      | `crates/edgequake-storage` |
| Lines     | ~10,000                    |
| Backends  | Memory, PostgreSQL/AGE     |

**Key Traits:**

```rust
// Vector storage abstraction
#[async_trait]
pub trait VectorStorage: Send + Sync {
    async fn insert(&self, id: &str, vector: &[f32], metadata: Value) -> Result<()>;
    async fn query(&self, vector: &[f32], top_k: usize, filter: Option<Filter>) -> Result<Vec<VectorResult>>;
    async fn delete(&self, id: &str) -> Result<()>;
}

// Graph storage abstraction
#[async_trait]
pub trait GraphStorage: Send + Sync {
    async fn create_node(&self, node: &Node) -> Result<()>;
    async fn create_edge(&self, edge: &Edge) -> Result<()>;
    async fn get_node(&self, id: &str) -> Result<Option<Node>>;
    async fn get_node_edges(&self, id: &str) -> Result<Vec<Edge>>;
    async fn query_nodes(&self, query: &str) -> Result<Vec<Node>>;
}
```

**Implementations:**

- `MemoryStorage` - In-memory (development)
- `PostgresVectorStorage` - pgvector
- `AgeGraphStorage` - Apache AGE

---

### edgequake-llm

**LLM provider abstraction layer.**

| Attribute | Value                  |
| --------- | ---------------------- |
| Path      | `crates/edgequake-llm` |
| Lines     | ~4,000                 |
| Providers | OpenAI, Ollama, Mock   |

**Key Traits:**

```rust
// LLM provider abstraction
#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn chat(&self, messages: &[ChatMessage], options: Option<&CompletionOptions>) -> Result<LLMResponse>;
    async fn complete(&self, prompt: &str) -> Result<LLMResponse>;
    fn model(&self) -> &str;
    fn name(&self) -> &str;
}

// Embedding provider abstraction
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
    async fn embed_one(&self, text: &str) -> Result<Vec<f32>>;
    fn dimensions(&self) -> usize;
}
```

**Implementations:**

- `OpenAIProvider` - OpenAI GPT models
- `OllamaProvider` - Local Ollama models
- `MockProvider` - Testing (no API calls)

---

### edgequake-pdf

**PDF extraction and parsing.**

| Attribute | Value                  |
| --------- | ---------------------- |
| Path      | `crates/edgequake-pdf` |
| Lines     | ~8,000                 |
| Features  | Text, tables, images   |

**Key Types:**

```rust
// PDF extractor
pub struct PdfExtractor {
    config: ExtractorConfig,
    processors: ProcessorChain,
}

// Extraction result
pub struct PdfDocument {
    pub pages: Vec<PdfPage>,
    pub metadata: PdfMetadata,
    pub text_content: String,
}

// Processor chain
pub struct ProcessorChain {
    processors: Vec<Box<dyn Processor>>,
}
```

**Processors:**

- `TextProcessor` - Text extraction
- `TableProcessor` - Table detection
- `StyleProcessor` - Font/formatting
- `LLMEnhanceProcessor` - AI-enhanced extraction

---

## Supporting Crates

### edgequake-auth

**Authentication and authorization.**

| Attribute | Value                   |
| --------- | ----------------------- |
| Path      | `crates/edgequake-auth` |
| Lines     | ~1,500                  |
| Features  | API keys, JWT           |

**Key Types:**

```rust
// Authentication middleware
pub struct AuthMiddleware {
    config: AuthConfig,
}

// API key validation
pub struct ApiKeyValidator {
    keys: HashSet<String>,
}

// Workspace authorization
pub struct WorkspaceAuth {
    workspace_id: String,
    permissions: Permissions,
}
```

---

### edgequake-tasks

**Background task management.**

| Attribute | Value                    |
| --------- | ------------------------ |
| Path      | `crates/edgequake-tasks` |
| Lines     | ~2,000                   |
| Features  | Async tasks, queues      |

**Key Types:**

```rust
// Task manager
pub struct TaskManager {
    queue: TaskQueue,
    workers: Vec<Worker>,
}

// Task definition
pub struct Task {
    pub id: Uuid,
    pub task_type: TaskType,
    pub status: TaskStatus,
    pub created_at: DateTime<Utc>,
}

// Task types
pub enum TaskType {
    DocumentProcessing(DocumentId),
    Reindexing(WorkspaceId),
    Cleanup,
}
```

---

### edgequake-rate-limiter

**Rate limiting for API protection.**

| Attribute | Value                           |
| --------- | ------------------------------- |
| Path      | `crates/edgequake-rate-limiter` |
| Lines     | ~800                            |
| Algorithm | Token bucket                    |

**Key Types:**

```rust
// Rate limiter
pub struct RateLimiter {
    config: RateLimitConfig,
    buckets: DashMap<String, TokenBucket>,
}

// Configuration
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub burst_size: u32,
}
```

---

### edgequake-audit

**Audit logging and compliance.**

| Attribute | Value                    |
| --------- | ------------------------ |
| Path      | `crates/edgequake-audit` |
| Lines     | ~1,000                   |
| Features  | Event logging            |

**Key Types:**

```rust
// Audit logger
pub struct AuditLogger {
    sink: Box<dyn AuditSink>,
}

// Audit event
pub struct AuditEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: EventType,
    pub actor: String,
    pub resource: String,
    pub action: String,
}
```

---

## Crate Size Summary

| Crate                  | Lines       | Purpose             |
| ---------------------- | ----------- | ------------------- |
| edgequake-pipeline     | ~12,000     | Document processing |
| edgequake-storage      | ~10,000     | Storage backends    |
| edgequake-pdf          | ~8,000      | PDF extraction      |
| edgequake-core         | ~8,000      | Orchestration       |
| edgequake-query        | ~6,000      | Query engine        |
| edgequake-api          | ~5,000      | HTTP server         |
| edgequake-llm          | ~4,000      | LLM abstraction     |
| edgequake-tasks        | ~2,000      | Background tasks    |
| edgequake-auth         | ~1,500      | Authentication      |
| edgequake-audit        | ~1,000      | Audit logging       |
| edgequake-rate-limiter | ~800        | Rate limiting       |
| **Total**              | **~58,300** | Core functionality  |

---

## Feature Flags

Key feature flags across crates:

| Flag       | Crate    | Description               |
| ---------- | -------- | ------------------------- |
| `postgres` | storage  | Enable PostgreSQL backend |
| `memory`   | storage  | Enable in-memory backend  |
| `openai`   | llm      | Enable OpenAI provider    |
| `ollama`   | llm      | Enable Ollama provider    |
| `pdf`      | pipeline | Enable PDF processing     |

---

## Adding a New Crate

1. Create crate directory: `cargo new --lib crates/edgequake-new`
2. Add to workspace: Edit root `Cargo.toml`
3. Define public API in `src/lib.rs`
4. Add tests in `tests/`
5. Document in this reference

---

## See Also

- [Architecture Overview](./overview.md) - High-level design
- [Data Flow](./data-flow.md) - How data moves through the system
- [REST API Reference](../api-reference/rest-api.md) - HTTP endpoints
