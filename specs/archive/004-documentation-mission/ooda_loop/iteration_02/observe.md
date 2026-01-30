# OODA Iteration 02 - Observe

**Date**: 2026-01-29
**Focus**: Architecture deep-dive

---

## Mission Re-read ✅

Read specs/004-documentation-mission.md - focusing on:

- Module roles and responsibilities using ASCII diagrams
- Crate dependencies and interactions
- Data flow through the system

---

## 1. Crate Analysis

### Core Crates (Business Logic)

| Crate              | SLOC    | Features                        | Key Files                 |
| ------------------ | ------- | ------------------------------- | ------------------------- |
| edgequake-core     | ~15,500 | FEAT0001, FEAT0007              | orchestrator.rs, types/   |
| edgequake-pipeline | ~10,500 | FEAT0003, FEAT0004, FEAT0017-19 | pipeline.rs, extractor.rs |
| edgequake-query    | ~11,900 | FEAT0007, FEAT0101-0106         | engine.rs, modes.rs       |

### Infrastructure Crates

| Crate             | SLOC    | Features      | Key Files             |
| ----------------- | ------- | ------------- | --------------------- |
| edgequake-api     | ~37,400 | FEAT0400-0403 | routes.rs, handlers/  |
| edgequake-storage | ~11,900 | FEAT0201-0205 | traits/, adapters/    |
| edgequake-llm     | ~8,500  | FEAT0017-0020 | traits.rs, providers/ |

### Specialized Crates

| Crate                  | SLOC    | Features           |
| ---------------------- | ------- | ------------------ |
| edgequake-pdf          | ~26,000 | PDF extraction     |
| edgequake-auth         | ~2,900  | JWT, API keys      |
| edgequake-audit        | ~580    | Compliance logging |
| edgequake-tasks        | ~3,400  | Background jobs    |
| edgequake-rate-limiter | ~1,000  | Throttling         |

---

## 2. Key Traits Discovered

### LLMProvider (edgequake-llm/src/traits.rs)

```rust
#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn complete(&self, prompt: &str) -> Result<LLMResponse>;
    async fn chat(&self, messages: &[ChatMessage], ...) -> Result<LLMResponse>;
    async fn stream(&self, prompt: &str) -> Result<BoxStream<'static, Result<String>>>;
}
```

### EmbeddingProvider (edgequake-llm/src/traits.rs)

```rust
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    fn dimension(&self) -> usize;  // e.g., 1536 for OpenAI
    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
}
```

### Storage Traits (edgequake-storage/src/traits/)

```rust
pub trait KVStorage: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<Value>>;
    async fn upsert(&self, items: &[(String, Value)]) -> Result<()>;
}

pub trait VectorStorage: Send + Sync {
    async fn search(&self, query: &[f32], top_k: usize) -> Result<Vec<SearchResult>>;
}

pub trait GraphStorage: Send + Sync {
    async fn add_node(&self, node: GraphNode) -> Result<()>;
    async fn add_edge(&self, edge: GraphEdge) -> Result<()>;
    async fn get_neighbors(&self, id: &str, depth: usize) -> Result<Vec<GraphNode>>;
}
```

---

## 3. Data Flow Analysis

### Ingestion Flow

```
Document → Chunker → Chunks
                       │
                       ▼
              LLMExtractor → ExtractionResult
                       │          (entities, relationships)
                       ▼
              KGMerger → Dedup + Merge
                       │
                       ▼
              Storage → KV (metadata)
                     → Vector (embeddings)
                     → Graph (entities/edges)
```

### Query Flow

```
Query → KeywordExtractor → Keywords
              │
              ▼
       QueryEngine → Mode Selection
              │
              ├─▶ Naive:  Vector similarity only
              ├─▶ Local:  Entity + neighbors
              ├─▶ Global: Community aggregation
              ├─▶ Hybrid: Local + Global
              ├─▶ Mix:    Weighted all
              └─▶ Bypass: Direct LLM
              │
              ▼
       Context Assembly → Truncation
              │
              ▼
       LLM Generation → Response
```

---

## 4. Feature Traceability

From code annotations discovered:

| FEAT ID  | Description              | Crate          |
| -------- | ------------------------ | -------------- |
| FEAT0001 | Document Ingestion       | core, pipeline |
| FEAT0003 | Entity Extraction        | pipeline       |
| FEAT0004 | Relationship Extraction  | pipeline       |
| FEAT0007 | Multi-Mode Query         | core, query    |
| FEAT0017 | Multi-Provider LLM       | llm            |
| FEAT0201 | Vector Similarity Search | storage        |
| FEAT0202 | Graph Traversal          | storage        |
| FEAT0400 | REST API                 | api            |
| FEAT0401 | OpenAPI Documentation    | api            |

---

## 5. Business Rules Enforced

| BR ID  | Rule                             | Enforcement        |
| ------ | -------------------------------- | ------------------ |
| BR0001 | Doc ID Uniqueness                | core/orchestrator  |
| BR0003 | Entity types configurable        | pipeline/extractor |
| BR0004 | Max 5 keywords per edge          | pipeline/extractor |
| BR0005 | Entity desc max 512 tokens       | pipeline/extractor |
| BR0008 | Entity name UPPERCASE_UNDERSCORE | pipeline/extractor |
| BR0201 | Tenant isolation                 | storage            |
| BR0301 | API rate limits                  | llm                |

---

## 6. API Endpoints (from routes.rs)

| Method | Path                     | Handler         |
| ------ | ------------------------ | --------------- |
| GET    | /health                  | health_check    |
| POST   | /api/v1/documents        | upload_document |
| GET    | /api/v1/documents        | list_documents  |
| DELETE | /api/v1/documents/{id}   | delete_document |
| POST   | /api/v1/query            | execute_query   |
| POST   | /api/v1/query/stream     | stream_query    |
| GET    | /api/v1/graph            | get_graph       |
| POST   | /api/v1/graph/entities   | create_entity   |
| GET    | /api/v1/tasks/{track_id} | get_task        |
