---
title: 'Knowledge Graph'
---

# Knowledge Graph

> **EdgeQuake's knowledge graph stores entities as nodes and relationships as edges,
> enabling traversal-based retrieval across documents.**

---

## What is a Knowledge Graph?

A knowledge graph is a structured representation of knowledge using:

- **Nodes**: Entities (people, concepts, organizations)
- **Edges**: Relationships between entities
- **Properties**: Attributes on nodes and edges (descriptions, weights)

```
┌─────────────────────────────────────────────────────────────────┐
│                    KNOWLEDGE GRAPH                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│     ┌─────────┐                         ┌─────────┐             │
│     │  NODE   │────── EDGE ────────────▶│  NODE   │             │
│     │ (Entity)│     (Relationship)      │ (Entity)│             │
│     └─────────┘                         └─────────┘             │
│         │                                    │                  │
│         │                                    │                  │
│         v                                    v                  │
│   ┌───────────┐                      ┌───────────┐              │
│   │ Properties│                      │ Properties│              │
│   │ - name    │                      │ - name    │              │
│   │ - type    │                      │ - type    │              │
│   │ - desc    │                      │ - desc    │              │
│   │ - embed   │                      │ - embed   │              │
│   └───────────┘                      └───────────┘              │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Nodes and Edges

### Entity Nodes

Each entity extracted from documents becomes a node:

```rust
// Conceptual structure (from edgequake-storage)
struct Entity {
    name: String,           // "SARAH_CHEN"
    entity_type: String,    // "PERSON"
    description: String,    // "Lead researcher at Quantum Lab..."
    embedding: Vec<f32>,    // [0.1, 0.2, ...] for vector search
    source_chunks: Vec<String>,  // Chunk IDs for citations
}
```

### Relationship Edges

Relationships connect entities with typed edges:

```rust
struct Relationship {
    source: String,         // "SARAH_CHEN"
    target: String,         // "QUANTUM_LAB"
    relation_type: String,  // "WORKS_AT"
    description: String,    // "Sarah works as lead researcher..."
    weight: f32,            // 0.8 (strength/confidence)
    keywords: Vec<String>,  // ["employment", "research"]
}
```

---

## Storage in EdgeQuake

EdgeQuake uses a hybrid storage architecture:

```
┌─────────────────────────────────────────────────────────────────┐
│                    STORAGE ARCHITECTURE                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                    PostgreSQL Database                       ││
│  │  ┌─────────────────────────────────────────────────────────┐││
│  │  │                    Apache AGE                            │││
│  │  │   • Graph storage (Cypher queries)                      │││
│  │  │   • Entity nodes with properties                        │││
│  │  │   • Relationship edges with properties                  │││
│  │  └─────────────────────────────────────────────────────────┘││
│  │  ┌─────────────────────────────────────────────────────────┐││
│  │  │                    pgvector                              │││
│  │  │   • Vector embeddings (1536 dims)                       │││
│  │  │   • Similarity search (cosine, L2)                      │││
│  │  │   • HNSW index for fast retrieval                       │││
│  │  └─────────────────────────────────────────────────────────┘││
│  │  ┌─────────────────────────────────────────────────────────┐││
│  │  │                    Standard Tables                       │││
│  │  │   • Documents metadata                                  │││
│  │  │   • Chunks with text content                            │││
│  │  │   • Multi-tenant isolation                              │││
│  │  └─────────────────────────────────────────────────────────┘││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

### Development Mode

> **Note:** In-memory storage was removed in v0.4.0. PostgreSQL is required
> for all deployments, including development. See the
> [installation guide](/docs/getting-started/installation/) for setup instructions.

---

## Vector + Graph Hybrid

The power of EdgeQuake comes from combining:

| Storage Type          | Purpose             | Query Method      |
| --------------------- | ------------------- | ----------------- |
| **Graph (AGE)**       | Relationships       | Cypher traversal  |
| **Vector (pgvector)** | Semantic similarity | Cosine similarity |
| **Relational (SQL)**  | Metadata, filtering | SQL WHERE clauses |

### Query Example

```
User: "How does Sarah's research relate to Bob's work?"

1. Vector Search: Find entities matching "Sarah" and "Bob"
   → SARAH_CHEN (score: 0.92)
   → BOB_SMITH (score: 0.89)

2. Graph Traversal: Find paths between them
   → SARAH_CHEN --[works_at]--> QUANTUM_LAB
   → BOB_SMITH --[works_at]--> QUANTUM_LAB
   → SARAH_CHEN --[collaborates_with]--> BOB_SMITH

3. Context Fusion: Combine vector + graph results
   → "Sarah and Bob both work at Quantum Lab and collaborate..."
```

---

## Multi-Tenancy

EdgeQuake supports data isolation across tenants:

```
┌─────────────────────────────────────────────────────────────────┐
│                    MULTI-TENANT ISOLATION                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Tenant: "acme-corp"          Tenant: "globex"                  │
│  ┌─────────────────┐          ┌─────────────────┐               │
│  │  Workspace A    │          │  Workspace X    │               │
│  │  ├─ Documents   │          │  ├─ Documents   │               │
│  │  ├─ Entities    │          │  ├─ Entities    │               │
│  │  └─ Graph       │          │  └─ Graph       │               │
│  ├─────────────────┤          └─────────────────┘               │
│  │  Workspace B    │                                            │
│  │  ├─ Documents   │          Each tenant has isolated          │
│  │  └─ ...         │          data with no cross-access         │
│  └─────────────────┘                                            │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

Queries are automatically scoped:

```rust
QueryRequest::new("What is our strategy?")
    .with_tenant_id("acme-corp")
    .with_workspace_id("strategy-team")
```

---

## Graph Operations

### Creating Entities

When documents are ingested, entities are upserted (insert or update):

```sql
-- Apache AGE Cypher
MERGE (e:Entity {name: 'SARAH_CHEN'})
SET e.type = 'PERSON',
    e.description = 'Lead researcher...',
    e.updated_at = now()
```

### Creating Relationships

```sql
MATCH (source:Entity {name: 'SARAH_CHEN'})
MATCH (target:Entity {name: 'QUANTUM_LAB'})
MERGE (source)-[r:WORKS_AT]->(target)
SET r.description = 'Sarah works at Quantum Lab',
    r.weight = 0.8
```

### Traversing for Queries

```sql
-- Find 2-hop neighbors of an entity
MATCH (start:Entity {name: 'SARAH_CHEN'})-[*1..2]-(neighbor)
RETURN neighbor.name, neighbor.description
LIMIT 10
```

---

## Learn More

- **How entities are extracted**: [Entity Extraction](/docs/concepts/entity-extraction/)
- **How queries combine vector + graph**: [Hybrid Retrieval](/docs/concepts/hybrid-retrieval/)
- **Underlying algorithm**: [LightRAG Algorithm](/docs/deep-dives/lightrag-algorithm/)

---

## Source Code

- **Graph storage trait**: [graph.rs](https://github.com/raphaelmansuy/edgequake/blob/edgequake-main/edgequake/crates/edgequake-storage/src/traits/graph.rs)
- **Vector storage trait**: [vector.rs](https://github.com/raphaelmansuy/edgequake/blob/edgequake-main/edgequake/crates/edgequake-storage/src/traits/vector.rs)
- **PostgreSQL implementation**: [postgres/](https://github.com/raphaelmansuy/edgequake/tree/edgequake-main/edgequake/crates/edgequake-storage/src/postgres/)
