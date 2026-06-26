# 02 — Query Pipeline

> **Spec**: 021-storage-study  
> **File**: 03-pipelines/02-query-pipeline.md  
> **Date**: 2026-06-25  
> **Source**: `edgequake-query/src/sota_engine/`,  
> `edgequake-query/src/strategies/`,  
> `edgequake-core/src/orchestrator/query_ops.rs`,  
> `edgequake-api/src/handlers/query.rs`

---

## Query Entry Point

```
HTTP POST /api/v1/query
    |
    +--> Validate request (workspace_id, query, mode)
    +--> Resolve workspace-scoped vector storage via WorkspaceVectorRegistry
    +--> Create SOTAQueryEngine with workspace storage
    +--> Call EdgeQuake::query() OR direct SOTAQueryEngine::execute()
    +--> Stream or return JSON response
```

---

## Six Query Modes

| Mode     | Storage Read                                       | Description                                     |
| -------- | -------------------------------------------------- | ----------------------------------------------- |
| `naive`  | Vector store only (type=chunk)                     | Pure similarity search on chunk text            |
| `local`  | Vector store (type=entity) + Graph (1-hop)         | Entity-centric: find entities, then their edges |
| `global` | Vector store (type=relationship) + Graph analytics | Community/theme-level: relationship clusters    |
| `hybrid` | Local + Global combined                            | Default; best general-purpose                   |
| `mix`    | Naive + local + global weighted                    | Adaptive blend                                  |
| `bypass` | None                                               | Direct LLM call, no retrieval                   |

---

## SOTA Query Pipeline (detailed)

### Step 1 — Keyword Extraction

```
Query: "What are Apple's products?"
    |
    +--> LLMKeywordExtractor.extract(query)
    |    (or CachedKeywordExtractor with InMemoryKeywordCache / PostgresKeywordCache)
    |
    +--> Output: ExtractedKeywords {
    |      high_level_keywords: ["Apple", "products", "technology"],   // for Global mode
    |      low_level_keywords: ["APPLE_INC", "IPHONE", "MACBOOK"],    // for Local mode
    |      intent: EntityQuery | RelationshipQuery | GeneralQuery
    |    }
    |
    +--> Cache check: eq_*_kv key "{query_hash}-kwcache"
```

**Storage reads**: `eq_*_kv` (keyword cache lookup)  
**Storage writes**: `eq_*_kv` (cache miss → write new entry, TTL=24h)

---

### Step 2 — Embedding Generation

```
EmbeddingProvider.embed(query_text) -> query_embedding: Vec<f32>
EmbeddingProvider.embed(high_level_keywords) -> hl_embedding: Vec<f32>
EmbeddingProvider.embed(low_level_keywords) -> ll_embedding: Vec<f32>

Struct: QueryEmbeddings { query, high_level, low_level }
```

**Storage reads**: None (LLM provider call).

---

### Step 3 — Mode-Specific Retrieval

#### Naive Mode
```
vector_storage.query_filtered(
    embedding = query_embedding,
    top_k     = max_chunks (default 20),
    filter    = MetadataFilter { type: "chunk", workspace_id, tenant_id }
)
--> Vec<VectorSearchResult> { id, score, metadata.content }
```

**Storage reads**: `eq_*_vectors` (chunk vectors)

#### Local Mode
```
Step 3a: Vector search for ENTITIES
  vector_storage.query_filtered(
      embedding = ll_embedding,
      top_k     = max_entities * 2 (default 120),
      filter    = MetadataFilter { type: "entity", workspace_id }
  )
  --> entity names from metadata.entity_name

Step 3b: Graph lookup for entity nodes
  graph_storage.get_node(entity_name) for each entity
  -> GraphNode { id, properties: {entity_type, description, ...} }

Step 3c: Graph edge traversal (1-hop)
  graph_storage.get_node_edges(entity_name)
  -> Vec<GraphEdge> { source, target, properties }
```

**Storage reads**: `eq_*_vectors` (entity vectors) + AGE graph (Node + EDGE)

#### Global Mode
```
Step 3a: Vector search for RELATIONSHIPS
  vector_storage.query_filtered(
      embedding = hl_embedding,
      top_k     = max_relationships (default 60),
      filter    = MetadataFilter { type: "relationship", workspace_id }
  )
  --> relationship source/target from metadata

Step 3b: Graph lookup for relationship nodes + edges
  graph_storage.get_edge(source, target)
  graph_storage.get_node(source), graph_storage.get_node(target)
```

**Storage reads**: `eq_*_vectors` (relationship vectors) + AGE graph (Node + EDGE)

#### Hybrid Mode
```
Run Local mode  -> local_context
Run Global mode -> global_context
Merge contexts, dedup, rank by score
```

---

### Step 4 — Chunk Retrieval from Graph Context

```
For each entity/relationship found in steps 3b/3c:
    Get source_ids from node/edge properties
    (source_ids = ["doc_id-chunk-0", "doc_id-chunk-3", ...])

Deduplicate chunk IDs
kv_storage.get_by_ids(chunk_ids) -> Vec<ChunkContent>
```

**Storage reads**: `eq_*_kv` (chunk text by ID)

---

### Step 5 — Reranking (BM25)

```
BM25Reranker.rerank(query, chunks)
    |-- Porter2 stemming
    |-- NFKD Unicode normalization
    |-- Stop word filtering
    |-- BM25 score per chunk
    |
    --> Top-K chunks (default 20) above min_score (default 0.1)
```

**Storage reads**: None (in-memory reranking).

---

### Step 6 — Token Budget & Context Truncation

```
TruncationConfig:
  max_entity_tokens:   10000
  max_relation_tokens: 10000
  max_total_tokens:    30000

truncate_entities(entities, max_entity_tokens)
truncate_relationships(rels, max_relation_tokens)
truncate_chunks(chunks, remaining_budget)

balance_context(entities, rels, chunks)
```

**Storage reads**: None (in-memory).

---

### Step 7 — LLM Generation

```
Build prompt:
  - System: RAG instructions
  - Context: entities + relationships + chunks (token-budgeted)
  - Conversation history (from ConversationStorage if enabled)
  - Query: user question

LLMProvider.generate(prompt) -> response_text

[Optional: stream via SSE if client requested streaming]
```

**Storage reads**: `conversations` + `messages` tables (if conversation history enabled)  
**Storage writes**: `messages` table (append user message + assistant response)

---

## Storage Reads per Query Mode (Summary)

| Mode     | Vector Store                  | KV Store                  | Graph Store | Conversations |
| -------- | ----------------------------- | ------------------------- | ----------- | ------------- |
| `naive`  | chunk vectors                 | chunk text (by source_id) | —           | optional      |
| `local`  | entity vectors                | chunk text                | Node + EDGE | optional      |
| `global` | relationship vectors          | chunk text                | Node + EDGE | optional      |
| `hybrid` | entity + relationship vectors | chunk text                | Node + EDGE | optional      |
| `mix`    | all types                     | chunk text                | Node + EDGE | optional      |
| `bypass` | —                             | —                         | —           | optional      |

---

## Workspace Vector Routing

The query pipeline uses workspace-scoped vector storage when `workspace_id` is
set in the query parameters:

```rust
let vector_storage = workspace_pipeline_factory
    .get_or_create_workspace_storage(workspace_id, embedding_dim)
    .await?;
// Returns Arc<dyn VectorStorage> pointing to eq_{ns}_ws_{uuid8}_vectors
```

If no workspace_id is set, falls back to the global default vector storage.
