# 003 — Code Is Law: Current Query Pipeline

**Spec:** 028-edgequake-query-service  
**Date:** 2026-06-28  
**Cross-ref:** [024-003](../024-egdequake-audit/003-query-retrieval-audit.md) | [021-02-query-pipeline](../021-storage-study/03-pipelines/02-query-pipeline.md)  
**Authority note:** This lens documents **current** code. Target state in [012-code-is-law-verdict.md](./012-code-is-law-verdict.md).

---

## Pipeline SSOT

**File:** `edgequake/crates/edgequake-query/src/engine_impl/query_entry/query_pipeline.rs`

Single implementation: **prepare → retrieve → finalize**. Used by all non-stream entry points (SPEC-017 P1-01).

```text
  run_query_pipeline(request, providers)
       │
       ├── [Bypass?] ──yes──► pipeline_finalize (empty context, direct LLM)
       │
       ├── pipeline_prepare
       │     ├── conversation-augmented query text
       │     ├── parallel: keyword LLM extract + embed_one
       │     ├── validate_keywords (graph label check)
       │     ├── QueryEmbeddings { query, high_level, low_level }
       │     └── mode: explicit | adaptive (QueryIntent) | config default
       │
       ├── pipeline_retrieve (by QueryMode)
       │     ├── Naive  → chunk ANN (+ BM25/FTS fusion)
       │     ├── Local  → entity ANN → graph batch → provenance chunks
       │     ├── Global → relationship ANN → graph expand → provenance chunks
       │     ├── Hybrid → 3-way parallel → round-robin interleave
       │     └── Mix    → 3-way parallel → weighted score or RRF
       │
       └── pipeline_finalize
             ├── filter by allowed_document_ids
             ├── optional rerank (Reranker trait)
             ├── sort entities by degree
             ├── balance_context (30k token budget default)
             ├── context_only → answer = ""
             ├── prompt_only  → answer = build_prompt(...)
             └── default      → generate_answer_with_provider
```

---

## System Context Diagram

```
+--------------------------- EdgeQuake Query Path ---------------------------+
|                                                                            |
|  +-------------+     +------------------+     +-------------------------+  |
|  | HTTP Client |---->| edgequake-api    |---->| query_execution.rs      |  |
|  | Agent / UI  |     | handlers/query/* |     | execute_sota_query*     |  |
|  +-------------+     +------------------+     +------------+------------+  |
|                                                            |               |
|                                                            v               |
|  +-------------------------------------------------- QueryEngine --------+  |
|  |  engine_impl/query_entry/query_pipeline.rs                           |  |
|  |       prepare │ retrieve │ finalize                                  |  |
|  +-------+--------------+--------------+--------------------------------+  |
|          |              |              |                                   |
|          v              v              v                                   |
|  +-----------+  +---------------+  +-----------+                          |
|  | Embedding |  | VectorStorage |  | GraphStorage │ (via vector/graph)  |
|  | Provider  |  | (ANN + BM25)  |  | (AGE/pg)     │                      |
|  +-----------+  +---------------+  +-----------+                          |
|          |              |              |                                   |
|          v              v              v                                   |
|  +-----------+  +---------------+  +-----------+                          |
|  | LLM (kw)  |  | KVStorage     |  | Reranker  │                          |
|  | optional  |  | chunk hydrate |  | optional  │                          |
|  +-----------+  +---------------+  +-----------+                          |
|                                                                            |
+----------------------------------------------------------------------------+
```

---

## Query Modes (Code Truth)

**File:** `edgequake/crates/edgequake-query/src/modes.rs`

| Mode | Default? | Vector | Graph | Fusion |
|------|----------|--------|-------|--------|
| Mix | **Enum default** | 3-way parallel | via Local+Global | weighted / RRF |
| Hybrid | Chat default | 3-way parallel | via Local+Global | round-robin |
| Local | — | entity (low) | nodes + N-hop | provenance chunks |
| Global | — | relationship (high) | batch + community | provenance chunks |
| Naive | — | chunk ANN | none | BM25/FTS optional |
| Bypass | — | skip | skip | direct LLM |

**Known inconsistency (QRY-001):** `/query` defaults to **Mix**; `/chat/completions` defaults to **Hybrid** when mode omitted.

---

## Internal Context Types (Engine Rich Truth)

**File:** `edgequake/crates/edgequake-query/src/context.rs`

```rust
pub struct QueryContext {
    pub chunks: Vec<RetrievedChunk>,
    pub entities: Vec<RetrievedEntity>,
    pub relationships: Vec<RetrievedRelationship>,
    pub token_count: usize,
    pub is_truncated: bool,
    pub metadata: HashMap<String, serde_json::Value>,
}
```

**RetrievedChunk:** `id`, `content` (full), `score`, `document_id`, `token_count`, `start_line`, `end_line`, `chunk_index`

**RetrievedEntity:** `name`, `entity_type`, `description`, `score`, `degree`, `source_chunk_ids`, `source_document_id`, `source_file_path`

**RetrievedRelationship:** `source`, `target`, `relation_type`, `description`, `score`, `source_chunk_id`, `source_document_id`, `source_file_path`

**LLM formatting:** `QueryContext::to_context_string()` — markdown sections for prompt injection.

---

## HTTP Layer (Current)

### Routes

**File:** `edgequake/crates/edgequake-api/src/routes.rs` (~390–397)

| Method | Path | Handler |
|--------|------|---------|
| POST | `/api/v1/query` | `query_execute::execute_query` |
| POST | `/api/v1/query/stream` | `query_stream::stream_query` |
| POST | `/api/v1/chat/completions` | `chat/completion::chat_completion` |
| POST | `/api/v1/chat/completions/stream` | `chat/streaming` |

### Service routing

**File:** `edgequake/crates/edgequake-api/src/services/query_execution.rs`

All paths → `QueryEngine::query_with_full_config` via workspace embedding/vector matrix + optional LLM override.

### API DTOs

**File:** `edgequake/crates/edgequake-api/src/handlers/query_types.rs`

**Request highlights:**
- `context_only: bool` — skips LLM generation
- `prompt_only: bool` — returns formatted prompt
- `include_references: bool` — **DEAD FIELD** (never read)
- `document_filter: DocumentFilter` — SPEC-005

**Response:** `QueryResponse { answer, mode, sources: SourceReference[], stats, conversation_id?, reranked }`

**SourceReference:** flat union type via `source_type` discriminator (chunk|entity|relationship); `snippet` not full content.

---

## Context-Only Path Today

```
  POST /api/v1/query
  { "query": "...", "context_only": true, "mode": "mix" }
       │
       ▼
  run_query_pipeline
       │
       ├── context_only → may hit result_cache (query_result_cache.rs)
       ├── pipeline_finalize → answer = ""
       └── returns QueryResponse with sources[] mapped from QueryContext
```

**Cache:** LRU+TTL for `context_only` only (`edgequake-query/src/cache/query_result_cache.rs`).

**Gap:** Engine returns full `QueryContext` in `QueryResponse.context` internally; HTTP handler **discards** it and maps to flat `sources[]`.

---

## Streaming Context Emission

**File:** `edgequake/crates/edgequake-api/src/handlers/query/query_stream.rs`

Stream v2 emits `QueryStreamEvent::Context { sources, stats }` **before** tokens — but still flat `SourceReference[]`.

Engine path: `run_context_pipeline` → `enrich_retrieved_context` → `stream_answer_from_context`.

---

## Document Scoping SSOT

```
  document_filter (API)
       │
       ▼
  document_filter_resolver.rs
       │  scoped metadata scan (SPEC-027 phase 18)
       ▼
  allowed_document_ids: Option<HashSet<String>>
       │
       ▼
  engine QueryRequest.params["allowed_document_ids"]
       │
       ▼
  pipeline_finalize → context_filter
```

Context service **must** reuse this exact path (FP-028-05).

---

## Duplication Map (DRY Violations — QRY-002)

| Logic | Location A | Location B |
|-------|------------|------------|
| QueryContext → sources | `query_execute.rs` | `chat/mod.rs::build_sources` |
| Document title resolve | `handlers/query/mod.rs` | duplicated in chat |
| Workspace resolve | each handler | should be service-only |
| Engine request build | query_execute | chat/completion |

See [008-dry-refactor-generation-lens.md](./008-dry-refactor-generation-lens.md).

---

## What Exists vs What SPEC-028 Adds

| Capability | Exists | Location | SPEC-028 action |
|------------|--------|----------|-----------------|
| Full retrieval pipeline | ✅ | `query_pipeline.rs` | Wrap, don't rewrite |
| Rich QueryContext | ✅ | `context.rs` | Expose via DTO |
| context_only flag | ✅ | `/query` | Deprecate → `/query/context` |
| Structured subgraph API | ❌ | — | **Add** |
| Full chunk in API | ❌ | snippet only | **Add** (`agent` tier) |
| Agent quality signals | ❌ | — | **Add** |
| MCP tools | ❌ | — | **Add** (phase 5) |
| QueryContextService | ❌ | — | **Add** |

---

## Key File Index

| Component | Path |
|-----------|------|
| Pipeline SSOT | `edgequake-query/src/engine_impl/query_entry/query_pipeline.rs` |
| Context types | `edgequake-query/src/context.rs` |
| Protocol types | `edgequake-query/src/types.rs` |
| Modes | `edgequake-query/src/modes.rs` |
| Prompt/LLM | `edgequake-query/src/engine_impl/prompt.rs` |
| API execute | `edgequake-api/src/handlers/query/query_execute.rs` |
| API stream | `edgequake-api/src/handlers/query/query_stream.rs` |
| API DTOs | `edgequake-api/src/handlers/query_types.rs` |
| Query service | `edgequake-api/src/services/query_execution.rs` |
| Result cache | `edgequake-query/src/cache/query_result_cache.rs` |
| Document filter | `edgequake-api/src/handlers/query/document_filter_resolver.rs` |
