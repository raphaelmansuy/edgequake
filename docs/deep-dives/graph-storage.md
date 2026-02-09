# Deep Dive: Graph Storage

> **How EdgeQuake Stores and Queries Knowledge Graphs**

Graph storage is the foundation of EdgeQuake's knowledge management. This document explains how entities and relationships are stored, the property graph model, and available storage backends.

---

## Overview

EdgeQuake uses a property graph model to store extracted knowledge:

```
┌─────────────────────────────────────────────────────────────────┐
│                    PROPERTY GRAPH MODEL                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                       NODES (Entities)                      ││
│  │                                                             ││
│  │  ┌───────────────────┐     ┌───────────────────┐            ││
│  │  │ SARAH_CHEN        │     │ MIT               │            ││
│  │  ├───────────────────┤     ├───────────────────┤            ││
│  │  │ type: PERSON      │     │ type: ORGANIZATION│            ││
│  │  │ description: ...  │     │ description: ...  │            ││
│  │  │ source_id: chunk1 │     │ source_id: chunk1 │            ││
│  │  │ importance: 0.9   │     │ importance: 0.8   │            ││
│  │  └─────────┬─────────┘     └─────────┬─────────┘            ││
│  │            │                         │                      ││
│  └────────────│─────────────────────────│──────────────────────┘│
│               │                         │                       │
│  ┌────────────│─────────────────────────│──────────────────────┐│
│  │            │     EDGES (Relationships)│                     ││
│  │            │                         │                      ││
│  │            └─────────────────────────┘                      ││
│  │                      │                                      ││
│  │                      ▼                                      ││
│  │  ┌─────────────────────────────────────────────────────────┐││
│  │  │ SARAH_CHEN ──[works_at]──▶ MIT                          │││
│  │  ├─────────────────────────────────────────────────────────┤││
│  │  │ relation_type: works_at                                 │││
│  │  │ description: "Dr. Chen is a researcher at MIT"          │││
│  │  │ weight: 0.9                                             │││
│  │  │ keywords: ["researcher", "faculty", "AI"]               │││
│  │  │ source_chunk_id: chunk1                                 │││
│  │  └─────────────────────────────────────────────────────────┘││
│  │                                                             ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Why Property Graphs?

| Feature                  | Benefit                                          |
| ------------------------ | ------------------------------------------------ |
| **Arbitrary Properties** | Each node/edge can have different attributes     |
| **Rich Metadata**        | Store descriptions, weights, timestamps, sources |
| **Flexible Schema**      | Adapt to different domains without migration     |
| **Graph Traversal**      | Efficient neighbor and path queries              |
| **Compatibility**        | Works with Apache AGE, Neo4j, SurrealDB          |

---

## Core Data Structures

### GraphNode

Represents an entity in the knowledge graph:

```rust
/// A node in the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    /// Node identifier (typically the normalized entity name)
    pub id: String,

    /// Node properties (arbitrary key-value pairs)
    pub properties: HashMap<String, serde_json::Value>,
}
```

**Standard Properties:**

| Property             | Type   | Description                         |
| -------------------- | ------ | ----------------------------------- |
| `entity_type`        | String | PERSON, ORGANIZATION, CONCEPT, etc. |
| `description`        | String | LLM-generated description           |
| `source_chunk_id`    | String | Origin chunk for lineage            |
| `source_document_id` | String | Origin document                     |
| `importance`         | f32    | Relevance score (0.0-1.0)           |
| `created_at`         | String | ISO timestamp                       |

### GraphEdge

Represents a relationship between entities:

```rust
/// An edge in the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    /// Source node identifier
    pub source: String,

    /// Target node identifier
    pub target: String,

    /// Edge properties
    pub properties: HashMap<String, serde_json::Value>,
}
```

**Standard Properties:**

| Property          | Type        | Description                            |
| ----------------- | ----------- | -------------------------------------- |
| `relation_type`   | String      | works_at, developed, uses, etc.        |
| `description`     | String      | LLM-generated relationship description |
| `weight`          | f32         | Relationship strength (0.0-1.0)        |
| `keywords`        | Vec<String> | Up to 5 keywords (BR0004)              |
| `source_chunk_id` | String      | Origin chunk                           |

### KnowledgeGraph

A subgraph result from queries:

```rust
/// A subgraph extracted from the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGraph {
    /// Nodes in the subgraph
    pub nodes: Vec<GraphNode>,

    /// Edges in the subgraph
    pub edges: Vec<GraphEdge>,

    /// Whether result was truncated
    pub is_truncated: bool,
}
```

---

## The GraphStorage Trait

All graph backends implement this trait:

```rust
#[async_trait]
pub trait GraphStorage: Send + Sync {
    /// Get the storage namespace (for multi-tenancy).
    fn namespace(&self) -> &str;

    /// Initialize the storage.
    async fn initialize(&self) -> Result<()>;

    /// Flush pending changes.
    async fn finalize(&self) -> Result<()>;

    // ========== Node Operations ==========

    /// Check if a node exists.
    async fn has_node(&self, node_id: &str) -> Result<bool>;

    /// Get a node by ID.
    async fn get_node(&self, node_id: &str) -> Result<Option<GraphNode>>;

    /// Insert or update a node.
    async fn upsert_node(&self, node: &GraphNode) -> Result<()>;

    /// Delete a node.
    async fn delete_node(&self, node_id: &str) -> Result<()>;

    /// Get all nodes (with optional limit).
    async fn get_all_nodes(&self, limit: Option<usize>) -> Result<Vec<GraphNode>>;

    // ========== Edge Operations ==========

    /// Check if an edge exists.
    async fn has_edge(&self, source: &str, target: &str) -> Result<bool>;

    /// Get edges from a node.
    async fn get_node_edges(&self, node_id: &str) -> Result<Vec<GraphEdge>>;

    /// Insert or update an edge.
    async fn upsert_edge(&self, edge: &GraphEdge) -> Result<()>;

    /// Delete an edge.
    async fn delete_edge(&self, source: &str, target: &str) -> Result<()>;

    // ========== Traversal Operations ==========

    /// Get neighbors of a node.
    async fn get_neighbors(&self, node_id: &str, depth: usize) -> Result<Vec<GraphNode>>;

    /// Find path between two nodes.
    async fn find_path(&self, from: &str, to: &str) -> Result<Option<Vec<GraphNode>>>;

    // ========== Analytics ==========

    /// Get total node count.
    async fn node_count(&self) -> Result<usize>;

    /// Get total edge count.
    async fn edge_count(&self) -> Result<usize>;

    /// Get degree of a node (number of edges).
    async fn node_degree(&self, node_id: &str) -> Result<usize>;

    // ========== Bulk Operations ==========

    /// Clear all data.
    async fn clear(&self) -> Result<()>;

    /// Get full graph.
    async fn get_graph(&self, limit: Option<usize>) -> Result<KnowledgeGraph>;
}
```

---

## Storage Backends

### MemoryGraphStorage

In-memory implementation for development and testing:

```rust
/// In-memory graph storage using DashMap.
pub struct MemoryGraphStorage {
    namespace: String,
    nodes: DashMap<String, GraphNode>,
    edges: DashMap<(String, String), GraphEdge>,
}
```

**Characteristics:**

| Attribute   | Value                                |
| ----------- | ------------------------------------ |
| Persistence | ❌ None (data lost on restart)       |
| Speed       | ⚡ Very fast (O(1) lookups)          |
| Scalability | Limited by memory                    |
| Use Case    | Development, testing, small datasets |

**Usage:**

```rust
let storage = MemoryGraphStorage::new("my_workspace");
storage.initialize().await?;
```

---

### PostgresAGEStorage

Production-grade storage using PostgreSQL with Apache AGE extension:

```rust
/// PostgreSQL Apache AGE graph storage.
pub struct PostgresAGEStorage {
    pool: PgPool,
    namespace: String,
    graph_name: String,
}
```

**Characteristics:**

| Attribute   | Value                    |
| ----------- | ------------------------ |
| Persistence | ✅ Full durability       |
| Speed       | Good (optimized queries) |
| Scalability | Millions of nodes/edges  |
| Use Case    | Production deployments   |

**Features:**

- Native graph queries via Cypher
- Automatic index creation
- Transaction support
- Connection pooling

**Schema:**

```sql
-- Apache AGE graph structure
SELECT * FROM cypher('edgequake', $$
    CREATE (n:Entity {
        id: 'SARAH_CHEN',
        entity_type: 'PERSON',
        description: 'Researcher at MIT'
    })
    RETURN n
$$) AS (n agtype);

-- Create relationship
SELECT * FROM cypher('edgequake', $$
    MATCH (a:Entity {id: 'SARAH_CHEN'})
    MATCH (b:Entity {id: 'MIT'})
    CREATE (a)-[r:WORKS_AT {
        relation_type: 'works_at',
        weight: 0.9
    }]->(b)
    RETURN r
$$) AS (r agtype);
```

**Usage:**

```rust
let pool = PgPoolOptions::new()
    .max_connections(10)
    .connect(&database_url)
    .await?;

let storage = PostgresAGEStorage::new(pool, "my_workspace").await?;
storage.initialize().await?;
```

---

## Storage Operations

### Node Operations

```rust
// Create or update a node
let mut node = GraphNode::new("SARAH_CHEN");
node.set_property("entity_type", json!("PERSON"));
node.set_property("description", json!("Researcher at MIT"));
node.set_property("importance", json!(0.9));

storage.upsert_node(&node).await?;

// Get a node
if let Some(node) = storage.get_node("SARAH_CHEN").await? {
    println!("Found: {}", node.id);
}

// Delete a node
storage.delete_node("SARAH_CHEN").await?;
```

### Edge Operations

```rust
// Create or update an edge
let mut edge = GraphEdge::new("SARAH_CHEN", "MIT");
edge.set_property("relation_type", json!("works_at"));
edge.set_property("description", json!("Research position"));
edge.set_property("weight", json!(0.9));
edge.set_property("keywords", json!(["researcher", "faculty"]));

storage.upsert_edge(&edge).await?;

// Get edges from a node
let edges = storage.get_node_edges("SARAH_CHEN").await?;
for edge in edges {
    println!("{} -> {}", edge.source, edge.target);
}

// Delete an edge
storage.delete_edge("SARAH_CHEN", "MIT").await?;
```

### Traversal Operations

```rust
// Get 1-hop neighbors
let neighbors = storage.get_neighbors("SARAH_CHEN", 1).await?;

// Get 2-hop neighbors
let extended = storage.get_neighbors("SARAH_CHEN", 2).await?;

// Find path between entities
if let Some(path) = storage.find_path("SARAH_CHEN", "GOOGLE").await? {
    println!("Path: {:?}", path.iter().map(|n| &n.id).collect::<Vec<_>>());
}
```

### Analytics

```rust
// Get counts
let node_count = storage.node_count().await?;
let edge_count = storage.edge_count().await?;

// Get node degree
let degree = storage.node_degree("SARAH_CHEN").await?;
println!("SARAH_CHEN has {} connections", degree);
```

---

## Multi-Tenancy

Graph storage supports namespace-based tenant isolation:

```
┌─────────────────────────────────────────────────────────────────┐
│                    MULTI-TENANT GRAPH STORAGE                   │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                     PostgreSQL Database                     ││
│  │                                                             ││
│  │  ┌───────────────┐ ┌───────────────┐ ┌───────────────┐      ││
│  │  │   tenant_a    │ │   tenant_b    │ │   tenant_c    │      ││
│  │  │   (Graph)     │ │   (Graph)     │ │   (Graph)     │      ││
│  │  │               │ │               │ │               │      ││
│  │  │ • 1000 nodes  │ │ • 500 nodes   │ │ • 2000 nodes  │      ││
│  │  │ • 3000 edges  │ │ • 1500 edges  │ │ • 6000 edges  │      ││
│  │  └───────────────┘ └───────────────┘ └───────────────┘      ││
│  │                                                             ││
│  │  Each namespace = separate AGE graph                        ││
│  │  Complete isolation, independent schema                     ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**Implementation:**

```rust
// Each workspace gets its own graph
let tenant_a = PostgresAGEStorage::new(pool.clone(), "tenant_a").await?;
let tenant_b = PostgresAGEStorage::new(pool.clone(), "tenant_b").await?;

// Data is completely isolated
tenant_a.upsert_node(&node).await?;
assert!(tenant_b.get_node(&node.id).await?.is_none());
```

---

## Performance Considerations

### Indexing

PostgreSQL AGE automatically creates indexes on:

- Node ID (primary key)
- Entity type (for type filtering)
- Edge source/target (for traversal)

### Query Optimization

```sql
-- Efficient: Index-based lookup
SELECT * FROM cypher('graph', $$
    MATCH (n:Entity {id: 'SARAH_CHEN'})
    RETURN n
$$) AS (n agtype);

-- Efficient: Limited traversal
SELECT * FROM cypher('graph', $$
    MATCH (n:Entity {id: 'SARAH_CHEN'})-[r]->(m)
    RETURN n, r, m
    LIMIT 100
$$) AS (n agtype, r agtype, m agtype);

-- Less efficient: Full scan
SELECT * FROM cypher('graph', $$
    MATCH (n:Entity)
    WHERE n.importance > 0.8
    RETURN n
$$) AS (n agtype);
```

### Connection Pooling

```rust
let pool = PgPoolOptions::new()
    .max_connections(20)           // Concurrent connections
    .min_connections(5)            // Keep-alive connections
    .acquire_timeout(Duration::from_secs(30))
    .idle_timeout(Duration::from_secs(600))
    .connect(&database_url)
    .await?;
```

---

## Best Practices

1. **Normalize Entity IDs** - Use UPPERCASE_UNDERSCORE format (BR0008)
2. **Limit Properties** - Don't store large text in properties
3. **Use Embeddings Separately** - Store embeddings in vector storage, not graph
4. **Batch Operations** - Use bulk insert for large imports
5. **Monitor Size** - Track node/edge counts for capacity planning

---

## Benchmarks

Performance on typical workloads (PostgreSQL AGE, 10K nodes, 30K edges):

| Operation          | Latency |
| ------------------ | ------- |
| `get_node`         | ~1ms    |
| `upsert_node`      | ~2ms    |
| `get_node_edges`   | ~3ms    |
| `get_neighbors(1)` | ~5ms    |
| `get_neighbors(2)` | ~15ms   |
| `node_count`       | ~50ms   |
| `get_graph(100)`   | ~10ms   |

---

## See Also

- [Entity Extraction](./entity-extraction.md) - How entities are created
- [Query Modes](./query-modes.md) - How graph is queried
- [Architecture: Crates](../architecture/crates/README.md) - Storage crate details
- [Performance Tuning](../operations/performance-tuning.md) - Optimization guide
