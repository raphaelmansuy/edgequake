# Phase 1: Component Mapping

**Phase Duration**: Weeks 1-2  
**Owner**: Lead Architect  
**Status**: 🔴 Not Started

---

## Objective

Map all Python LightRAG components to their Rust EdgeQuake equivalents, establishing the foundational type system and project structure that will guide all subsequent implementation work.

---

## Reference Documentation

| Document | Purpose |
|----------|---------|
| [docs_retro/01-executive-summary.md](../../docs_retro/01-executive-summary.md) | System overview |
| [docs_retro/02-architecture.md](../../docs_retro/02-architecture.md) | Component relationships |
| [docs_retro/03-domain-model.md](../../docs_retro/03-domain-model.md) | Entity definitions |
| [docs_retro/06-storage-contracts.md](../../docs_retro/06-storage-contracts.md) | Storage interfaces |
| [docs_retro/07-external-integrations.md](../../docs_retro/07-external-integrations.md) | LLM/Embedding contracts |
| [tech_stack/technology_choice.md](../../tech_stack/technology_choice.md) | Technology decisions |
| [tech_stack/README.md](../../tech_stack/README.md) | Project structure |

---

## Deliverables

### 1. Component Mapping Matrix

#### Python Files → Rust Crates

| Python Source | Lines | Rust Crate | Purpose |
|---------------|-------|------------|---------|
| `lightrag/lightrag.py` | ~3700 | `edgequake-core` | Orchestrator, main API |
| `lightrag/operate.py` | ~800 | `edgequake-pipeline`, `edgequake-query` | Pipeline & query logic |
| `lightrag/base.py` | ~300 | `edgequake-storage` | Storage traits |
| `lightrag/types.py` | ~150 | `edgequake-core/types.rs` | Type definitions |
| `lightrag/constants.py` | ~50 | `edgequake-core/config.rs` | Configuration constants |
| `lightrag/kg/*_impl.py` | ~2000 | `edgequake-storage/adapters/` | Storage implementations |
| `lightrag/llm/*.py` | ~1500 | `edgequake-llm` | LLM providers |
| `lightrag/api/*.py` | ~1200 | `edgequake-api` | REST API |
| `lightrag/utils*.py` | ~500 | `edgequake-core/utils/` | Utility functions |

#### Storage Instance Mapping

| Python Instance | Python Type | Rust Type | Database |
|----------------|-------------|-----------|----------|
| `full_docs` | `BaseKVStorage` | `KVStorage<Document>` | PostgreSQL / SurrealDB |
| `doc_status` | `DocStatusStorage` | `DocStatusStorage` | PostgreSQL / SurrealDB |
| `text_chunks` | `BaseKVStorage` | `KVStorage<Chunk>` | PostgreSQL / SurrealDB |
| `llm_response_cache` | `BaseKVStorage` | `KVStorage<LLMCache>` | PostgreSQL / SurrealDB |
| `full_entities` | `BaseKVStorage` | Embedded in Entity | Graph relation |
| `full_relations` | `BaseKVStorage` | Embedded in Relation | Graph relation |
| `entity_chunks` | `BaseKVStorage` | Embedded in Entity | Graph relation |
| `relation_chunks` | `BaseKVStorage` | Embedded in Relation | Graph relation |
| `chunk_entity_relation_graph` | `BaseGraphStorage` | `GraphStorage` | AGE / SurrealDB |
| `entities_vdb` | `BaseVectorStorage` | `VectorStorage<Entity>` | pgvector / SurrealDB |
| `relationships_vdb` | `BaseVectorStorage` | `VectorStorage<Relation>` | pgvector / SurrealDB |
| `chunks_vdb` | `BaseVectorStorage` | `VectorStorage<Chunk>` | pgvector / SurrealDB |

---

### 2. Crate Structure

```
edgequake/
├── Cargo.toml                    # Workspace manifest
├── rust-toolchain.toml           # Rust version pinning
├── .cargo/config.toml            # Build configuration
│
├── crates/
│   ├── edgequake-core/           # Core types and orchestrator
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── types/
│   │       │   ├── mod.rs
│   │       │   ├── document.rs
│   │       │   ├── chunk.rs
│   │       │   ├── entity.rs
│   │       │   ├── relationship.rs
│   │       │   └── embedding.rs
│   │       ├── config.rs
│   │       ├── error.rs
│   │       └── utils/
│   │           ├── mod.rs
│   │           ├── hash.rs
│   │           └── text.rs
│   │
│   ├── edgequake-storage/        # Storage abstractions and adapters
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── traits/
│   │       │   ├── mod.rs
│   │       │   ├── kv.rs
│   │       │   ├── vector.rs
│   │       │   └── graph.rs
│   │       ├── adapters/
│   │       │   ├── mod.rs
│   │       │   ├── memory.rs       # In-memory (testing)
│   │       │   ├── postgres/
│   │       │   │   ├── mod.rs
│   │       │   │   ├── kv.rs
│   │       │   │   ├── vector.rs   # pgvector
│   │       │   │   └── graph.rs    # AGE
│   │       │   └── surrealdb/
│   │       │       ├── mod.rs
│   │       │       └── unified.rs
│   │       └── error.rs
│   │
│   ├── edgequake-llm/            # LLM provider abstractions
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── traits.rs
│   │       ├── providers/
│   │       │   ├── mod.rs
│   │       │   ├── openai.rs
│   │       │   ├── anthropic.rs
│   │       │   └── ollama.rs
│   │       ├── embedding.rs
│   │       └── error.rs
│   │
│   ├── edgequake-pipeline/       # Document processing pipeline
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── chunking.rs
│   │       ├── extraction.rs
│   │       ├── merging.rs
│   │       └── embedding.rs
│   │
│   ├── edgequake-query/          # Query processing engine
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── modes/
│   │       │   ├── mod.rs
│   │       │   ├── naive.rs
│   │       │   ├── local.rs
│   │       │   ├── global.rs
│   │       │   └── hybrid.rs
│   │       ├── context.rs
│   │       └── response.rs
│   │
│   └── edgequake-api/            # REST API server
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           ├── lib.rs
│           ├── routes/
│           │   ├── mod.rs
│           │   ├── documents.rs
│           │   ├── query.rs
│           │   └── graph.rs
│           ├── middleware/
│           │   ├── mod.rs
│           │   ├── auth.rs
│           │   └── tracing.rs
│           ├── openapi.rs
│           └── error.rs
│
├── examples/
│   ├── simple_insert.rs
│   ├── query_modes.rs
│   └── custom_provider.rs
│
├── tests/
│   └── integration/
│       ├── document_lifecycle.rs
│       ├── query_modes.rs
│       └── storage_adapters.rs
│
└── benches/
    ├── chunking.rs
    ├── query.rs
    └── storage.rs
```

---

### 3. Core Type Definitions

#### Document
```rust
// edgequake-core/src/types/document.rs
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Document processing status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DocumentStatus {
    Pending,
    Processing,
    Processed,
    Failed,
}

/// A document to be processed into the knowledge graph
/// Reference: docs_retro/03-domain-model.md#entity-document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// MD5 hash of content - primary key
    pub id: String,
    /// Raw text content
    pub content: String,
    /// Source file path (optional)
    pub file_path: Option<String>,
    /// Processing status
    pub status: DocumentStatus,
    /// Batch tracking ID
    pub track_id: Option<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Number of chunks generated
    pub chunks_count: Option<u32>,
    /// Error message if failed
    pub error: Option<String>,
}

impl Document {
    /// Generate document ID from content (MD5 hash)
    pub fn generate_id(content: &str) -> String {
        use md5::{Md5, Digest};
        let mut hasher = Md5::new();
        hasher.update(content.as_bytes());
        format!("doc-{:x}", hasher.finalize())
    }
    
    /// Create new document with PENDING status
    pub fn new(content: String, file_path: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Self::generate_id(&content),
            content,
            file_path,
            status: DocumentStatus::Pending,
            track_id: None,
            created_at: now,
            updated_at: now,
            chunks_count: None,
            error: None,
        }
    }
}
```

#### Chunk
```rust
// edgequake-core/src/types/chunk.rs
use serde::{Deserialize, Serialize};

/// A segment of a document
/// Reference: docs_retro/03-domain-model.md#entity-chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    /// MD5 hash of content - primary key
    pub id: String,
    /// Chunk text content
    pub content: String,
    /// Token count
    pub tokens: u32,
    /// Position in document
    pub chunk_order_index: u32,
    /// Parent document ID
    pub full_doc_id: String,
    /// Source file path
    pub file_path: Option<String>,
}

impl Chunk {
    /// Generate chunk ID from content
    pub fn generate_id(content: &str) -> String {
        use md5::{Md5, Digest};
        let mut hasher = Md5::new();
        hasher.update(content.as_bytes());
        format!("chunk-{:x}", hasher.finalize())
    }
}
```

#### GraphEntity
```rust
// edgequake-core/src/types/entity.rs
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// An entity extracted from text
/// Reference: docs_retro/03-domain-model.md#entity-graphentity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEntity {
    /// Entity name (uppercase normalized) - primary key
    pub id: String,
    /// Display name
    pub entity_name: String,
    /// Entity type (person, organization, etc.)
    pub entity_type: String,
    /// Aggregated description from all mentions
    pub description: String,
    /// Pipe-separated chunk IDs
    pub source_id: String,
    /// Source file paths
    pub file_path: Option<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

impl GraphEntity {
    /// Normalize entity name for consistent storage
    pub fn normalize_name(name: &str) -> String {
        name.trim().to_uppercase()
    }
    
    /// Generate entity ID from name
    pub fn generate_id(name: &str) -> String {
        Self::normalize_name(name)
    }
}
```

#### GraphRelationship
```rust
// edgequake-core/src/types/relationship.rs
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Separator for relationship IDs
pub const RELATIONSHIP_SEP: &str = "<SEP>";

/// A relationship between two entities
/// Reference: docs_retro/03-domain-model.md#entity-graphrelationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphRelationship {
    /// Composite key: "entity1<SEP>entity2" (alphabetically sorted)
    pub id: String,
    /// Source entity name
    pub source_entity: String,
    /// Target entity name
    pub target_entity: String,
    /// Relationship description
    pub description: String,
    /// Keywords describing relationship
    pub keywords: Option<String>,
    /// Relationship weight/strength
    pub weight: f32,
    /// Pipe-separated chunk IDs
    pub source_id: String,
    /// Source file path
    pub file_path: Option<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

impl GraphRelationship {
    /// Generate relationship ID (sorted alphabetically)
    pub fn generate_id(source: &str, target: &str) -> String {
        let normalized_source = source.trim().to_uppercase();
        let normalized_target = target.trim().to_uppercase();
        
        // Sort alphabetically for consistent key regardless of direction
        if normalized_source <= normalized_target {
            format!("{}{}{}", normalized_source, RELATIONSHIP_SEP, normalized_target)
        } else {
            format!("{}{}{}", normalized_target, RELATIONSHIP_SEP, normalized_source)
        }
    }
}
```

#### Embedding
```rust
// edgequake-core/src/types/embedding.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Vector representation of text
/// Reference: docs_retro/03-domain-model.md#entity-embedding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    /// Unique identifier
    pub id: String,
    /// Dense vector representation
    pub vector: Vec<f32>,
    /// Original text that was embedded
    pub content: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Embedding function configuration
pub struct EmbeddingConfig {
    /// Vector dimension (e.g., 1536 for OpenAI)
    pub embedding_dim: usize,
    /// Maximum tokens per text
    pub max_token_size: usize,
}
```

---

### 4. Storage Trait Definitions

#### KVStorage Trait
```rust
// edgequake-storage/src/traits/kv.rs
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashSet;
use crate::error::StorageError;

/// Key-value storage interface
/// Reference: docs_retro/06-storage-contracts.md#interface-basekvstorage
#[async_trait]
pub trait KVStorage: Send + Sync {
    /// Storage namespace
    fn namespace(&self) -> &str;
    
    /// Initialize storage (create tables, indices)
    async fn initialize(&self) -> Result<(), StorageError>;
    
    /// Flush changes to persistent storage
    async fn finalize(&self) -> Result<(), StorageError>;
    
    /// Retrieve single record by ID
    async fn get_by_id<T: DeserializeOwned + Send>(
        &self,
        id: &str,
    ) -> Result<Option<T>, StorageError>;
    
    /// Batch retrieve records by IDs
    async fn get_by_ids<T: DeserializeOwned + Send>(
        &self,
        ids: &[String],
    ) -> Result<Vec<T>, StorageError>;
    
    /// Return keys that do NOT exist in storage
    async fn filter_keys(&self, keys: HashSet<String>) -> Result<HashSet<String>, StorageError>;
    
    /// Insert or update multiple records
    async fn upsert<T: Serialize + Send + Sync>(
        &self,
        data: &[(String, T)],
    ) -> Result<(), StorageError>;
    
    /// Delete records by IDs
    async fn delete(&self, ids: &[String]) -> Result<(), StorageError>;
    
    /// Check if storage is empty
    async fn is_empty(&self) -> Result<bool, StorageError>;
}
```

#### VectorStorage Trait
```rust
// edgequake-storage/src/traits/vector.rs
use async_trait::async_trait;
use crate::error::StorageError;

/// Vector search result
#[derive(Debug, Clone)]
pub struct VectorSearchResult {
    pub id: String,
    pub score: f32,
    pub metadata: serde_json::Value,
}

/// Vector storage interface
/// Reference: docs_retro/06-storage-contracts.md#interface-basevectorstorage
#[async_trait]
pub trait VectorStorage: Send + Sync {
    /// Storage namespace
    fn namespace(&self) -> &str;
    
    /// Initialize storage
    async fn initialize(&self) -> Result<(), StorageError>;
    
    /// Flush changes
    async fn finalize(&self) -> Result<(), StorageError>;
    
    /// Similarity search
    async fn query(
        &self,
        query_embedding: &[f32],
        top_k: usize,
        filter_ids: Option<&[String]>,
    ) -> Result<Vec<VectorSearchResult>, StorageError>;
    
    /// Insert or update vectors with metadata
    async fn upsert(
        &self,
        data: &[(String, Vec<f32>, serde_json::Value)], // (id, vector, metadata)
    ) -> Result<(), StorageError>;
    
    /// Delete vectors by IDs
    async fn delete(&self, ids: &[String]) -> Result<(), StorageError>;
    
    /// Delete entity and related data
    async fn delete_entity(&self, entity_name: &str) -> Result<(), StorageError>;
    
    /// Delete all relationships involving an entity
    async fn delete_entity_relations(&self, entity_name: &str) -> Result<(), StorageError>;
    
    /// Get vector by ID
    async fn get_by_id(&self, id: &str) -> Result<Option<Vec<f32>>, StorageError>;
    
    /// Batch get vectors
    async fn get_by_ids(&self, ids: &[String]) -> Result<Vec<(String, Vec<f32>)>, StorageError>;
}
```

#### GraphStorage Trait
```rust
// edgequake-storage/src/traits/graph.rs
use async_trait::async_trait;
use crate::error::StorageError;
use std::collections::HashMap;

/// Knowledge graph node
#[derive(Debug, Clone)]
pub struct GraphNode {
    pub id: String,
    pub properties: HashMap<String, serde_json::Value>,
}

/// Knowledge graph edge
#[derive(Debug, Clone)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub properties: HashMap<String, serde_json::Value>,
}

/// Subgraph result
#[derive(Debug, Clone)]
pub struct KnowledgeGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub is_truncated: bool,
}

/// Graph storage interface
/// Reference: docs_retro/06-storage-contracts.md#interface-basegraphstorage
#[async_trait]
pub trait GraphStorage: Send + Sync {
    /// Storage namespace
    fn namespace(&self) -> &str;
    
    /// Initialize storage
    async fn initialize(&self) -> Result<(), StorageError>;
    
    /// Flush changes
    async fn finalize(&self) -> Result<(), StorageError>;
    
    // Node operations
    async fn has_node(&self, node_id: &str) -> Result<bool, StorageError>;
    async fn get_node(&self, node_id: &str) -> Result<Option<GraphNode>, StorageError>;
    async fn upsert_node(&self, node_id: &str, properties: HashMap<String, serde_json::Value>) -> Result<(), StorageError>;
    async fn delete_node(&self, node_id: &str) -> Result<(), StorageError>;
    async fn node_degree(&self, node_id: &str) -> Result<usize, StorageError>;
    async fn get_all_nodes(&self) -> Result<Vec<GraphNode>, StorageError>;
    
    // Edge operations
    async fn has_edge(&self, source: &str, target: &str) -> Result<bool, StorageError>;
    async fn get_edge(&self, source: &str, target: &str) -> Result<Option<GraphEdge>, StorageError>;
    async fn upsert_edge(&self, source: &str, target: &str, properties: HashMap<String, serde_json::Value>) -> Result<(), StorageError>;
    async fn delete_edge(&self, source: &str, target: &str) -> Result<(), StorageError>;
    async fn get_node_edges(&self, node_id: &str) -> Result<Vec<GraphEdge>, StorageError>;
    async fn get_all_edges(&self) -> Result<Vec<GraphEdge>, StorageError>;
    
    // Query operations
    async fn get_knowledge_graph(
        &self,
        start_node: &str,
        max_depth: usize,
        max_nodes: usize,
    ) -> Result<KnowledgeGraph, StorageError>;
    
    async fn get_popular_labels(&self, limit: usize) -> Result<Vec<String>, StorageError>;
    async fn search_labels(&self, query: &str, limit: usize) -> Result<Vec<String>, StorageError>;
}
```

---

### 5. LLM Provider Trait

```rust
// edgequake-llm/src/traits.rs
use async_trait::async_trait;
use crate::error::LLMError;

/// Chat message role
#[derive(Debug, Clone)]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

/// Chat message
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
}

/// LLM completion options
#[derive(Debug, Clone, Default)]
pub struct CompletionOptions {
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub stream: bool,
}

/// Streaming response chunk
pub struct StreamChunk {
    pub content: String,
    pub is_done: bool,
}

/// LLM provider interface
/// Reference: docs_retro/07-external-integrations.md#llm-provider-interface
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Generate chat completion
    async fn chat_completion(
        &self,
        messages: Vec<ChatMessage>,
        options: CompletionOptions,
    ) -> Result<String, LLMError>;
    
    /// Streaming chat completion
    async fn chat_completion_stream(
        &self,
        messages: Vec<ChatMessage>,
        options: CompletionOptions,
    ) -> Result<Box<dyn futures::Stream<Item = Result<StreamChunk, LLMError>> + Send + Unpin>, LLMError>;
}

/// Embedding provider interface
/// Reference: docs_retro/07-external-integrations.md#embedding-provider-interface
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Get embedding dimension
    fn embedding_dim(&self) -> usize;
    
    /// Maximum tokens per text
    fn max_token_size(&self) -> usize;
    
    /// Generate embeddings for texts
    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, LLMError>;
}
```

---

## Week-by-Week Tasks

### Week 1: Analysis & Design

| Task | Description | Owner | Status |
|------|-------------|-------|--------|
| 1.1.1 | Complete Python→Rust component mapping | Architect | ⬜ |
| 1.1.2 | Design workspace structure | Architect | ⬜ |
| 1.1.3 | Define core type hierarchy | Architect | ⬜ |
| 1.1.4 | Create Cargo.toml manifests | Architect | ⬜ |
| 1.1.5 | Set up CI/CD pipeline | DevOps | ⬜ |
| 1.1.6 | Document design decisions | Architect | ⬜ |

### Week 2: Implementation & Validation

| Task | Description | Owner | Status |
|------|-------------|-------|--------|
| 1.2.1 | Implement Document struct | Backend | ⬜ |
| 1.2.2 | Implement Chunk struct | Backend | ⬜ |
| 1.2.3 | Implement GraphEntity struct | Backend | ⬜ |
| 1.2.4 | Implement GraphRelationship struct | Backend | ⬜ |
| 1.2.5 | Implement Embedding struct | Backend | ⬜ |
| 1.2.6 | Define KVStorage trait | Backend | ⬜ |
| 1.2.7 | Define VectorStorage trait | Backend | ⬜ |
| 1.2.8 | Define GraphStorage trait | Backend | ⬜ |
| 1.2.9 | Define LLMProvider trait | Backend | ⬜ |
| 1.2.10 | Define EmbeddingProvider trait | Backend | ⬜ |
| 1.2.11 | Write unit tests for types | QA | ⬜ |
| 1.2.12 | Documentation review | Tech Writer | ⬜ |

---

## Acceptance Criteria

- [ ] All 12 storage instances mapped to EdgeQuake equivalents
- [ ] Workspace compiles with `cargo build --all-targets`
- [ ] Core types have serde Serialize/Deserialize
- [ ] All traits are object-safe with async methods
- [ ] Unit tests pass for ID generation
- [ ] Documentation covers all public types
- [ ] Design review completed and approved

---

## Dependencies

### External Dependencies

```toml
[workspace.dependencies]
tokio = { version = "1.35", features = ["full"] }
async-trait = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
chrono = { version = "0.4", features = ["serde"] }
md5 = "0.7"
thiserror = "1.0"
anyhow = "1.0"
futures = "0.3"
```

### Internal Dependencies

- Phase 2 depends on: trait definitions, core types
- Phase 3 depends on: storage adapters, LLM providers

---

## Risks & Mitigations

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Trait design changes later | Medium | Review with senior team, keep traits minimal |
| Missing Python behaviors | Medium | Cross-reference docs_retro/ carefully |
| Async complexity | Low | Use established patterns from async-book |

---

## Related Documents

- [master.md](../master.md) - Overall plan
- [Phase 2: Migration Strategy](phase-2-migration-strategy.md) - Next phase
- [craft_pad.md](../craft_pad.md) - Working notes
