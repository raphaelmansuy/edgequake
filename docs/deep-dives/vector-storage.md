# Deep Dive: Vector Storage

> **How EdgeQuake Stores and Searches Vector Embeddings**

Vector storage powers EdgeQuake's semantic search capabilities. This document explains how embeddings are stored, indexed, and queried for similarity.

---

## Overview

EdgeQuake uses a trait-based vector storage abstraction to support multiple backends:

```
┌─────────────────────────────────────────────────────────────────┐
│                    VECTOR STORAGE ARCHITECTURE                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                     LLM PROVIDER                            ││
│  │                                                             ││
│  │  ┌───────────────┐     ┌───────────────────────────────────┐││
│  │  │ Text Chunk    │────▶│ Embedding Model                   │││
│  │  │ "Dr. Sarah    │     │ (text-embedding-3-small)          │││
│  │  │  Chen..."     │     │                                   │││
│  │  └───────────────┘     └─────────────┬─────────────────────┘││
│  │                                      │                      ││
│  │                              [1536-dim vector]              ││
│  │                                      │                      ││
│  └──────────────────────────────────────│──────────────────────┘│
│                                         │                       │
│                                         ▼                       │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                    VECTOR STORAGE                          │ │
│  │                                                            │ │
│  │  ┌──────────┐    ┌──────────┐    ┌──────────┐              │ │
│  │  │ IVFFlat  │    │  HNSW    │    │ Memory   │              │ │
│  │  │ (lists)  │    │ (graph)  │    │ (brute)  │              │ │
│  │  └────┬─────┘    └────┬─────┘    └────┬─────┘              │ │
│  │       │               │               │                    │ │
│  │       └───────────────┼───────────────┘                    │ │
│  │                       │                                    │ │
│  │                       ▼                                    │ │
│  │           ┌───────────────────────────────────────┐        │ │
│  │           │          pgvector / memory            │        │ │
│  │           │     [id, embedding, metadata]         │        │ │
│  │           └───────────────────────────────────────┘        │ │
│  │                                                            │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Why Separate Vector Storage?

| Reason                  | Benefit                                         |
| ----------------------- | ----------------------------------------------- |
| **Specialized Indices** | HNSW, IVFFlat optimized for nearest-neighbor    |
| **GPU Acceleration**    | Backends like Faiss can use GPU                 |
| **Different Scaling**   | Vectors scale differently than graph data       |
| **Backend Flexibility** | Can use Pinecone, Weaviate, Qdrant, or pgvector |

---

## Core Data Structures

### VectorSearchResult

Results from similarity queries:

```rust
/// Vector similarity search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSearchResult {
    /// Record identifier (chunk ID, entity name, etc.)
    pub id: String,

    /// Similarity score (higher = more similar)
    /// Range: -1.0 to 1.0 for cosine similarity
    pub score: f32,

    /// Associated metadata (source, timestamps, etc.)
    pub metadata: serde_json::Value,
}
```

**Metadata Fields:**

| Field          | Type   | Description           |
| -------------- | ------ | --------------------- |
| `source_id`    | String | Origin document/chunk |
| `entity_type`  | String | PERSON, CONCEPT, etc. |
| `created_at`   | String | ISO timestamp         |
| `workspace_id` | UUID   | Tenant isolation      |

---

## The VectorStorage Trait

All vector backends implement this interface:

```rust
#[async_trait]
pub trait VectorStorage: Send + Sync {
    /// Get the storage namespace (for multi-tenancy)
    fn namespace(&self) -> &str;

    /// Get the expected embedding dimension
    fn dimension(&self) -> usize;

    /// Initialize storage (create tables/indices)
    async fn initialize(&self) -> Result<()>;

    /// Flush pending changes
    async fn finalize(&self) -> Result<()>;

    // ========== Search Operations ==========

    /// Perform similarity search
    async fn query(
        &self,
        query_embedding: &[f32],
        top_k: usize,
        filter_ids: Option<&[String]>,
    ) -> Result<Vec<VectorSearchResult>>;

    // ========== CRUD Operations ==========

    /// Insert or update vectors
    async fn upsert(
        &self,
        data: &[(String, Vec<f32>, serde_json::Value)]
    ) -> Result<()>;

    /// Delete vectors by IDs
    async fn delete(&self, ids: &[String]) -> Result<()>;

    /// Delete all vectors for an entity
    async fn delete_entity(&self, entity_name: &str) -> Result<()>;

    /// Delete relationship vectors for an entity
    async fn delete_entity_relations(&self, entity_name: &str) -> Result<()>;

    // ========== Retrieval ==========

    /// Get single vector by ID
    async fn get_by_id(&self, id: &str) -> Result<Option<Vec<f32>>>;

    /// Get multiple vectors by IDs
    async fn get_by_ids(&self, ids: &[String]) -> Result<Vec<(String, Vec<f32>)>>;

    // ========== Utility ==========

    async fn is_empty(&self) -> Result<bool>;
    async fn count(&self) -> Result<usize>;
    async fn clear(&self) -> Result<()>;
    async fn clear_workspace(&self, workspace_id: &Uuid) -> Result<usize>;
}
```

---

## Storage Backends

### MemoryVectorStorage

In-memory implementation using brute-force cosine similarity:

```rust
pub struct MemoryVectorStorage {
    namespace: String,
    dimension: usize,
    vectors: RwLock<HashMap<String, Vec<f32>>>,
    metadata: RwLock<HashMap<String, serde_json::Value>>,
}
```

**Characteristics:**

| Attribute         | Value                    |
| ----------------- | ------------------------ |
| Index Type        | None (brute-force)       |
| Search Complexity | O(n) per query           |
| Memory Usage      | ~4KB per 1024-dim vector |
| Best For          | Testing, <10K vectors    |

**Cosine Similarity:**

```rust
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter())
        .map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}
```

**Usage:**

```rust
let storage = MemoryVectorStorage::new("my_workspace", 1536);
storage.initialize().await?;
```

---

### PgVectorStorage

Production-grade storage using PostgreSQL with pgvector extension:

```rust
pub struct PgVectorStorage {
    pool: PostgresPool,
    table_name: String,
    namespace: String,
    dimension: usize,
    index_type: VectorIndexType,
    ivfflat_lists: u32,
    hnsw_m: u32,
    hnsw_ef_construction: u32,
}
```

**Characteristics:**

| Attribute        | Value                     |
| ---------------- | ------------------------- |
| Persistence      | ✅ Full durability        |
| Index Types      | IVFFlat, HNSW             |
| Distance Metrics | Cosine, L2, Inner Product |
| Best For         | Production, >10K vectors  |

**Schema:**

```sql
CREATE TABLE vectors (
    id TEXT PRIMARY KEY,
    embedding vector(1536) NOT NULL,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

---

## Index Types

### IVFFlat (Inverted File with Flat Quantization)

Partitions vector space into clusters:

```
┌─────────────────────────────────────────────────────────────────┐
│                    IVFFlat INDEX                                │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Step 1: Cluster vectors into lists (Voronoi cells)             │
│                                                                 │
│     ┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐    │
│     │ List 1  │    │ List 2  │    │ List 3  │    │ List N  │    │
│     │         │    │         │    │         │    │         │    │
│     │ ● ● ●   │    │ ● ● ●   │    │ ● ●     │    │ ● ● ●   │    │
│     │   ●     │    │ ● ●     │    │ ● ● ●   │    │ ●       │    │
│     └─────────┘    └─────────┘    └─────────┘    └─────────┘    │
│                                                                 │
│  Step 2: Query finds nearest centroid(s)                        │
│  Step 3: Search only those lists (probes=1-5)                   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**Configuration:**

```sql
-- Create IVFFlat index
CREATE INDEX ON vectors
USING ivfflat (embedding vector_cosine_ops)
WITH (lists = 100);  -- ~sqrt(n) lists recommended
```

| Parameter | Recommendation                |
| --------- | ----------------------------- |
| `lists`   | sqrt(n) to 4\*sqrt(n)         |
| `probes`  | 1-10 (higher = better recall) |

---

### HNSW (Hierarchical Navigable Small World)

Multi-layer graph for efficient nearest-neighbor:

```
┌─────────────────────────────────────────────────────────────────┐
│                    HNSW INDEX                                   │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Layer 2 (sparse)     ●─────────────────────────●               │
│                       │                         │               │
│                       │                         │               │
│  Layer 1 (medium)     ●───●───●─────────●───●───●               │
│                       │   │   │         │   │   │               │
│                       │   │   │         │   │   │               │
│  Layer 0 (dense)    ●─●─●─●─●─●───────●─●─●─●─●─●               │
│                                                                 │
│  • Entry point at top layer                                     │
│  • Greedy descent through layers                                │
│  • Local search at layer 0                                      │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**Configuration:**

```sql
-- Create HNSW index
CREATE INDEX ON vectors
USING hnsw (embedding vector_cosine_ops)
WITH (m = 16, ef_construction = 64);
```

| Parameter         | Meaning                   | Default |
| ----------------- | ------------------------- | ------- |
| `m`               | Max connections per layer | 16      |
| `ef_construction` | Build-time beam width     | 64      |
| `ef_search`       | Query-time beam width     | 40      |

---

## Index Selection Guide

```
┌─────────────────────────────────────────────────────────────────┐
│                 WHEN TO USE EACH INDEX                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Dataset Size:                                                  │
│                                                                 │
│    < 1K vectors     →  None (brute-force is fine)               │
│    1K - 100K        →  IVFFlat (good balance)                   │
│    > 100K           →  HNSW (faster queries)                    │
│                                                                 │
│  Query Pattern:                                                 │
│                                                                 │
│    Many inserts     →  IVFFlat (faster builds)                  │
│    Many queries     →  HNSW (faster search)                     │
│    Both balanced    →  HNSW (better latency)                    │
│                                                                 │
│  Resource Constraints:                                          │
│                                                                 │
│    Limited memory   →  IVFFlat (lower overhead)                 │
│    Abundant memory  →  HNSW (better performance)                │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Embedding Dimensions

EdgeQuake supports multiple embedding models:

| Model                    | Dimensions | Provider |
| ------------------------ | ---------- | -------- |
| `text-embedding-3-small` | 1536       | OpenAI   |
| `text-embedding-3-large` | 3072       | OpenAI   |
| `text-embedding-ada-002` | 1536       | OpenAI   |
| `nomic-embed-text`       | 768        | Ollama   |
| `mxbai-embed-large`      | 1024       | Ollama   |
| `all-MiniLM-L6-v2`       | 384        | Local    |

**Dimension Mismatch Handling:**

EdgeQuake automatically detects when stored vectors have a different dimension than the current provider:

```rust
// Check stored dimension from pg_attribute
pub async fn get_stored_dimension(&self) -> Result<Option<usize>> {
    // Query atttypmod from pg_attribute (works for empty tables!)
    let sql = r#"
        SELECT a.atttypmod FROM pg_attribute a
        JOIN pg_class c ON a.attrelid = c.oid
        WHERE c.relname = $1 AND a.attname = 'embedding'
    "#;
    // ...
}
```

---

## Vector Operations

### Upsert Vectors

```rust
// Prepare embeddings
let data = vec![
    (
        "chunk_001".to_string(),
        vec![0.1, 0.2, 0.3, ...],  // 1536 dimensions
        json!({
            "source_id": "doc_123",
            "content_preview": "Dr. Sarah Chen..."
        })
    ),
    // ...more vectors
];

// Bulk insert
storage.upsert(&data).await?;
```

### Similarity Search

```rust
// Query embedding from LLM
let query_embedding = llm.embed("Who is Sarah Chen?").await?;

// Search top 10 most similar
let results = storage.query(
    &query_embedding,
    10,    // top_k
    None,  // no filter
).await?;

for result in results {
    println!("ID: {}, Score: {:.4}", result.id, result.score);
}
```

### Filtered Search

```rust
// Search within specific chunks only
let filter = vec![
    "chunk_001".to_string(),
    "chunk_002".to_string(),
    "chunk_003".to_string(),
];

let results = storage.query(
    &query_embedding,
    10,
    Some(&filter),  // restrict to these IDs
).await?;
```

---

## Performance Tuning

### pgvector Settings

```sql
-- Set HNSW search beam width (higher = better recall)
SET hnsw.ef_search = 100;

-- Set IVF probes (higher = better recall)
SET ivfflat.probes = 10;

-- Enable parallel queries
SET max_parallel_workers_per_gather = 4;
```

### Connection Pooling

```rust
let pool = PgPoolOptions::new()
    .max_connections(20)           // Concurrent queries
    .min_connections(5)            // Keep-alive
    .acquire_timeout(Duration::from_secs(30))
    .connect(&database_url)
    .await?;
```

### Batch Operations

```rust
// Bad: Many small inserts
for (id, vec, meta) in data {
    storage.upsert(&[(id, vec, meta)]).await?;
}

// Good: Single batch insert
storage.upsert(&data).await?;
```

---

## Benchmarks

Performance on typical workloads (pgvector with HNSW, 100K vectors, 1536 dimensions):

| Operation      | Latency | Notes         |
| -------------- | ------- | ------------- |
| `query(k=10)`  | ~5ms    | HNSW ef=100   |
| `query(k=100)` | ~15ms   | HNSW ef=100   |
| `upsert(1)`    | ~3ms    | Single vector |
| `upsert(100)`  | ~50ms   | Batch insert  |
| Index build    | ~30s    | 100K vectors  |

**Memory Usage:**

- 1536-dim vector: ~6KB (with overhead)
- 100K vectors: ~600MB
- HNSW index: ~200MB additional

---

## Multi-Tenancy

Vector storage supports workspace-based isolation:

```rust
// Each workspace gets own prefix
let storage_a = PgVectorStorage::new(config_a);  // eq_ws_a_vectors
let storage_b = PgVectorStorage::new(config_b);  // eq_ws_b_vectors

// Clear specific workspace
storage_a.clear_workspace(&workspace_id).await?;
```

---

## Best Practices

1. **Match Dimensions** - Always use same embedding model for indexing and querying
2. **Batch Inserts** - Use bulk upsert for multiple vectors
3. **Tune Indices** - Adjust HNSW m/ef or IVFFlat lists for your dataset
4. **Monitor Size** - Track vector counts for capacity planning
5. **Normalize Vectors** - Cosine similarity assumes unit vectors

---

## Common Issues

### Dimension Mismatch

```
Error: Vector dimension 768 doesn't match expected 1536
```

**Solution:** Either:

- Rebuild embeddings with correct model
- Drop and recreate table with `drop_table()`

### Slow Queries

**Symptoms:** Query latency >100ms

**Solutions:**

1. Create index if missing
2. Increase `ef_search` for HNSW
3. Increase `probes` for IVFFlat
4. Check connection pool exhaustion

### Index Build Timeout

**Symptoms:** Index creation hangs

**Solution:** For large datasets, create index with reduced parameters:

```sql
CREATE INDEX CONCURRENTLY ON vectors
USING hnsw (embedding vector_cosine_ops)
WITH (m = 8, ef_construction = 32);
```

---

## See Also

- [Graph Storage](./graph-storage.md) - Knowledge graph storage
- [Entity Extraction](./entity-extraction.md) - How entities get embeddings
- [Query Modes](./query-modes.md) - How vector search is used
- [Performance Tuning](../operations/performance-tuning.md) - Optimization guide
