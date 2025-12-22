# ADR-0006: Graph-Centric Knowledge Representation

## Status

Accepted

## Date

2024-02

## Context

RAG systems traditionally use flat vector similarity search. LightRAG introduced **graph-augmented retrieval** which:

1. Captures entity relationships, not just document chunks
2. Enables multi-hop reasoning across related concepts
3. Supports different query modes (local, global, hybrid)
4. Provides explainable retrieval paths

We need to decide how to represent and store this knowledge graph.

## Decision

We use a **property graph model** with the following design:

### Graph Model

```
Nodes (Entities):
- id: Unique identifier (slug of entity name)
- label: Entity name
- entity_type: Category (person, organization, concept, etc.)
- description: Merged descriptions from all occurrences
- source_ids: List of source document IDs

Edges (Relationships):
- source_id: Source entity ID
- target_id: Target entity ID
- relationship: Relationship type/label
- description: Relationship description
- weight: Strength (occurrence count)
- source_ids: List of source document IDs
```

### Storage Strategy

| Mode        | Storage                 | Use Case              |
| ----------- | ----------------------- | --------------------- |
| Development | In-memory graph         | Fast iteration, tests |
| Production  | PostgreSQL + Apache AGE | Cypher queries, ACID  |
| Alternative | FalkorDB                | Redis protocol, fast  |
| Alternative | SurrealDB               | Multi-model, flexible |

### Query Modes Leveraging Graph

1. **Naive**: Traditional vector search only
2. **Local**: Find entities near query, expand neighbors
3. **Global**: Use high-degree hub nodes for broad context
4. **Hybrid**: Combine local specificity + global overview
5. **Mix**: Weighted combination of all strategies

### Graph Operations

```rust
// Entity extraction and graph building
let entities = extractor.extract_entities(&chunk).await?;
let relationships = extractor.extract_relationships(&chunk, &entities).await?;

// Graph merging with description summarization
merger.merge_entity(&entity).await?;
merger.merge_relationship(&rel).await?;

// Query-time graph traversal
let neighbors = graph.get_neighbors(&entity_id, depth: 2).await?;
let hubs = graph.get_popular_labels(limit: 10).await?;
```

## Consequences

### Positive

- **Relationship-aware retrieval**: Find related concepts, not just similar text
- **Multi-hop reasoning**: Traverse connections for complex queries
- **Query flexibility**: Different modes for different query types
- **Explainability**: Show why certain entities were retrieved
- **Entity consolidation**: Merge mentions across documents

### Negative

- **Storage overhead**: Graph storage in addition to vectors
- **Extraction costs**: LLM calls for entity/relationship extraction
- **Complexity**: More complex than pure vector RAG
- **Consistency**: Graph and vectors must stay synchronized

### Mitigations

- Efficient graph storage (AGE uses PostgreSQL)
- Batch LLM calls for extraction
- Clear abstraction boundaries
- Transactional updates where possible

### Design Principles

1. **Immutable documents**: Documents are append-only
2. **Merged entities**: Same entity from multiple docs = one node
3. **Weighted edges**: More occurrences = higher weight
4. **Async traversal**: Non-blocking graph operations
5. **Namespaced**: Multi-tenant isolation at graph level
