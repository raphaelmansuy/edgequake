# EdgeQuake Storage Backends

> **Implements**: [FEAT0030](features.md#feat0030) Storage Abstraction, [FEAT0031](features.md#feat0031) PostgreSQL Backend
>
> Complete guide to storage backend configuration and implementation

**Version**: 2.0.0 | **Last Updated**: January 2026

> **Code Reference**: See [edgequake/crates/edgequake-storage/](../edgequake/crates/edgequake-storage/) for all storage implementations

---

## Quick Decision Guide

| Environment     | Recommended Stack           | Persistence      | Performance              |
| --------------- | --------------------------- | ---------------- | ------------------------ |
| **Development** | Memory                      | None (ephemeral) | ⚡ Fastest               |
| **Testing/CI**  | Memory                      | None (ephemeral) | ⚡ Fastest               |
| **Staging**     | PostgreSQL + pgvector       | Full             | 🔄 Good                  |
| **Production**  | PostgreSQL + pgvector + AGE | Full             | 🔄 Good + Graph features |

> **Enforces**: [BR0001](business_rules.md#br0001) Tenant Isolation - All storage is namespace-scoped

---

## Table of Contents

1. [Overview](#overview)
2. [Storage Traits](#storage-traits)
3. [Memory Storage](#memory-storage)
4. [PostgreSQL Storage](#postgresql-storage)
5. [Configuration Reference](#configuration-reference)
6. [Migration Guide](#migration-guide)

---

## Overview

EdgeQuake uses three types of storage, each with pluggable backend implementations:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                      Storage Architecture                               │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                    EdgeQuake Core                                │   │
│  └───────────────────────────┬─────────────────────────────────────┘   │
│                              │                                          │
│          ┌───────────────────┼───────────────────┐                     │
│          │                   │                   │                     │
│          ▼                   ▼                   ▼                     │
│  ┌───────────────┐   ┌───────────────┐   ┌───────────────┐            │
│  │   KV Storage  │   │ Vector Store  │   │  Graph Store  │            │
│  │   (Documents, │   │  (Embeddings) │   │   (KG Nodes   │            │
│  │    Chunks,    │   │               │   │    & Edges)   │            │
│  │    Cache)     │   │               │   │               │            │
│  └───────┬───────┘   └───────┬───────┘   └───────┬───────┘            │
│          │                   │                   │                     │
│          ▼                   ▼                   ▼                     │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                   Backend Implementations                        │   │
│  │                                                                  │   │
│  │       Memory (Development)  │  PostgreSQL (Production)          │   │
│  │                             │  + pgvector + AGE                 │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Storage Traits

EdgeQuake defines abstract traits for each storage type, allowing pluggable backends.

### KVStorage Trait

Key-value storage for documents, chunks, and metadata.

```rust
// Located: edgequake/crates/edgequake-storage/src/traits/kv.rs

#[async_trait]
pub trait KVStorage: Send + Sync {
    /// Get the storage namespace.
    fn namespace(&self) -> &str;

    /// Initialize the storage backend.
    async fn initialize(&self) -> Result<()>;

    /// Flush pending changes.
    async fn finalize(&self) -> Result<()>;

    /// Get a value by ID.
    async fn get_by_id(&self, id: &str) -> Result<Option<serde_json::Value>>;

    /// Get multiple values by IDs.
    async fn get_by_ids(&self, ids: &[String]) -> Result<Vec<serde_json::Value>>;

    /// Filter keys to find which do NOT exist in storage.
    async fn filter_keys(&self, keys: HashSet<String>) -> Result<HashSet<String>>;

    /// Upsert key-value pairs.
    async fn upsert(&self, data: &[(String, serde_json::Value)]) -> Result<()>;

    /// Delete by IDs.
    async fn delete(&self, ids: &[String]) -> Result<()>;
}
```

**Usage:**

```rust
// Store document metadata
let metadata = serde_json::json!({
    "title": "Document Title",
    "created_at": "2025-12-24T14:30:00Z"
});
kv_storage.upsert(&[("doc-123-metadata".to_string(), metadata)]).await?;

// Retrieve using get_by_id
let doc = kv_storage.get_by_id("doc-123-metadata").await?;

// Retrieve multiple documents using get_by_ids
let docs = kv_storage.get_by_ids(&["doc-123".into(), "doc-456".into()]).await?;
```

### VectorStorage Trait

Vector storage for embeddings and similarity search.

```rust
// Located: edgequake/crates/edgequake-storage/src/traits/vector.rs

#[async_trait]
pub trait VectorStorage: Send + Sync {
    /// Get the storage namespace.
    fn namespace(&self) -> &str;

    /// Get the expected embedding dimension.
    fn dimension(&self) -> usize;

    /// Initialize the vector storage.
    async fn initialize(&self) -> Result<()>;

    /// Flush pending changes.
    async fn finalize(&self) -> Result<()>;

    /// Perform similarity search.
    async fn query(
        &self,
        query_embedding: &[f32],
        top_k: usize,
        filter_ids: Option<&[String]>,
    ) -> Result<Vec<VectorSearchResult>>;

    /// Insert or update vectors with metadata.
    async fn upsert(
        &self,
        data: &[(String, Vec<f32>, serde_json::Value)],
    ) -> Result<()>;

    /// Delete vectors by IDs.
    async fn delete(&self, ids: &[String]) -> Result<()>;

    /// Get vector by ID.
    async fn get_by_id(&self, id: &str) -> Result<Option<Vec<f32>>>;

    /// Get count of stored vectors.
    async fn count(&self) -> Result<usize>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSearchResult {
    pub id: String,
    pub score: f32,
    pub metadata: serde_json::Value,
}
```

**Usage:**

```rust
// Store embedding with (id, vector, metadata) tuple
let chunk_text = "Your chunk text here";
let embedding = embedding_provider.embed(&[chunk_text]).await?[0].clone();
let metadata = serde_json::json!({
    "doc_id": "doc-123",
    "chunk_index": 0
});
vector_storage.upsert(&[
    ("chunk-001".to_string(), embedding, metadata)
]).await?;

// Search using query()
let results = vector_storage.query(&query_embedding, 10, None).await?;
```

### GraphStorage Trait

Graph storage for knowledge graph nodes and edges.

```rust
// Located: edgequake/crates/edgequake-storage/src/traits/graph.rs

#[async_trait]
pub trait GraphStorage: Send + Sync {
    /// Get the storage namespace.
    fn namespace(&self) -> &str;

    /// Initialize the graph storage.
    async fn initialize(&self) -> Result<()>;

    /// Flush pending changes.
    async fn finalize(&self) -> Result<()>;

    // ========== Node Operations ==========

    /// Check if a node exists.
    async fn has_node(&self, node_id: &str) -> Result<bool>;

    /// Get node by ID.
    async fn get_node(&self, node_id: &str) -> Result<Option<GraphNode>>;

    /// Insert or update a node.
    async fn upsert_node(
        &self,
        node_id: &str,
        properties: HashMap<String, serde_json::Value>,
    ) -> Result<()>;

    /// Delete node and its connected edges.
    async fn delete_node(&self, node_id: &str) -> Result<()>;

    /// Get the degree (number of edges) of a node.
    async fn node_degree(&self, node_id: &str) -> Result<usize>;

    /// Get all nodes.
    async fn get_all_nodes(&self) -> Result<Vec<GraphNode>>;

    /// Get nodes by a list of IDs.
    async fn get_nodes_by_ids(&self, node_ids: &[String]) -> Result<Vec<GraphNode>>;

    // ========== Edge Operations ==========

    /// Check if an edge exists between two nodes.
    async fn has_edge(&self, source: &str, target: &str) -> Result<bool>;

    /// Get an edge between two nodes.
    async fn get_edge(&self, source: &str, target: &str) -> Result<Option<GraphEdge>>;

    /// Insert or update an edge.
    async fn upsert_edge(
        &self,
        source: &str,
        target: &str,
        properties: HashMap<String, serde_json::Value>,
    ) -> Result<()>;

    /// Delete an edge.
    async fn delete_edge(&self, source: &str, target: &str) -> Result<()>;

    /// Get all edges connected to a node.
    async fn get_node_edges(&self, node_id: &str) -> Result<Vec<GraphEdge>>;

    /// Get all edges.
    async fn get_all_edges(&self) -> Result<Vec<GraphEdge>>;

    // ========== Graph Queries ==========

    /// Extract a subgraph starting from a node.
    async fn get_knowledge_graph(
        &self,
        start_node: &str,
        max_depth: usize,
        max_nodes: usize,
    ) -> Result<KnowledgeGraph>;

    /// Get the most connected (popular) node labels.
    async fn get_popular_labels(&self, limit: usize) -> Result<Vec<String>>;

    /// Search for nodes by label prefix.
    async fn search_labels(&self, query: &str, limit: usize) -> Result<Vec<String>>;

    /// Get neighbors of a node at a specific depth.
    async fn get_neighbors(&self, node_id: &str, depth: usize) -> Result<Vec<GraphNode>>;

    // ========== Utility Operations ==========

    /// Get node count.
    async fn node_count(&self) -> Result<usize>;

    /// Get edge count.
    async fn edge_count(&self) -> Result<usize>;

    /// Clear all nodes and edges.
    async fn clear(&self) -> Result<()>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub properties: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub properties: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub is_truncated: bool,
}
```

---

## Memory Storage

In-memory storage for development and testing. EdgeQuake provides separate memory storage implementations for each trait.

### Features

| Feature     | Support                     |
| ----------- | --------------------------- |
| Persistence | ❌ (data lost on restart)   |
| Concurrency | ✅ (thread-safe via RwLock) |
| Performance | ✅ (very fast)              |
| Production  | ❌ (development only)       |

### Available Implementations

| Class                 | Trait           | Description                          |
| --------------------- | --------------- | ------------------------------------ |
| `MemoryKVStorage`     | `KVStorage`     | In-memory key-value store            |
| `MemoryVectorStorage` | `VectorStorage` | Brute-force cosine similarity search |
| `MemoryGraphStorage`  | `GraphStorage`  | Adjacency list-based graph           |

### Usage

```rust
use edgequake_storage::adapters::memory::{
    MemoryKVStorage, MemoryVectorStorage, MemoryGraphStorage
};
use std::sync::Arc;

// Create separate storage instances with namespace
let kv_storage = Arc::new(MemoryKVStorage::new("my_namespace"));
let vector_storage = Arc::new(MemoryVectorStorage::new("my_namespace", 1536)); // dimension for embeddings
let graph_storage = Arc::new(MemoryGraphStorage::new("my_namespace"));

// Initialize each storage
kv_storage.initialize().await?;
vector_storage.initialize().await?;
graph_storage.initialize().await?;

// Initialize EdgeQuake
let mut eq = EdgeQuake::new(config)
    .with_storage_backends(kv_storage, vector_storage, graph_storage);
```

### Implementation Details

```rust
// Located: edgequake/crates/edgequake-storage/src/adapters/memory/

// Key-Value Storage
pub struct MemoryKVStorage {
    namespace: String,
    data: RwLock<HashMap<String, serde_json::Value>>,
    initialized: RwLock<bool>,
}

// Vector Storage (brute-force cosine similarity)
pub struct MemoryVectorStorage {
    namespace: String,
    dimension: usize,
    vectors: RwLock<HashMap<String, Vec<f32>>>,
    metadata: RwLock<HashMap<String, serde_json::Value>>,
}

// Graph Storage (adjacency list)
pub struct MemoryGraphStorage {
    namespace: String,
    nodes: RwLock<HashMap<String, HashMap<String, serde_json::Value>>>,
    edges: RwLock<HashMap<(String, String), HashMap<String, serde_json::Value>>>,
    adjacency: RwLock<HashMap<String, HashSet<String>>>,
}
```

---

## PostgreSQL Storage

Production-ready PostgreSQL storage with pgvector and Apache AGE.

### Features

| Feature       | Support                 |
| ------------- | ----------------------- |
| Persistence   | ✅                      |
| Concurrency   | ✅ (connection pooling) |
| Vector Search | ✅ (pgvector)           |
| Graph Queries | ✅ (Apache AGE)         |
| Scalability   | ✅ (horizontal scaling) |
| ACID          | ✅                      |
| Production    | ✅                      |

### Prerequisites

```sql
-- Enable extensions
CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS age;

-- Load AGE
LOAD 'age';
SET search_path = ag_catalog, "$user", public;
```

### Configuration

```rust
use edgequake_core::Config;

let config = Config {
    storage: StorageConfig {
        database_url: "postgres://user:pass@localhost:5432/edgequake".to_string(),
        max_connections: 10,
        min_connections: 2,
        connect_timeout_secs: 30,
        namespace: Some("default".to_string()),
    },
    ..Default::default()
};
```

### Environment Variables

```bash
# Required
EDGEQUAKE_DATABASE_URL=postgres://user:pass@localhost:5432/edgequake

# Optional
POSTGRES_MAX_CONNECTIONS=10
POSTGRES_MIN_CONNECTIONS=2
POSTGRES_CONNECT_TIMEOUT=30
EDGEQUAKE_NAMESPACE=default
```

### Schema

```sql
-- Documents table
CREATE TABLE IF NOT EXISTS edgequake_documents (
    id UUID PRIMARY KEY,
    namespace VARCHAR(255) NOT NULL DEFAULT 'default',
    title VARCHAR(1024),
    content TEXT,
    content_hash VARCHAR(64),
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processed_at TIMESTAMPTZ
);

CREATE INDEX idx_documents_namespace ON edgequake_documents(namespace);
CREATE INDEX idx_documents_status ON edgequake_documents(status);
CREATE INDEX idx_documents_hash ON edgequake_documents(content_hash);

-- Chunks table with vector embeddings
CREATE TABLE IF NOT EXISTS edgequake_chunks (
    id UUID PRIMARY KEY,
    namespace VARCHAR(255) NOT NULL DEFAULT 'default',
    document_id UUID NOT NULL REFERENCES edgequake_documents(id) ON DELETE CASCADE,
    chunk_index INTEGER NOT NULL,
    content TEXT NOT NULL,
    embedding vector(1536),
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_chunks_document ON edgequake_chunks(document_id);
CREATE INDEX idx_chunks_namespace ON edgequake_chunks(namespace);

-- Vector similarity index (HNSW)
CREATE INDEX idx_chunks_embedding ON edgequake_chunks
    USING hnsw (embedding vector_cosine_ops)
    WITH (m = 16, ef_construction = 64);

-- Entities table
CREATE TABLE IF NOT EXISTS edgequake_entities (
    id VARCHAR(512) PRIMARY KEY,
    namespace VARCHAR(255) NOT NULL DEFAULT 'default',
    entity_type VARCHAR(255) NOT NULL,
    description TEXT,
    embedding vector(1536),
    source_ids TEXT[],
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_entities_namespace ON edgequake_entities(namespace);
CREATE INDEX idx_entities_type ON edgequake_entities(entity_type);
CREATE INDEX idx_entities_embedding ON edgequake_entities
    USING hnsw (embedding vector_cosine_ops);

-- Relationships table
CREATE TABLE IF NOT EXISTS edgequake_relationships (
    id UUID PRIMARY KEY,
    namespace VARCHAR(255) NOT NULL DEFAULT 'default',
    source_entity VARCHAR(512) NOT NULL REFERENCES edgequake_entities(id),
    target_entity VARCHAR(512) NOT NULL REFERENCES edgequake_entities(id),
    relationship_type VARCHAR(255) NOT NULL,
    description TEXT,
    weight FLOAT NOT NULL DEFAULT 1.0,
    source_ids TEXT[],
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_relationships_namespace ON edgequake_relationships(namespace);
CREATE INDEX idx_relationships_source ON edgequake_relationships(source_entity);
CREATE INDEX idx_relationships_target ON edgequake_relationships(target_entity);
CREATE INDEX idx_relationships_type ON edgequake_relationships(relationship_type);

-- Tasks table
CREATE TABLE IF NOT EXISTS edgequake_tasks (
    id UUID PRIMARY KEY,
    track_id VARCHAR(255) NOT NULL UNIQUE,
    namespace VARCHAR(255) NOT NULL DEFAULT 'default',
    task_type VARCHAR(50) NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    progress FLOAT DEFAULT 0.0,
    message TEXT,
    data JSONB,
    error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);

CREATE INDEX idx_tasks_track ON edgequake_tasks(track_id);
CREATE INDEX idx_tasks_status ON edgequake_tasks(status);
CREATE INDEX idx_tasks_namespace ON edgequake_tasks(namespace);
```

### Vector Search Query

```sql
-- Find similar chunks
SELECT id, content, metadata,
       1 - (embedding <=> $1::vector) AS similarity
FROM edgequake_chunks
WHERE namespace = $2
ORDER BY embedding <=> $1::vector
LIMIT $3;
```

### Graph Queries (Apache AGE)

```sql
-- Create graph
SELECT create_graph('edgequake_graph');

-- Add node
SELECT * FROM cypher('edgequake_graph', $$
    MERGE (e:Entity {id: 'MARIE_CURIE'})
    SET e.entity_type = 'PERSON',
        e.description = 'Polish-French physicist'
    RETURN e
$$) AS (entity agtype);

-- Add relationship
SELECT * FROM cypher('edgequake_graph', $$
    MATCH (a:Entity {id: 'MARIE_CURIE'})
    MATCH (b:Entity {id: 'RADIUM'})
    MERGE (a)-[r:DISCOVERED]->(b)
    SET r.weight = 1.0
    RETURN r
$$) AS (relationship agtype);

-- Traverse neighbors
SELECT * FROM cypher('edgequake_graph', $$
    MATCH (start:Entity {id: 'MARIE_CURIE'})-[*1..2]-(neighbor:Entity)
    RETURN DISTINCT neighbor
    LIMIT 50
$$) AS (neighbor agtype);
```

### Usage

> **Code Reference**: See [edgequake/crates/edgequake-storage/src/adapters/postgres/](../edgequake/crates/edgequake-storage/src/adapters/postgres/) for PostgreSQL implementations

```rust
use edgequake_storage::{PostgresConfig, PostgresKVStorage, PgVectorStorage, PostgresAGEGraphStorage};
use std::sync::Arc;

// Configure PostgreSQL
let config = PostgresConfig {
    host: "localhost".to_string(),
    port: 5432,
    database: "edgequake".to_string(),
    user: "postgres".to_string(),
    password: "password".to_string(),
    namespace: "production".to_string(),
    ..Default::default()
};

// Create storage instances (each manages its own connection pool)
let kv_storage = Arc::new(PostgresKVStorage::new(config.clone()));
let vector_storage = Arc::new(PgVectorStorage::new(config.clone()));
let graph_storage = Arc::new(PostgresAGEGraphStorage::new(config));

// Initialize storages
kv_storage.initialize().await?;
vector_storage.initialize().await?;
graph_storage.initialize().await?;
```

---

## Configuration Reference

### PostgresConfig

```rust
// Located: edgequake/crates/edgequake-storage/src/adapters/postgres/config.rs

pub struct PostgresConfig {
    /// Database host
    pub host: String,

    /// Database port
    pub port: u16,

    /// Database name
    pub database: String,

    /// Username
    pub user: String,

    /// Password
    pub password: String,

    /// Namespace for multi-tenancy
    pub namespace: String,  // Default: "default"

    /// Maximum connections in pool
    pub max_connections: u32,  // Default: 10

    /// Minimum connections in pool
    pub min_connections: u32,  // Default: 1

    /// Connection timeout
    pub connect_timeout: Duration,  // Default: 30 seconds

    /// Idle connection timeout
    pub idle_timeout: Duration,  // Default: 600 seconds

    /// SSL mode (Prefer, Require, Disable)
    pub ssl_mode: SslMode,

    /// Vector index type (HNSW or IVFFlat)
    pub vector_index_type: VectorIndexType,

    /// HNSW M parameter
    pub hnsw_m: u32,  // Default: 16

    /// HNSW ef_construction parameter
    pub hnsw_ef_construction: u32,  // Default: 64
}
```

### Environment Variables

| Variable                   | Default | Description                  |
| -------------------------- | ------- | ---------------------------- |
| `EDGEQUAKE_DATABASE_URL`   | -       | PostgreSQL connection string |
| `POSTGRES_MAX_CONNECTIONS` | 10      | Max pool connections         |
| `POSTGRES_MIN_CONNECTIONS` | 1       | Min pool connections         |
| `POSTGRES_CONNECT_TIMEOUT` | 30      | Connection timeout (seconds) |
| `EDGEQUAKE_NAMESPACE`      | default | Multi-tenant namespace       |

---

## Migration Guide

### From Memory to PostgreSQL

1. **Set up PostgreSQL:**

```bash
# Start PostgreSQL with extensions
docker run -d --name edgequake-pg \
  -e POSTGRES_PASSWORD=password \
  -p 5432:5432 \
  ankane/pgvector:latest

# Connect and enable extensions
psql -h localhost -U postgres -c "CREATE EXTENSION vector;"
```

2. **Update configuration:**

```rust
let config = Config {
    storage: StorageConfig {
        database_url: "postgres://postgres:password@localhost:5432/edgequake".to_string(),
        ..Default::default()
    },
    ..Default::default()
};
```

3. **Run migrations:**

```rust
let storage = PostgresStorage::connect(&config.storage.database_url).await?;
storage.run_migrations().await?;
```

4. **Re-index existing documents:**

```bash
# Re-process all documents to populate PostgreSQL
cargo run --bin edgequake-reindex -- --all
```

---

## Best Practices

### Development

```rust
use edgequake_storage::adapters::memory::{
    MemoryKVStorage, MemoryVectorStorage, MemoryGraphStorage
};

// Use memory storage for fast iteration
let kv = MemoryKVStorage::new("dev");
let vector = MemoryVectorStorage::new("dev", 1536);
let graph = MemoryGraphStorage::new("dev");
```

### Production

> **Enforces**: [BR0015](business_rules.md#br0015) Connection Pooling, [BR0016](business_rules.md#br0016) Graceful Degradation

```rust
use edgequake_storage::adapters::postgres::{PostgresConfig, PostgresPool};

// Use PostgreSQL with connection pooling
let config = PostgresConfig {
    host: std::env::var("DB_HOST").unwrap_or("localhost".to_string()),
    database: std::env::var("DB_NAME").unwrap_or("edgequake".to_string()),
    user: std::env::var("DB_USER").unwrap_or("postgres".to_string()),
    password: std::env::var("DB_PASSWORD").unwrap_or_default(),
    namespace: "production".to_string(),
    max_connections: 20,
    min_connections: 5,
    ..Default::default()
};
```

### Testing

```rust
#[tokio::test]
async fn test_with_memory_storage() {
    use edgequake_storage::adapters::memory::MemoryKVStorage;

    let storage = MemoryKVStorage::new("test");
    storage.initialize().await.unwrap();
    // Test with isolated in-memory storage
}
```

---

## Troubleshooting

| Problem                        | Cause                           | Solution                                |
| ------------------------------ | ------------------------------- | --------------------------------------- |
| "connection refused"           | PostgreSQL not running          | `docker compose up -d postgres`         |
| "extension 'vector' not found" | pgvector not installed          | Use `ankane/pgvector` Docker image      |
| "relation does not exist"      | Migrations not run              | Run `storage.run_migrations().await?`   |
| Slow vector searches           | Missing HNSW index              | Create index: `CREATE INDEX USING hnsw` |
| Memory OOM                     | Large dataset in memory storage | Switch to PostgreSQL for production     |

---

## Next Steps

| Your Goal               | Next Document                                              |
| ----------------------- | ---------------------------------------------------------- |
| Configure LLM providers | [LLM Integration](0005-llm-integration.md)                 |
| Deploy to production    | [Deployment Guide](0006-deployment-guide.md)               |
| Configure all options   | [Configuration Reference](0007-configuration-reference.md) |
| Set up multi-tenancy    | [Multi-Tenancy](0008-multi-tenancy.md)                     |

> **See Also**: [Features Registry](features.md) | [Architecture Overview](0002-architecture-overview.md)
