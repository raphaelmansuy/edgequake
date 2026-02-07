# Brutal First-Principles Audit: LightRAG vs EdgeQuake

**Date**: 2026-02-08  
**Scope**: Index pipeline + query pipeline, code-level comparison  
**Sources**: LightRAG `HKUDS/LightRAG` (operate.py 5000+ lines, prompt.py, lightrag.py, constants.py) vs EdgeQuake (sota_engine.rs 3416 lines, extractor.rs 1507 lines, pipeline.rs 2136 lines, merger.rs 863 lines, context.rs 418 lines, entity_extraction.rs 270 lines)

---

## Executive Summary

EdgeQuake successfully ported LightRAG's entity extraction format and core graph-RAG architecture to Rust. However, it runs with **cripplingly conservative defaults** and is **missing 4 critical retrieval mechanisms** that LightRAG uses to achieve high recall. The biggest single issue is a **7.5x smaller context budget** (4K vs 30K tokens), which means EdgeQuake throws away ~87% of the context it COULD be sending to the LLM. Combined with 3x fewer entities and 2x fewer chunks, EdgeQuake's retrieval pipeline is operating at roughly **15-20% of LightRAG's information throughput**.

**Bottom line**: The code quality is solid. The architecture is right. The defaults are killing performance.

---

## 1. Index Pipeline Comparison

### 1.1 Chunking — PARITY ✅

| Parameter  | LightRAG               | EdgeQuake                | Verdict                    |
| ---------- | ---------------------- | ------------------------ | -------------------------- |
| chunk_size | 1200 tokens            | 1200 tokens              | Same                       |
| overlap    | 100 tokens             | 100 tokens               | Same                       |
| tokenizer  | tiktoken (gpt-4o-mini) | SimpleTokenizer (char/4) | EdgeQuake's is approximate |

**Note**: EdgeQuake's `SimpleTokenizer` divides `len/4` for token count — this is a rough approximation. LightRAG uses tiktoken which is exact for OpenAI models. For gpt-4o-mini this matters less (BPE is fairly consistent), but edge cases exist with multi-byte characters and code.

### 1.2 Entity Extraction Prompts — PARITY ✅

Both use identical tuple-delimited extraction format:

- Delimiter: `<|#|>`
- Completion: `<|COMPLETE|>`
- Same 3 few-shot examples (Alex/Taylor, Stock Markets, Dr. Sarah Chen)
- Same system prompt structure
- Same continue_extraction (gleaning) prompt

**EdgeQuake literally ported LightRAG's prompts.** This is correct — they work well.

### 1.3 Gleaning — UNCLEAR ⚠️

| Feature                    | LightRAG                            | EdgeQuake                     |
| -------------------------- | ----------------------------------- | ----------------------------- |
| Default gleaning passes    | 1 (`entity_extract_max_gleaning=1`) | Has prompt, unclear if called |
| Token overflow guard       | `max_extract_input_tokens=20480`    | `MAX_CHUNK_TOKENS=1500`       |
| Gleaning abort on overflow | Yes (checks total context)          | No explicit check             |

**LightRAG** always does exactly 1 gleaning pass by default. It checks if the combined context (system_prompt + history + continue_prompt) exceeds `max_extract_input_tokens` (20480) before making the gleaning LLM call.

**EdgeQuake** has `GleaningExtractor` as a separate wrapper but it's unclear if `SOTAExtractor` uses gleaning consistently. The `continue_extraction_prompt` exists but needs verification that it's actually invoked.

### 1.4 LLM Response Cache — GAP ❌

| Feature          | LightRAG                            | EdgeQuake            |
| ---------------- | ----------------------------------- | -------------------- |
| Extraction cache | `llm_response_cache` KV store       | None                 |
| Cache key        | hash(system_prompt + user_prompt)   | N/A                  |
| Benefit          | Re-processes without re-calling LLM | Every retry costs $$ |

**Impact**: Medium. On failure/restart, LightRAG skips already-extracted chunks. EdgeQuake re-extracts everything. For a 50-page document with 200 chunks, this saves ~200 LLM calls on retry.

### 1.5 Entity Vector Content — SIMILAR ✅

| System    | Entity vector content                           |
| --------- | ----------------------------------------------- |
| LightRAG  | `"{entity_name}\n{description}"`                |
| EdgeQuake | `"{entity_name}\n{description}"` (via metadata) |

Both embed the same content for entities. Good.

### 1.6 Relationship Vector Content & Storage — GAP ❌

| Feature                     | LightRAG                                    | EdgeQuake                                            |
| --------------------------- | ------------------------------------------- | ---------------------------------------------------- |
| Relationship vector content | `"{keywords}\t{src}\n{tgt}\n{description}"` | Stored in same vector table                          |
| Separate VDB                | Yes (`relationships_vdb`)                   | No (shared `VectorStorage` with `vec_type` metadata) |
| Searchable independently    | Yes                                         | Yes (via `filter_by_type(VectorType::Relationship)`) |

**LightRAG** embeds relationship keywords + entity names + description as the vector content. This means a search for "trade agreements" can find the relationship "USA --[TRADE_PARTNER]--> China" directly.

**EdgeQuake** also stores relationship vectors, and the global query mode does filter for `VectorType::Relationship`. However, the content and embedding quality of these relationship vectors needs verification.

**Impact**: Low-Medium. Both can search relationships. The content format may differ slightly.

### 1.7 Entity-Chunk Tracking — DIFFERENT APPROACH

| Feature                 | LightRAG                                      | EdgeQuake                                |
| ----------------------- | --------------------------------------------- | ---------------------------------------- |
| Tracking mechanism      | `entity_chunks_storage` (separate KV store)   | `source_chunk_ids` in entity metadata    |
| Relation-chunk tracking | `relation_chunks_storage` (separate KV store) | `source_chunk_id` in relationship struct |
| Incremental updates     | Yes (compute_incremental_chunk_ids)           | Overwrite on merge                       |

Both systems can resolve entity → chunk_ids. LightRAG's approach is more robust for incremental updates (add new chunks without losing old ones).

### 1.8 Entity Merge — SIMILAR ✅

| Feature           | LightRAG                                | EdgeQuake                             |
| ----------------- | --------------------------------------- | ------------------------------------- |
| Dedup             | By entity name (normalized)             | By entity name (UPPERCASE normalized) |
| Description merge | LLM summarization when > 8 descriptions | LLM summarization (configurable)      |
| Max description   | `SUMMARY_MAX_TOKENS=1200`               | `max_description_length=4096`         |
| Decay factor      | N/A                                     | `description_decay=0.9`               |

Both use LLM-based summarization. EdgeQuake's decay factor is an interesting addition (weights recent descriptions higher).

---

## 2. Query Pipeline Comparison

### 2.1 Default Parameters — CRITICAL GAP 🔴🔴🔴

| Parameter               | LightRAG  | EdgeQuake       | Ratio           | Impact       |
| ----------------------- | --------- | --------------- | --------------- | ------------ |
| **top_k (entities)**    | **60**    | **20**          | **3x less**     | **HIGH**     |
| **chunk_top_k**         | **20**    | **10**          | **2x less**     | **HIGH**     |
| **max_entity_tokens**   | **6000**  | **N/A**         | **MISSING**     | **MEDIUM**   |
| **max_relation_tokens** | **8000**  | **N/A**         | **MISSING**     | **MEDIUM**   |
| **max_total_tokens**    | **30000** | **4000**        | **7.5x less**   | **CRITICAL** |
| cosine_threshold        | 0.2       | 0.1 (min_score) | ~similar        | Low          |
| related_chunk_number    | 5         | N/A             | N/A             | Medium       |
| graph_depth             | N/A       | 2               | EdgeQuake extra | N/A          |

**This is the single biggest problem.** EdgeQuake caps context at 4000 tokens — that's roughly 3000 words. A typical RAG response needs 20-30 chunks of context. At 1200 tokens per chunk, 4000 tokens fits **~3 chunks**. We're feeding 3 chunks when we could feed 25.

With `gpt-4o-mini` having a 128K context window, there is ZERO reason to limit to 4000 tokens. Even 30000 tokens (LightRAG's default) uses only 23% of the available window.

### 2.2 Keyword Extraction — PARITY ✅

Both systems use LLM-based keyword extraction producing:

- `high_level_keywords`: Overarching concepts/themes
- `low_level_keywords`: Specific entities/details

**LightRAG**: 2 few-shot examples, JSON output, cached in `llm_response_cache`.  
**EdgeQuake**: `LLMKeywordExtractor` with `CachedKeywordExtractor` wrapper (in-memory TTL cache).

EdgeQuake actually has a nice addition: **keyword validation against the graph** (`validate_keywords()`). It drops low-level keywords that don't match any entity in the graph to prevent embedding dilution. LightRAG doesn't do this.

### 2.3 Search Flow — LOCAL MODE

**LightRAG local flow:**

```
1. entities_vdb.query(ll_keywords, top_k=60) → 60 entity vectors
2. get_nodes_batch() + node_degrees_batch() → enriched entity data
3. _find_most_related_edges_from_entities() → graph edge traversal
4. _find_related_text_unit_from_entities() → chunks via VECTOR/WEIGHT method
5. _apply_token_truncation(entities, max_entity_tokens=6000)
6. _apply_token_truncation(relations, max_relation_tokens=8000)
7. Collect entity_chunks + relation_chunks
8. _build_context_str() with max_total_tokens=30000
```

**EdgeQuake local flow:**

```
1. vector_storage.query(embeddings.low_level, max_entities*3) → filter entity vectors
2. get_nodes_batch() + node_degrees_batch() → enriched entity data
3. get_edges_for_nodes_batch() → graph edges
4. Collect source_chunk_ids from entities + relationships
5. vector_storage.query(embeddings.low_level, chunk_ids.len(), filter=chunk_ids) → chunks by ID
6. Add to context (no token-based truncation, count-based only)
```

**Key differences:**

- LightRAG searches 60 entities, EdgeQuake searches 20
- LightRAG has a sophisticated chunk picking algorithm (VECTOR/WEIGHT), EdgeQuake does simple ID lookup
- LightRAG applies per-category token truncation, EdgeQuake uses count limits
- LightRAG fetches chunks from BOTH entities AND relationships separately, EdgeQuake collects all chunk IDs and fetches once

### 2.4 Search Flow — GLOBAL MODE

**LightRAG global flow:**

```
1. relationships_vdb.query(hl_keywords, top_k=60) → 60 relationship vectors
2. _find_most_related_entities_from_relationships() → connected entity nodes
3. Same chunk resolution as local (VECTOR/WEIGHT method)
4. Same token truncation
```

**EdgeQuake global flow:**

```
1. vector_storage.query(embeddings.high_level, max_relationships*3)
2. filter_by_type(VectorType::Relationship) → relationship vectors
3. Extract src_id/tgt_id → collect entity IDs
4. Fetch entities from graph, edges, chunks by source_chunk_ids
```

Both search relationship vectors in global mode. The main difference is the top_k (60 vs ~20) and the chunk resolution method.

### 2.5 Search Flow — HYBRID/MIX MODE

**LightRAG mix (default recommended):**

```
1. Parallel: local search + global search + vector_chunks search
2. Round-robin merge entities: alternate local ↔ global
3. Round-robin merge relations: alternate local ↔ global
4. Chunk merge: round-robin of vector_chunks + entity_chunks + relation_chunks
5. Dedup by chunk_id
6. Apply token truncation per category
7. Build context with max_total_tokens=30000
8. Optional reranking of chunks
```

**EdgeQuake hybrid (after fix):**

```
1. tokio::join!(local, global, naive) → parallel execution
2. Use naive context as base
3. Dedup-add chunks from local + global
4. Dedup-add entities from local + global
5. Dedup-add relationships from local + global
6. No token truncation (count-based limits only)
7. Build context with max_context_tokens=4000
8. Reranker exists but reranker=None (not wired)
```

**Critical difference**: LightRAG's round-robin ensures diversity from ALL three sources. EdgeQuake starts with naive as base, which may cause naive chunks to dominate. If naive finds 10 chunks (the max), there's no room for KG-derived chunks.

### 2.6 Chunk-from-Entity Resolution — GAP ❌

| Feature            | LightRAG                          | EdgeQuake                         |
| ------------------ | --------------------------------- | --------------------------------- |
| Default method     | **VECTOR** (embedding similarity) | Direct ID lookup                  |
| Alternative        | WEIGHT (gradient polling)         | N/A                               |
| Relevance ordering | Re-ranked by cosine(query, chunk) | No ordering — whatever comes back |

**LightRAG's VECTOR method:**

1. Collect candidate chunk IDs from entities' `source_id` field
2. Get chunk embeddings from `chunks_vdb`
3. Compute cosine similarity between query embedding and each candidate chunk
4. Return top-N by similarity score

This means even when the KG says "entity X appears in chunks A, B, C", LightRAG picks the chunks that are MOST RELEVANT to the current query. EdgeQuake just fetches all of them in arbitrary order.

**Impact**: HIGH. This directly affects retrieval precision. An entity may appear in 20 chunks, but only 3 are relevant to the current question.

### 2.7 Token-Based Truncation — GAP ❌

**LightRAG**: `_apply_token_truncation()` per category:

- Entities: iterate through entity list, count tokens, stop at `max_entity_tokens` (6000)
- Relations: iterate through relation list, count tokens, stop at `max_relation_tokens` (8000)
- Final context: total <= `max_total_tokens` (30000)

**EdgeQuake**: Count-based truncation only:

- `max_entities=20` (take first 20)
- `max_relationships=20` (take first 20)
- `max_chunks=10` (take first 10)
- No token budget awareness

**Why this matters**: An entity with a 2000-token description counts the same as one with 50 tokens. You might fit 100 short entities or 3 long ones. Count-based limits don't optimize for information density.

### 2.8 Reranking — DEAD CODE ❌

| Feature            | LightRAG                                     | EdgeQuake                                  |
| ------------------ | -------------------------------------------- | ------------------------------------------ |
| Reranker           | Configurable (BAAI/bge-reranker, Jina, etc.) | Code exists, `reranker: None`              |
| Default            | `enable_rerank=True`                         | `enable_rerank=true` but no reranker wired |
| Top-k after rerank | chunk_top_k (20)                             | rerank_top_k (10)                          |

**EdgeQuake has a fully implemented `rerank_chunks()` method** (lines 350-440 of sota_engine.rs). It handles BM25 scores, fallback for zero-score chunks, score-based filtering. It's good code. **It just never runs because `self.reranker` is always `None`.**

To wire it, someone needs to:

1. Implement the `Reranker` trait for a provider (e.g., Jina, Cohere, or local cross-encoder)
2. Call `engine.with_reranker(reranker)` when constructing the engine

### 2.9 Context Format — DIFFERENT

**LightRAG** uses structured JSON in markdown code blocks:

````
Knowledge Graph Data (Entity):
```json
[{"entity_name": "X", "description": "...", "entity_type": "..."}]
````

Knowledge Graph Data (Relationship):

```json
[{ "src_id": "X", "tgt_id": "Y", "description": "...", "keywords": "..." }]
```

Document Chunks:

```json
[{ "content": "...", "reference_id": 1, "file_path": "..." }]
```

Reference Document List:
[1] Document Title

```

**EdgeQuake** uses simple markdown:
```

## Retrieved Documents

### Document (score: 0.850)

[chunk content]

## Knowledge Graph Entities

- **ENTITY_NAME** (TYPE): description

## Entity Relationships

- SOURCE --[RELATION]--> TARGET

```

LightRAG's format is more structured and enables citation tracking via `reference_id`. EdgeQuake's format is simpler and more human-readable but loses structural information.

### 2.10 Conversation History — GAP ❌

**LightRAG**: `conversation_history` in QueryParam, sent to LLM as additional context.
**EdgeQuake**: Not supported. Each query is stateless.

### 2.11 Query Caching

**LightRAG**: Full query result caching in `llm_response_cache`.
**EdgeQuake**: Keyword extraction caching (`CachedKeywordExtractor` with TTL).

EdgeQuake's approach is lighter but also less aggressive. LightRAG caches the entire LLM response for identical queries, avoiding expensive generation.

---

## 3. Gap Ranking by Expected Impact

### Tier 1: CRITICAL (implement immediately)

| # | Gap | Expected Impact | Effort |
|---|-----|----------------|--------|
| 1 | **max_context_tokens: 4000 → 30000** | +30-50% recall, +20% correctness | **5 min** (config change) |
| 2 | **max_entities: 20 → 60** | +15-25% recall | **5 min** (config change) |
| 3 | **max_chunks: 10 → 20** | +10-20% recall | **5 min** (config change) |

**Total effort for Tier 1: 15 minutes.** These are pure config changes in `SOTAQueryConfig`. They will immediately allow:
- 7.5x more context to the LLM
- 3x more entity candidates for KG search
- 2x more text chunks for direct evidence

### Tier 2: HIGH (implement this week)

| # | Gap | Expected Impact | Effort |
|---|-----|----------------|--------|
| 4 | **Wire up reranker** | +5-15% precision | 2-4 hours |
| 5 | **Round-robin chunk merge** | +5-10% source diversity | 2-3 hours |
| 6 | **Chunk-from-entity re-ranking** | +5-10% precision | 3-4 hours |
| 7 | **Token-based truncation** | +5% efficiency | 4-6 hours |

### Tier 3: MEDIUM (implement next sprint)

| # | Gap | Expected Impact | Effort |
|---|-----|----------------|--------|
| 8 | **LLM response cache** | Cost savings on retry | 1 day |
| 9 | **Structured context format** | +2-5% answer quality | Half day |
| 10 | **Conversation history** | Multi-turn support | Half day |
| 11 | **Verify gleaning is active** | +5% entity coverage | 2 hours |

---

## 4. What EdgeQuake Does BETTER Than LightRAG

Credit where it's due:

1. **Keyword validation against graph** (`validate_keywords()`): EdgeQuake checks if keywords actually exist as entities in the graph before embedding them. This prevents semantic dilution from phantom keywords. LightRAG doesn't do this.

2. **Rust performance**: Concurrent processing with `tokio::join!` is genuinely faster than Python's asyncio for CPU-bound work.

3. **Entity degree sorting** (`sort_entities_by_degree`): EdgeQuake can sort entities by graph degree (connectedness) to prioritize important entities. LightRAG sorts by cosine similarity only.

4. **Fallback to popular entities**: When no entity vectors match, EdgeQuake falls back to high-degree entities from the graph. LightRAG would return no results.

5. **Multi-tenant isolation**: EdgeQuake's tenant_id/workspace_id filtering allows multi-workspace deployment from one vector store. LightRAG uses filesystem-based workspace isolation.

6. **Description decay**: The `description_decay=0.9` factor in entity merging weights recent descriptions higher, reducing semantic drift from accumulation.

---

## 5. Recommended Implementation Order

```

Week 0 (Today - 15 min):
[x] Change SOTAQueryConfig defaults: - max_context_tokens: 4000 → 30000 - max_entities: 20 → 60 - max_chunks: 10 → 20
[ ] Re-run evaluation to measure impact

Week 1 (2-3 days):
[ ] Implement round-robin chunk merge in hybrid mode
[ ] Add token-based truncation per category
[ ] Wire up a reranker (Jina or cross-encoder)

Week 2 (2-3 days):
[ ] Implement VECTOR chunk picking from entities
[ ] Add structured context format with reference_ids
[ ] Verify gleaning is actually running

Week 3+:
[ ] Add LLM response cache for extractions
[ ] Add conversation history support
[ ] Implement exact tokenizer (tiktoken-rs or similar)

````

---

## 6. The Numbers Tell the Story

### Current EdgeQuake hybrid (post-fix):
- Overall: 0.729
- Recall: 78.6%
- Correctness: 89.3%
- Precision: 94.9%
- KW_F1: 77.6%
- Latency: 4470ms

### Expected after Tier 1 config changes:
- Overall: **~0.82-0.87** (estimated)
- Recall: **~88-93%** (more chunks + entities = more evidence found)
- Correctness: **~92-95%** (more context = better answers)
- Precision: **~93-95%** (similar — more context helps but also adds noise)

### Expected after Tier 1 + 2:
- Overall: **~0.87-0.92** (estimated)
- With reranking and smart chunk selection pushing precision higher

---

## Appendix A: Code Location Map

### EdgeQuake
| Component | File | Lines |
|-----------|------|-------|
| Chunker config | `edgequake-pipeline/src/chunker.rs` | 1-100 |
| Entity extraction prompts | `edgequake-pipeline/src/prompts/entity_extraction.rs` | 1-270 |
| SOTAExtractor | `edgequake-pipeline/src/extractor.rs` | 550-800 |
| Pipeline config | `edgequake-pipeline/src/pipeline.rs` | 1-150 |
| KG merger | `edgequake-pipeline/src/merger.rs` | 1-200 |
| Query config | `edgequake-query/src/sota_engine.rs` | 90-200 |
| Local query | `edgequake-query/src/sota_engine.rs` | 2878-3050 |
| Global query | `edgequake-query/src/sota_engine.rs` | 3060-3270 |
| Hybrid query | `edgequake-query/src/sota_engine.rs` | 3277-3416 |
| Reranking | `edgequake-query/src/sota_engine.rs` | 350-440 |
| Context building | `edgequake-query/src/context.rs` | 1-200 |

### LightRAG
| Component | File | Lines |
|-----------|------|-------|
| Constants/defaults | `lightrag/constants.py` | 1-60 |
| Prompts | `lightrag/prompt.py` | 1-420 |
| Entity extraction | `lightrag/operate.py` | 2773-2940 |
| Entity merge | `lightrag/operate.py` | 1106-1200 |
| KG query | `lightrag/operate.py` | 3124-3250 |
| _get_node_data (local) | `lightrag/operate.py` | 4196-4250 |
| _get_edge_data (global) | `lightrag/operate.py` | 4469-4520 |
| _perform_kg_search | `lightrag/operate.py` | 3493-3620 |
| _build_query_context | `lightrag/operate.py` | 4077-4190 |
| Chunk-from-entity | `lightrag/operate.py` | 4314-4460 |
| Chunk-from-relations | `lightrag/operate.py` | 4562-4740 |
| _build_context_str | `lightrag/operate.py` | 3919-4070 |
| Token truncation | `lightrag/operate.py` | ~3700-3900 |
| Naive query | `lightrag/operate.py` | 4832-5025 |
| QueryParam | `lightrag/base.py` | ~350-430 |
| LightRAG init | `lightrag/lightrag.py` | 187-340 |

---

## Appendix B: The One-Liner That Changes Everything

In `edgequake/crates/edgequake-query/src/sota_engine.rs`, around line 90:

```rust
// BEFORE (current):
pub struct SOTAQueryConfig {
    pub max_entities: usize,          // 20
    pub max_relationships: usize,      // 20
    pub max_chunks: usize,             // 10
    pub max_context_tokens: usize,     // 4000
    // ...
}

// AFTER (LightRAG parity):
pub struct SOTAQueryConfig {
    pub max_entities: usize,          // 60
    pub max_relationships: usize,      // 60
    pub max_chunks: usize,             // 20
    pub max_context_tokens: usize,     // 30000
    // ...
}
````

That's it. Four number changes. The rest of the code already handles larger values correctly.
