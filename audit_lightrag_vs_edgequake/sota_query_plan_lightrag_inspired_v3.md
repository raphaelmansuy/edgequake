# SOTA Query Implementation Plan v3 - LightRAG Parity

> **Date:** 2025-12-31  
> **Status:** Active Implementation  
> **Goal:** Achieve LightRAG feature parity with PostgreSQL+AGE+pgvector backend

---

## Current State Assessment

### ✅ Already Implemented
1. **LLM Keyword Extraction** - `edgequake-query/src/keywords/`
   - LLMKeywordExtractor with high/low level keywords
   - InMemoryKeywordCache with TTL
   - QueryIntent classification (Factual, Relational, Exploratory, Comparative)
   - MockKeywordExtractor for testing

2. **SOTA Query Engine** - `edgequake-query/src/sota_engine.rs`
   - 5 query modes (Local, Global, Hybrid, Mix, Naive)
   - Mode-specific vector filtering (Entity/Relationship/Chunk)
   - Adaptive mode selection based on query intent
   - Streaming support via `query_stream()`
   - Batch graph operations

3. **API Integration** - `edgequake-api/src/handlers/`
   - Query and chat handlers use SOTAQueryEngine
   - Streaming chat completions working

### ❌ Missing for LightRAG Parity

| Feature | LightRAG | EdgeQuake | Gap |
|---------|----------|-----------|-----|
| Source ID Tracking | ✅ Entities/Rels store source_id | ❌ Only source_spans (text) | **CRITICAL** |
| Document Path | ✅ file_path in metadata | ❌ Not stored | **CRITICAL** |
| Citation Links | ✅ [1], [2] markers | ❌ Not implemented | HIGH |
| Token Budgeting | ✅ Dynamic allocation | Partial (truncation) | MEDIUM |
| Reranking | ✅ Optional | ✅ JinaReranker exists | LOW (wire up) |
| Query Caching | ✅ Result cache | ❌ Only keyword cache | MEDIUM |

---

## Phase 1: Source ID Tracking (Critical Path)

### 1.1 Schema Changes

**Goal:** Store chunk_id and document_id in entity/relationship properties

#### 1.1.1 Modify ExtractedEntity and ExtractedRelationship

**File:** `edgequake-pipeline/src/extractor.rs`

```rust
// Add to ExtractedEntity:
pub source_chunk_ids: Vec<String>,    // Chunk IDs that mention this entity
pub source_document_id: Option<String>, // Document this entity came from
pub source_file_path: Option<String>,   // Original file path

// Add to ExtractedRelationship:
pub source_chunk_id: Option<String>,    // Chunk where relationship was extracted
pub source_document_id: Option<String>, // Document this relationship came from
pub source_file_path: Option<String>,   // Original file path
```

#### 1.1.2 Propagate Source Info During Extraction

**File:** `edgequake-pipeline/src/extractor.rs`

In `EntityExtractor::extract()`:
- Pass chunk_id to extracted entities
- Pass document_id and file_path to entities/relationships

#### 1.1.3 Store Source Info in Graph Nodes

**File:** `edgequake-pipeline/src/merger.rs`

In `create_entity_node()`:
```rust
properties.insert("source_chunk_ids", serde_json::json!(entity.source_chunk_ids));
properties.insert("source_document_id", serde_json::json!(entity.source_document_id));
properties.insert("source_file_path", serde_json::json!(entity.source_file_path));
```

#### 1.1.4 Store Source Info in Vector Metadata

**File:** `edgequake-pipeline/src/merger.rs`

In entity/relationship vector upsert:
```rust
metadata["source_chunk_ids"] = serde_json::json!(entity.source_chunk_ids);
metadata["source_document_id"] = serde_json::json!(entity.source_document_id);
metadata["source_file_path"] = serde_json::json!(entity.source_file_path);
```

### 1.2 Query Response Enhancement

**Goal:** Include source info in query responses

#### 1.2.1 Enhance RetrievedEntity/Relationship

**File:** `edgequake-query/src/context.rs`

```rust
// Add to RetrievedEntity:
pub source_chunk_ids: Vec<String>,
pub source_document_id: Option<String>,
pub source_file_path: Option<String>,

// Add to RetrievedRelationship:
pub source_chunk_id: Option<String>,
pub source_document_id: Option<String>,
pub source_file_path: Option<String>,
```

#### 1.2.2 Populate Source Info in SOTA Engine

**File:** `edgequake-query/src/sota_engine.rs`

In `query_local()`, `query_global()`, etc.:
- Extract source info from graph node properties
- Populate in RetrievedEntity/Relationship

#### 1.2.3 Expose in API Response

**File:** `edgequake-api/src/handlers/query.rs`

Already has `SourceReference` with:
- `document_id: Option<String>`
- `file_path: Option<String>`

Just need to populate from RetrievedEntity/Relationship.

### 1.3 Tests

#### Unit Tests
- [ ] `extractor.rs`: Entity extraction with source_chunk_id
- [ ] `merger.rs`: Graph node contains source info
- [ ] `sota_engine.rs`: Retrieved entities have source info

#### Integration Tests (PostgreSQL)
- [ ] End-to-end: Ingest → Query → Verify source_id in response
- [ ] Multi-document: Sources correctly track to documents

---

## Phase 2: Citation Links

### 2.1 LLM Prompt Enhancement

**Goal:** Get LLM to include [1], [2] style citations

**File:** `edgequake-query/src/prompts.rs` (or inline in sota_engine.rs)

Add to generation prompt:
```
When answering, cite sources using [1], [2], etc. Each number corresponds to the sources provided below.
```

### 2.2 Response Post-Processing

**File:** `edgequake-query/src/citations.rs` (new file)

```rust
pub struct AnnotatedAnswer {
    pub text: String,
    pub citations: Vec<Citation>,
}

pub struct Citation {
    pub marker: usize,       // [1], [2], etc.
    pub source_type: String, // entity, relationship, chunk
    pub source_id: String,
    pub document_id: Option<String>,
    pub file_path: Option<String>,
}

pub fn extract_citations(answer: &str, sources: &[SourceReference]) -> AnnotatedAnswer {
    // Parse [1], [2] markers from answer
    // Map to source references
}
```

### 2.3 Web UI Enhancement

**File:** `edgequake_webui/src/components/query/chat-message.tsx`

- Render [1], [2] as clickable links
- Show source panel when clicked
- Display file path and chunk content

---

## Phase 3: Token Budgeting

### 3.1 Budget Allocation by Mode

**File:** `edgequake-query/src/sota_engine.rs`

```rust
pub struct TokenBudget {
    pub total: usize,
    pub entities_ratio: f32,
    pub relationships_ratio: f32,
    pub chunks_ratio: f32,
}

impl TokenBudget {
    pub fn for_mode(mode: QueryMode, total: usize) -> Self {
        match mode {
            QueryMode::Local => Self {
                total,
                entities_ratio: 0.5,
                relationships_ratio: 0.3,
                chunks_ratio: 0.2,
            },
            QueryMode::Global => Self {
                total,
                entities_ratio: 0.3,
                relationships_ratio: 0.5,
                chunks_ratio: 0.2,
            },
            QueryMode::Hybrid => Self {
                total,
                entities_ratio: 0.35,
                relationships_ratio: 0.35,
                chunks_ratio: 0.3,
            },
            // ...
        }
    }
}
```

### 3.2 Accurate Token Counting

**File:** `edgequake-query/src/tokenizer.rs`

- Use tiktoken-rs for accurate GPT token counting
- Fall back to simple word count if unavailable

### 3.3 Priority-Based Truncation

**File:** `edgequake-query/src/truncation.rs`

- Sort by relevance score
- Truncate low-relevance items first
- Ensure each category stays within budget

---

## Phase 4: Query Caching

### 4.1 Cache Key Design

**File:** `edgequake-query/src/cache.rs` (new file)

```rust
#[derive(Hash, Eq, PartialEq)]
pub struct QueryCacheKey {
    query_hash: u64,
    mode: QueryMode,
    tenant_id: Option<String>,
    workspace_id: Option<String>,
    graph_version: u64,  // Invalidate on document changes
}
```

### 4.2 Cache Storage

- In-memory LRU cache with TTL (default: 1 hour)
- Optional Redis for distributed deployment

### 4.3 Cache Invalidation

- On document ingestion: increment graph_version for tenant/workspace
- On document deletion: same
- Manual invalidation via API

---

## Phase 5: Reranking Integration

### 5.1 Wire Existing JinaReranker

**File:** `edgequake-query/src/sota_engine.rs`

```rust
// After vector retrieval, before context building:
if self.config.enable_reranking {
    let reranked = self.reranker.rerank(&query, results, top_n).await?;
    // Use reranked results
}
```

### 5.2 Configuration

**File:** `edgequake-query/src/sota_engine.rs`

```rust
pub struct SOTAQueryConfig {
    // ...existing fields...
    pub enable_reranking: bool,
    pub reranker_top_k: usize,  // Retrieve 50, rerank to top 10
}
```

---

## Test Strategy

### Unit Tests (Per Feature)

| Test File | Tests |
|-----------|-------|
| `tests/unit_source_tracking.rs` | Entity/rel have source_chunk_ids |
| `tests/unit_citations.rs` | Citation extraction from answer |
| `tests/unit_token_budget.rs` | Budget allocation by mode |
| `tests/unit_cache.rs` | Cache hit/miss/invalidation |

### Integration Tests (PostgreSQL/AGE)

| Test File | Tests |
|-----------|-------|
| `tests/integration_source_tracking.rs` | Full ingestion → query → source in response |
| `tests/integration_citations.rs` | LLM generates citations correctly |
| `tests/integration_caching.rs` | Cache works across requests |

### E2E Tests (API Layer)

| Test File | Tests |
|-----------|-------|
| `tests/e2e_query_sources.rs` | POST /query returns sources with document_id |
| `tests/e2e_chat_citations.rs` | Chat includes [1], [2] citations |
| `tests/e2e_streaming_sources.rs` | Streaming includes sources event |

---

## Implementation Order

```
Week 1 (Current):
├── Phase 1.1: Schema changes (ExtractedEntity, Merger)
├── Phase 1.2: Query response enhancement
├── Phase 1.3: Unit tests
└── Phase 1.3: Integration tests

Week 2:
├── Phase 2.1: Citation prompt
├── Phase 2.2: Response post-processing
├── Phase 2.3: Web UI citation links
├── Phase 3: Token budgeting
└── Phase 5: Reranking wiring

Week 3:
├── Phase 4: Query caching
├── E2E tests
└── Performance benchmarking
```

---

## Success Criteria

### LightRAG Parity Checklist

- [ ] Entities store source_chunk_ids
- [ ] Relationships store source_chunk_id
- [ ] Query response includes document_id and file_path
- [ ] Web UI shows source links
- [ ] Citations appear in answers as [1], [2]
- [ ] Token budget prevents overflow
- [ ] Reranking improves relevance
- [ ] Query caching reduces latency

### Performance Targets

| Metric | Target |
|--------|--------|
| Query latency (p50) | < 500ms |
| Query latency (p99) | < 1.5s |
| Source tracking overhead | < 10% |
| Cache hit rate | > 20% |

---

## Files to Modify

### Backend (Rust)

| File | Changes |
|------|---------|
| `edgequake-pipeline/src/extractor.rs` | Add source_chunk_ids fields |
| `edgequake-pipeline/src/merger.rs` | Store source info in graph/vector |
| `edgequake-query/src/context.rs` | Add source fields to Retrieved* |
| `edgequake-query/src/sota_engine.rs` | Populate source info, citations |
| `edgequake-query/src/citations.rs` | New - citation extraction |
| `edgequake-query/src/cache.rs` | New - query result caching |
| `edgequake-api/src/handlers/query.rs` | Expose sources in response |
| `edgequake-api/src/handlers/chat.rs` | Expose sources in streaming |

### Frontend (TypeScript)

| File | Changes |
|------|---------|
| `edgequake_webui/src/types/index.ts` | Add source fields to types |
| `edgequake_webui/src/components/query/chat-message.tsx` | Render citations |
| `edgequake_webui/src/components/query/source-panel.tsx` | New - source details |

---

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Schema migration breaks existing data | Graceful fallback for missing fields |
| Citation parsing errors | Robust regex, fallback to no citations |
| Token counting inaccuracy | Use tiktoken-rs, validate against OpenAI |
| Cache invalidation complexity | Start with simple TTL, add versioning later |
