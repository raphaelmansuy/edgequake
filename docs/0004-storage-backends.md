# EdgeQuake Storage Backends

> Complete guide to storage backend configuration and implementation

**Version**: 0.1.0 | **Last Updated**: December 2025

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
    /// Get a value by key.
    async fn get(&self, key: &str) -> Result<Option<serde_json::Value>>;
    
    /// Get multiple values by keys.
    async fn get_batch(&self, keys: &[String]) -> Result<Vec<(String, serde_json::Value)>>;
    
    /// Upsert key-value pairs.
    async fn upsert(&self, items: &[(String, serde_json::Value)]) -> Result<()>;
    
    /// Delete keys.
    async fn delete(&self, keys: &[String]) -> Result<()>;
    
    /// List all keys (with optional prefix filter).
    async fn keys(&self, prefix: Option<&str>) -> Result<Vec<String>>;
    
    /// Check if key exists.
    async fn exists(&self, key: &str) -> Result<bool>;
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

// Retrieve
let doc = kv_storage.get("doc-123-metadata").await?;
```

### VectorStorage Trait

Vector storage for embeddings and similarity search.

```rust
// Located: edgequake/crates/edgequake-storage/src/traits/vector.rs

#[async_trait]
pub trait VectorStorage: Send + Sync {
    /// Insert or update vectors.
    async fn upsert(&self, vectors: &[VectorEntry]) -> Result<()>;
    
    /// Search for similar vectors.
    async fn search(
        &self, 
        query: &[f32], 
        top_k: usize,
        filter: Option<&VectorFilter>
    ) -> Result<Vec<SearchResult>>;
    
    /// Delete vectors by ID.
    async fn delete(&self, ids: &[String]) -> Result<()>;
    
    /// Get vector by ID.
    async fn get(&self, id: &str) -> Result<Option<VectorEntry>>;
    
    /// Get total vector count.
    async fn count(&self) -> Result<usize>;
}

pub struct VectorEntry {
    pub id: String,
    pub vector: Vec<f32>,
    pub metadata: HashMap<String, serde_json::Value>,
}

pub struct SearchResult {
    pub id: String,
    pub score: f32,
    pub metadata: HashMap<String, serde_json::Value>,
}
```

**Usage:**

```rust
// Store embedding
let entry = VectorEntry {
    id: "chunk-001".to_string(),
    vector: embedding_provider.embed(&["chunk text"]).await?[0].clone(),
    metadata: HashMap::from([
        ("doc_id".to_string(), json!("doc-123")),
        ("chunk_index".to_string(), json!(0)),
    ]),
};
vector_storage.upsert(&[entry]).await?;

// Search
let results = vector_storage.search(&query_embedding, 10, None).await?;
```

### GraphStorage Trait

Graph storage for knowledge graph nodes and edges.

```rust
// Located: edgequake/crates/edgequake-storage/src/traits/graph.rs

#[async_trait]
pub trait GraphStorage: Send + Sync {
    /// Add or update a node.
    async fn add_node(&self, node: GraphNode) -> Result<()>;
    
    /// Add or update an edge.
    async fn add_edge(&self, edge: GraphEdge) -> Result<()>;
    
    /// Get node by ID.
    async fn get_node(&self, id: &str) -> Result<Option<GraphNode>>;
    
    /// Get edge by ID.
    async fn get_edge(&self, id: &str) -> Result<Option<GraphEdge>>;
    
    /// Get neighbors of a node.
    async fn get_neighbors(&self, id: &str, depth: usize) -> Result<Vec<GraphNode>>;
    
    /// Get knowledge graph subgraph.
    async fn get_knowledge_graph(
        &self,
        start_node: &str,
        depth: usize,
        limit: usize
    ) -> Result<KnowledgeGraph>;
    
    /// Delete node and its edges.
    async fn delete_node(&self, id: &str) -> Result<()>;
    
    /// Delete edge.
    async fn delete_edge(&self, id: &str) -> Result<()>;
    
    /// Get node count.
    async fn node_count(&self) -> Result<usize>;
    
    /// Get edge count.
    async fn edge_count(&self) -> Result<usize>;
    
    /// Get node degree.
    async fn node_degree(&self, id: &str) -> Result<usize>;
    
    /// Get popular labels (most connected nodes).
    async fn get_popular_labels(&self, limit: usize) -> Result<Vec<String>>;
}

pub struct GraphNode {
    pub id: String,
    pub properties: HashMap<String, serde_json::Value>,
}

pub struct GraphEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub properties: HashMap<String, serde_json::Value>,
}

pub struct KnowledgeGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub is_truncated: bool,
}
```

---

## Memory Storage

In-memory storage for development and testing.

### Features

| Feature | Support |
|---------|---------|
| Persistence | ❌ (data lost on restart) |
| Concurrency | ✅ (thread-safe) |
| Performance | ✅ (very fast) |
| Production | ❌ (development only) |

### Usage

```rust
use edgequake_storage::MemoryStorage;
use std::sync::Arc;

// Create memory storage
let storage = MemoryStorage::new();

// Use for all storage types
let kv_storage: Arc<dyn KVStorage> = Arc::new(storage.clone());
let vector_storage: Arc<dyn VectorStorage> = Arc::new(storage.clone());
let graph_storage: Arc<dyn GraphStorage> = Arc::new(storage.clone());

// Initialize EdgeQuake
let mut eq = EdgeQuake::new(config)
    .with_storage_backends(kv_storage, vector_storage, graph_storage);
```

### Implementation Details

```rust
// Located: edgequake/crates/edgequake-storage/src/adapters/memory/

pub struct MemoryStorage {
    kv: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    vectors: Arc<RwLock<HashMap<String, VectorEntry>>>,
    nodes: Arc<RwLock<HashMap<String, GraphNode>>>,
    edges: Arc<RwLock<HashMap<String, GraphEdge>>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            kv: Arc::new(RwLock::new(HashMap::new())),
            vectors: Arc::new(RwLock::new(HashMap::new())),
            nodes: Arc::new(RwLock::new(HashMap::new())),
            edges: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}
```

---

## PostgreSQL Storage

Production-ready PostgreSQL storage with pgvector and Apache AGE.

### Features

| Feature | Support |
|---------|---------|
| Persistence | ✅ |
| Concurrency | ✅ (connection pooling) |
| Vector Search | ✅ (pgvector) |
| Graph Queries | ✅ (Apache AGE) |
| Scalability | ✅ (horizontal scaling) |
| ACID | ✅ |
| Production | ✅ |

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

```rust
use edgequake_storage::PostgresStorage;

// Connect to PostgreSQL
let storage = PostgresStorage::connect(
    "postgres://user:pass@localhost:5432/edgequake"
).await?;

// Run migrations
storage.run_migrations().await?;

// Get storage instances
let kv_storage = Arc::new(storage.kv_storage());
let vector_storage = Arc::new(storage.vector_storage());
let graph_storage = Arc::new(storage.graph_storage());
```

---

## Configuration Reference

### Storage Configuration

```rust
pub struct StorageConfig {
    /// Database connection URL
    pub database_url: String,
    
    /// Maximum connections in pool
    pub max_connections: u32,  // Default: 10
    
    /// Minimum connections in pool
    pub min_connections: u32,  // Default: 1
    
    /// Connection timeout (seconds)
    pub connect_timeout_secs: u64,  // Default: 30
    
    /// Namespace for multi-tenancy
    pub namespace: Option<String>,  // Default: None
}
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `EDGEQUAKE_DATABASE_URL` | - | PostgreSQL connection string |
| `POSTGRES_MAX_CONNECTIONS` | 10 | Max pool connections |
| `POSTGRES_MIN_CONNECTIONS` | 1 | Min pool connections |
| `POSTGRES_CONNECT_TIMEOUT` | 30 | Connection timeout (seconds) |
| `EDGEQUAKE_NAMESPACE` | default | Multi-tenant namespace |

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
// Use memory storage for fast iteration
let storage = MemoryStorage::new();
```

### Production

```rust
// Use PostgreSQL with connection pooling
let config = StorageConfig {
    database_url: std::env::var("DATABASE_URL")?,
    max_connections: 20,
    min_connections: 5,
    connect_timeout_secs: 30,
    namespace: Some("production".to_string()),
};
```

### Testing

```rust
#[tokio::test]
async fn test_with_memory_storage() {
    let storage = MemoryStorage::new();
    // Test with isolated in-memory storage
}
```

---

## Next Steps

- **[LLM Integration](0005-llm-integration.md)** - Configure LLM providers
- **[Deployment Guide](0006-deployment-guide.md)** - Production deployment
- **[Configuration Reference](0007-configuration-reference.md)** - All config options
