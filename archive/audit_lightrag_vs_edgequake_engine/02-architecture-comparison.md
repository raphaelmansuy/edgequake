# Architecture Comparison: LightRAG vs EdgeQuake

## 1. High-Level Architecture

### LightRAG Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              LightRAG System                                 │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐   ┌─────────────────┐   ┌─────────────────┐           │
│  │   REST API      │   │  Tenant Manager │   │   Namespace     │           │
│  │ (FastAPI/Uvicorn)│   │ (Multi-tenant)  │   │  (Workspace)    │           │
│  └────────┬────────┘   └────────┬────────┘   └────────┬────────┘           │
│           │                     │                     │                     │
│  ┌────────┴─────────────────────┴─────────────────────┴────────┐           │
│  │                         LightRAG Class                       │           │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │           │
│  │  │   Insert     │  │    Query     │  │   Delete     │       │           │
│  │  │  Pipeline    │  │   Pipeline   │  │  Pipeline    │       │           │
│  │  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘       │           │
│  └─────────┼─────────────────┼─────────────────┼───────────────┘           │
│            │                 │                 │                            │
│  ┌─────────┴─────────────────┴─────────────────┴───────────────┐           │
│  │                       operate.py                             │           │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────┐             │           │
│  │  │  Chunking  │  │ Extraction │  │  Merging   │             │           │
│  │  └────────────┘  └────────────┘  └────────────┘             │           │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────┐             │           │
│  │  │  kg_query  │  │naive_query │  │  Context   │             │           │
│  │  └────────────┘  └────────────┘  └────────────┘             │           │
│  └─────────────────────────────────────────────────────────────┘           │
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │                         Storage Layer                                  │ │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐                 │ │
│  │  │  KV Storage  │  │Vector Storage│  │Graph Storage │                 │ │
│  │  │(JsonKV/Redis)│  │(Nano/Milvus) │  │(NetworkX/AGE)│                 │ │
│  │  └──────────────┘  └──────────────┘  └──────────────┘                 │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │                          LLM Layer                                     │ │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐              │ │
│  │  │  OpenAI  │  │ Anthropic│  │  Ollama  │  │  Gemini  │              │ │
│  │  └──────────┘  └──────────┘  └──────────┘  └──────────┘              │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────┘
```

### EdgeQuake Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                             EdgeQuake System                                 │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │                         edgequake-api (Axum)                            ││
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐                  ││
│  │  │   Ingest     │  │    Query     │  │   Health     │                  ││
│  │  │   Routes     │  │   Routes     │  │   Routes     │                  ││
│  │  └──────┬───────┘  └──────┬───────┘  └──────────────┘                  ││
│  └─────────┼─────────────────┼─────────────────────────────────────────────┘│
│            │                 │                                              │
│  ┌─────────┴─────────────────┴─────────────────────────────────────────────┐│
│  │                        edgequake-core                                    ││
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐                  ││
│  │  │ Orchestrator │  │   Tenant     │  │  Workspace   │                  ││
│  │  │              │  │   Manager    │  │   Service    │                  ││
│  │  └──────┬───────┘  └──────────────┘  └──────────────┘                  ││
│  └─────────┼───────────────────────────────────────────────────────────────┘│
│            │                                                                │
│  ┌─────────┴───────────────────────────────────────────────────────────────┐│
│  │  ┌──────────────────────┐  ┌──────────────────────┐                    ││
│  │  │   edgequake-pipeline │  │    edgequake-query   │                    ││
│  │  │  ┌────────────┐      │  │  ┌────────────┐      │                    ││
│  │  │  │  Chunker   │      │  │  │ SOTAEngine │      │                    ││
│  │  │  ├────────────┤      │  │  ├────────────┤      │                    ││
│  │  │  │ Extractor  │      │  │  │ Strategies │      │                    ││
│  │  │  ├────────────┤      │  │  ├────────────┤      │                    ││
│  │  │  │  Merger    │      │  │  │ Truncation │      │                    ││
│  │  │  ├────────────┤      │  │  └────────────┘      │                    ││
│  │  │  │ Summarizer │      │  │                      │                    ││
│  │  │  └────────────┘      │  │                      │                    ││
│  │  └──────────────────────┘  └──────────────────────┘                    ││
│  └─────────────────────────────────────────────────────────────────────────┘│
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │                        edgequake-storage                                ││
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐                  ││
│  │  │  KVStorage   │  │VectorStorage │  │ GraphStorage │                  ││
│  │  │   (trait)    │  │   (trait)    │  │   (trait)    │                  ││
│  │  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘                  ││
│  │  ┌──────┴───────┐  ┌──────┴───────┐  ┌──────┴───────┐                  ││
│  │  │MemoryKV     │  │MemoryVector  │  │MemoryGraph  │                  ││
│  │  │PostgresKV   │  │PgVector      │  │PostgresAGE  │                  ││
│  │  └──────────────┘  └──────────────┘  └──────────────┘                  ││
│  └─────────────────────────────────────────────────────────────────────────┘│
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │                          edgequake-llm                                  ││
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐                              ││
│  │  │LLMProvider│  │Embedding │  │  Mock    │                              ││
│  │  │ (trait)  │  │ Provider │  │ Provider │                              ││
│  │  └──────────┘  └──────────┘  └──────────┘                              ││
│  └─────────────────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────────────────┘
```

## 2. Crate/Module Structure Comparison

### LightRAG Module Structure

```
lightrag/
├── __init__.py
├── lightrag.py          # Main LightRAG class (4043 lines)
├── operate.py           # Core operations (5000 lines) ⚠️ Too large
├── base.py              # Base classes and types
├── prompt.py            # Prompt templates
├── constants.py         # Default values
├── utils.py             # Utility functions
├── namespace.py         # Workspace isolation
├── types.py             # Type definitions
├── kg/                  # Storage implementations
│   ├── postgres_impl.py # PostgreSQL + AGE (5121 lines)
│   ├── networkx_impl.py # In-memory graph
│   ├── milvus_impl.py   # Milvus vector
│   ├── qdrant_impl.py   # Qdrant vector
│   ├── redis_impl.py    # Redis KV
│   └── ...
├── llm/                 # LLM providers
│   ├── openai.py
│   ├── anthropic.py
│   ├── ollama.py
│   └── ...
├── api/                 # REST API
│   ├── lightrag_server.py
│   └── routers/
└── services/            # Service layer
```

### EdgeQuake Crate Structure

```
edgequake/crates/
├── edgequake-api/       # REST API (Axum)
│   └── src/
│       ├── lib.rs
│       ├── routes/
│       └── handlers/
├── edgequake-core/      # Core orchestration
│   └── src/
│       ├── lib.rs
│       ├── orchestrator.rs
│       ├── workspace_service.rs
│       └── tenant_manager.rs
├── edgequake-pipeline/  # Document processing
│   └── src/
│       ├── lib.rs
│       ├── pipeline.rs      # Main pipeline (707 lines)
│       ├── chunker.rs       # Text chunking
│       ├── extractor.rs     # Entity extraction (996 lines)
│       ├── merger.rs        # Entity merging
│       ├── summarizer.rs    # Description summarization
│       └── lineage.rs       # Lineage tracking
├── edgequake-query/     # Query engine
│   └── src/
│       ├── lib.rs
│       ├── sota_engine.rs   # SOTA engine (1627 lines)
│       ├── engine.rs        # Base engine
│       ├── strategies/      # Query strategies
│       ├── keywords.rs      # Keyword extraction
│       ├── truncation.rs    # Context truncation
│       └── chunk_retrieval.rs
├── edgequake-storage/   # Storage abstractions
│   └── src/
│       ├── lib.rs
│       ├── traits/          # Storage traits
│       ├── adapters/
│       │   ├── memory/      # In-memory implementations
│       │   └── postgres/    # PostgreSQL + pgvector
│       └── community.rs     # Community detection
├── edgequake-llm/       # LLM providers
│   └── src/
│       ├── lib.rs
│       ├── traits.rs        # Provider traits
│       ├── openai.rs
│       └── mock.rs
└── edgequake-auth/      # Authentication
```

## 3. Design Pattern Comparison

### LightRAG Design Patterns

| Pattern            | Usage            | Example                              |
| ------------------ | ---------------- | ------------------------------------ |
| **Dataclass**      | Configuration    | `@dataclass class LightRAG`          |
| **Factory**        | Storage creation | `STORAGES[storage_type]()`           |
| **Strategy**       | Query modes      | `mode` parameter switches behavior   |
| **Async Iterator** | Streaming        | `AsyncIterator[str]` for responses   |
| **Semaphore**      | Concurrency      | `asyncio.Semaphore(max_async)`       |
| **Cache**          | LLM responses    | `handle_cache()` / `save_to_cache()` |

### EdgeQuake Design Patterns

| Pattern            | Usage                | Example                               |
| ------------------ | -------------------- | ------------------------------------- |
| **Trait**          | Abstraction          | `trait EntityExtractor`               |
| **Builder**        | Configuration        | `Pipeline::new().with_extractor()`    |
| **Strategy**       | Query modes          | `QueryStrategy` trait implementations |
| **Arc<dyn Trait>** | Dependency Injection | `Arc<dyn LLMProvider>`                |
| **Semaphore**      | Concurrency          | `tokio::sync::Semaphore`              |
| **Result<T,E>**    | Error handling       | `Result<ProcessingResult>`            |

## 4. Data Flow Comparison

### Ingestion Data Flow

#### LightRAG

```
Document
    │
    ▼
┌───────────────────────────────────────────────────────────┐
│ 1. Chunking (chunking_by_token_size)                      │
│    - Split by token size (1200 default)                   │
│    - Overlap (100 tokens default)                         │
│    - Optional character split                             │
└───────────────────────────────────────────────────────────┘
    │
    ▼
┌───────────────────────────────────────────────────────────┐
│ 2. Entity Extraction (extract_entities)                   │
│    - LLM call with entity_extraction prompt               │
│    - Parse tuple-delimited output                         │
│    - Gleaning: second pass for missed entities            │
│    - Merge gleaning results (prefer longer descriptions)  │
└───────────────────────────────────────────────────────────┘
    │
    ▼
┌───────────────────────────────────────────────────────────┐
│ 3. Merge Phase 1: Entities (_merge_nodes_then_upsert)     │
│    - Get existing node data                               │
│    - Merge source_ids (KEEP or FIFO)                      │
│    - Deduplicate descriptions                             │
│    - LLM summary if needed (map-reduce)                   │
│    - Update Graph + VDB                                   │
└───────────────────────────────────────────────────────────┘
    │
    ▼
┌───────────────────────────────────────────────────────────┐
│ 4. Merge Phase 2: Relationships (_merge_edges_then_upsert)│
│    - Similar process for edges                            │
│    - May create missing entity nodes                      │
│    - Update Graph + VDB                                   │
└───────────────────────────────────────────────────────────┘
    │
    ▼
┌───────────────────────────────────────────────────────────┐
│ 5. Storage Updates                                        │
│    - full_entities_storage                                │
│    - full_relations_storage                               │
│    - text_chunks storage                                  │
│    - LLM cache                                            │
└───────────────────────────────────────────────────────────┘
```

#### EdgeQuake

```
Document
    │
    ▼
┌───────────────────────────────────────────────────────────┐
│ 1. Chunking (Chunker)                                     │
│    - Sliding window approach                              │
│    - Configurable size and overlap                        │
│    - Generate TextChunk with metadata                     │
└───────────────────────────────────────────────────────────┘
    │
    ▼
┌───────────────────────────────────────────────────────────┐
│ 2. Parallel Extraction (extract_parallel)                 │
│    - Semaphore-controlled concurrency                     │
│    - LLM call with JSON output format                     │
│    - Parse JSON response                                  │
│    - Track token usage                                    │
│    ⚠️ No gleaning implemented                             │
└───────────────────────────────────────────────────────────┘
    │
    ▼
┌───────────────────────────────────────────────────────────┐
│ 3. Embedding Generation                                   │
│    - Batch chunk embeddings                               │
│    - Batch entity embeddings                              │
│    - Batch relationship embeddings                        │
│    - Cost tracking                                        │
└───────────────────────────────────────────────────────────┘
    │
    ▼
┌───────────────────────────────────────────────────────────┐
│ 4. Lineage Tracking (optional)                            │
│    - Build DocumentLineage                                │
│    - Record source spans                                  │
│    - Track extraction metadata                            │
└───────────────────────────────────────────────────────────┘
    │
    ▼
┌───────────────────────────────────────────────────────────┐
│ 5. Storage Updates                                        │
│    - Graph storage (entities + relationships)             │
│    - Vector storage (embeddings)                          │
│    - KV storage (chunks)                                  │
└───────────────────────────────────────────────────────────┘
```

### Query Data Flow

#### LightRAG

```
Query
    │
    ▼
┌───────────────────────────────────────────────────────────┐
│ 1. Keyword Extraction                                     │
│    - LLM call for high-level keywords                     │
│    - LLM call for low-level keywords                      │
│    - Cache results                                        │
└───────────────────────────────────────────────────────────┘
    │
    ▼
┌───────────────────────────────────────────────────────────┐
│ 2. Mode-Specific Search (_perform_kg_search)              │
│    - local: entity VDB + low-level keywords               │
│    - global: relationship VDB + high-level keywords       │
│    - hybrid: both                                         │
│    - mix: naive + KG                                      │
│    - naive: chunk VDB only                                │
└───────────────────────────────────────────────────────────┘
    │
    ▼
┌───────────────────────────────────────────────────────────┐
│ 3. Token Truncation (_apply_token_truncation)             │
│    - max_entity_tokens                                    │
│    - max_relation_tokens                                  │
│    - Create ID mappings                                   │
└───────────────────────────────────────────────────────────┘
    │
    ▼
┌───────────────────────────────────────────────────────────┐
│ 4. Chunk Merging (_merge_all_chunks)                      │
│    - Get chunks from entities                             │
│    - Get chunks from relationships                        │
│    - Round-robin merge with deduplication                 │
└───────────────────────────────────────────────────────────┘
    │
    ▼
┌───────────────────────────────────────────────────────────┐
│ 5. Context Building (_build_context_str)                  │
│    - Dynamic token allocation                             │
│    - Optional reranking                                   │
│    - Generate reference list                              │
│    - Build LLM prompt                                     │
└───────────────────────────────────────────────────────────┘
    │
    ▼
┌───────────────────────────────────────────────────────────┐
│ 6. LLM Generation                                         │
│    - Cache check                                          │
│    - LLM call (streaming or non-streaming)                │
│    - Return QueryResult                                   │
└───────────────────────────────────────────────────────────┘
```

#### EdgeQuake

```
Query
    │
    ▼
┌───────────────────────────────────────────────────────────┐
│ 1. Keyword Extraction (with caching)                      │
│    - CachedKeywordExtractor                               │
│    - Extract high_level and low_level keywords            │
│    - Determine QueryIntent                                │
└───────────────────────────────────────────────────────────┘
    │
    ▼
┌───────────────────────────────────────────────────────────┐
│ 2. Mode Selection (adaptive)                              │
│    - Use provided mode OR                                 │
│    - QueryIntent.recommended_mode()                       │
└───────────────────────────────────────────────────────────┘
    │
    ▼
┌───────────────────────────────────────────────────────────┐
│ 3. Compute Embeddings (QueryEmbeddings::compute)          │
│    - Batch embed: query, high_level, low_level            │
│    - Single provider call for efficiency                  │
└───────────────────────────────────────────────────────────┘
    │
    ▼
┌───────────────────────────────────────────────────────────┐
│ 4. Mode-Specific Retrieval                                │
│    - query_local: entity VDB + low_level embedding        │
│    - query_global: relationship VDB + high_level embedding│
│    - query_hybrid: combined                               │
│    - query_mix: weighted naive + graph                    │
│    - query_naive: chunk VDB only                          │
└───────────────────────────────────────────────────────────┘
    │
    ▼
┌───────────────────────────────────────────────────────────┐
│ 5. Context Balancing (balance_context)                    │
│    - TruncationConfig-based limits                        │
│    - Token-aware truncation                               │
│    - Prioritize high-relevance items                      │
└───────────────────────────────────────────────────────────┘
    │
    ▼
┌───────────────────────────────────────────────────────────┐
│ 6. Answer Generation                                      │
│    - build_prompt()                                       │
│    - LLM call (generate_answer or stream)                 │
│    - Return QueryResponse                                 │
└───────────────────────────────────────────────────────────┘
```

## 5. Storage Architecture Comparison

### LightRAG Storage Classes

| Storage Type   | Class               | Backend Options                                                          |
| -------------- | ------------------- | ------------------------------------------------------------------------ |
| KV Storage     | `BaseKVStorage`     | JsonKVStorage, RedisKVStorage, PostgresKVStorage                         |
| Vector Storage | `BaseVectorStorage` | NanoVectorDBStorage, MilvusStorage, QdrantStorage, PostgresVectorStorage |
| Graph Storage  | `BaseGraphStorage`  | NetworkXStorage, Neo4jStorage, MemgraphStorage, PostgresAGEStorage       |
| Doc Status     | `DocStatusStorage`  | JsonDocStatusStorage, PostgresDocStatusStorage                           |

### EdgeQuake Storage Traits

| Storage Type   | Trait           | Implementations                             |
| -------------- | --------------- | ------------------------------------------- |
| KV Storage     | `KVStorage`     | MemoryKVStorage, PostgresKVStorage          |
| Vector Storage | `VectorStorage` | MemoryVectorStorage, PgVectorStorage        |
| Graph Storage  | `GraphStorage`  | MemoryGraphStorage, PostgresAGEGraphStorage |

### Schema Comparison

#### Entity/Node Schema

| Field       | LightRAG                                  | EdgeQuake                          |
| ----------- | ----------------------------------------- | ---------------------------------- |
| ID          | `entity_id: str`                          | `id: String`                       |
| Type        | `entity_type: str`                        | `entity_type: String`              |
| Description | `description: str`                        | `description: String`              |
| Source IDs  | `source_id: str` (GRAPH_FIELD_SEP joined) | `source_chunk_ids: Vec<String>`    |
| File Path   | `file_path: str` (GRAPH_FIELD_SEP joined) | `source_file_path: Option<String>` |
| Created At  | `created_at: int`                         | In properties                      |
| Truncation  | `truncate: str`                           | N/A                                |
| Properties  | Flat fields                               | `properties: serde_json::Value`    |

#### Relationship/Edge Schema

| Field       | LightRAG           | EdgeQuake                          |
| ----------- | ------------------ | ---------------------------------- |
| Source      | `src_id: str`      | `source: String`                   |
| Target      | `tgt_id: str`      | `target: String`                   |
| Type        | N/A                | `relation_type: String`            |
| Weight      | `weight: float`    | `weight: f32`                      |
| Description | `description: str` | `description: String`              |
| Keywords    | `keywords: str`    | `keywords: Vec<String>`            |
| Source ID   | `source_id: str`   | `source_chunk_id: Option<String>`  |
| File Path   | `file_path: str`   | `source_file_path: Option<String>` |
| Properties  | Flat fields        | `properties: serde_json::Value`    |

## 6. Error Handling Comparison

### LightRAG Error Handling

```python
# Exception-based with try/except
try:
    result = await some_operation()
except ValueError as e:
    logger.error(f"Value error: {e}")
    raise
except Exception as e:
    logger.error(f"Unexpected error: {e}")
    # Often continues with fallback behavior
```

**Patterns:**

- Custom exceptions: `PipelineCancelledException`, `ChunkTokenLimitExceededError`
- Extensive logging with context
- Fallback behaviors on non-critical errors
- `create_prefixed_exception()` for error context

### EdgeQuake Error Handling

```rust
// Result-based with ? operator
pub fn process(&self, content: &str) -> Result<ProcessingResult> {
    let chunks = self.chunker.chunk(content)?;
    let extractions = self.extract(chunks).await?;
    Ok(ProcessingResult { ... })
}

// Custom error types
#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("Extraction error: {0}")]
    ExtractionError(String),
    #[error("Embedding error: {0}")]
    EmbeddingError(String),
}
```

**Patterns:**

- `Result<T, E>` for all fallible operations
- `thiserror` derive macros
- `?` operator for propagation
- Type-safe error handling

## 7. Concurrency Model Comparison

### LightRAG Concurrency

```python
# Semaphore-controlled async
semaphore = asyncio.Semaphore(max_async)

async def _process_with_semaphore(chunk):
    async with semaphore:
        return await process_chunk(chunk)

tasks = [asyncio.create_task(_process_with_semaphore(c)) for c in chunks]
done, pending = await asyncio.wait(tasks, return_when=asyncio.FIRST_EXCEPTION)
```

**Characteristics:**

- Python GIL limits CPU parallelism
- Effective for I/O-bound operations
- asyncio event loop
- Explicit semaphore management

### EdgeQuake Concurrency

```rust
// Tokio semaphore with true parallelism
let semaphore = Arc::new(tokio::sync::Semaphore::new(max_concurrent));

let futures: Vec<_> = chunks.iter().map(|chunk| {
    let semaphore = semaphore.clone();
    async move {
        let _permit = semaphore.acquire().await?;
        extractor.extract(chunk).await
    }
}).collect();

let results: Vec<_> = stream::iter(futures)
    .buffer_unordered(max_concurrent)
    .collect()
    .await;
```

**Characteristics:**

- True multi-threading with tokio
- No GIL limitation
- Work-stealing scheduler
- Zero-cost abstractions

## 8. Summary

| Aspect                    | LightRAG                | EdgeQuake                 |
| ------------------------- | ----------------------- | ------------------------- |
| **Language**              | Python 3.x              | Rust 2021                 |
| **Async Runtime**         | asyncio                 | tokio                     |
| **Type System**           | Dynamic                 | Static                    |
| **Organization**          | Monolithic              | Modular crates            |
| **Error Handling**        | Exceptions              | Result<T,E>               |
| **Concurrency**           | Semaphore (GIL limited) | Semaphore (true parallel) |
| **Storage**               | Multiple backends       | PostgreSQL focus          |
| **Feature Maturity**      | High                    | Medium                    |
| **Performance Potential** | Good                    | Excellent                 |

---

_Document Version: 1.0_
_Last Updated: 2025-12-31_
