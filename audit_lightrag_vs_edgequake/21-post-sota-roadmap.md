# Post-SOTA Roadmap: EdgeQuake Query Engine Enhancement

> **Date:** 2025-01-01  
> **Status:** SOTA Integration Complete - Planning Next Phase  
> **Current State:** LightRAG-style query pipeline active with LLM keyword extraction

---

## Executive Summary

The SOTA query engine is now fully integrated and serving production traffic. All query paths (direct query, chat completions, streaming) use the `SOTAQueryEngine` with:

- ✅ LLM keyword extraction (high/low level keywords)
- ✅ Query intent classification (Factual, Relational, Exploratory, Comparative)
- ✅ Adaptive mode selection (auto-selects local/global/hybrid)
- ✅ VectorType filtering (Entity/Relationship/Chunk)
- ✅ Batch graph operations (efficient traversal)
- ✅ Keyword caching (LRU cache)

This roadmap outlines the next phases of enhancement to achieve true SOTA quality.

---

## Phase 1: Immediate Priorities (P0) - Week 1-2

### 1.1 Source ID Tracking

**What:** Link every retrieved entity/relationship back to its source document chunks.

**Why:** Users need to verify answers and trace claims back to original sources. This is critical for trust and explainability.

**Implementation:**

```rust
// In GraphNode struct, add:
pub source_chunks: Vec<String>,  // Chunk IDs that mentioned this entity

// During ingestion (entity extraction):
// Store the chunk_id alongside each entity mention

// During query:
// When retrieving entities, also fetch their source_chunks
// Include in SourceReference response
```

**Success Criteria:**

- [ ] Every entity in the graph has `source_chunks` field
- [ ] Query response includes `document_id` and `file_path` in sources
- [ ] Web UI can show "View source" link for each citation

**Effort:** 3-5 days

---

### 1.2 Token Budgeting

**What:** Dynamically allocate token budget based on query complexity and context size.

**Why:** Prevents context overflow errors, optimizes cost, ensures consistent response quality.

**Implementation:**

```rust
// In SOTAQueryConfig, add:
pub max_context_tokens: usize,  // Total budget (e.g., 8000)
pub entity_budget_ratio: f32,   // 0.3 = 30% for entities
pub relationship_budget_ratio: f32,  // 0.3 for relationships
pub chunk_budget_ratio: f32,    // 0.4 for chunks

// During context assembly:
fn allocate_budget(&self, query_intent: &QueryIntent) -> BudgetAllocation {
    // Adjust ratios based on intent
    // Factual: more chunks, fewer relationships
    // Relational: more relationships, fewer chunks
    // Exploratory: balanced
}

fn truncate_to_budget(&self, context: Context, budget: usize) -> Context {
    // Use tiktoken-rs for accurate token counting
    // Prioritize by relevance score within each category
}
```

**Success Criteria:**

- [ ] No context overflow errors in production
- [ ] Consistent response quality regardless of graph size
- [ ] Cost stays within budget (trackable)

**Effort:** 2-3 days

---

## Phase 2: Short-Term Goals (P1) - Week 3-4

### 2.1 Reranking Integration

**What:** Use cross-encoder reranking (Jina, Cohere) to improve retrieval quality.

**Why:** Vector similarity is imprecise. Reranking significantly improves precision@k.

**Implementation:**

- Already have `JinaReranker` in codebase (verify it's used)
- Add reranking step after initial retrieval, before context assembly
- Configure in `SOTAQueryConfig`:
  ```rust
  pub enable_reranking: bool,
  pub reranker_top_k: usize,  // Rerank top 50, return top 10
  ```

**Success Criteria:**

- [ ] Reranking reduces irrelevant context by 30%+
- [ ] No significant latency increase (< 200ms)
- [ ] Configurable per-query or globally

**Effort:** 2 days (if already implemented, just wire it up)

---

### 2.2 Query Result Caching

**What:** Cache query results for repeated queries with TTL.

**Why:** Many users ask similar questions. Caching reduces latency and cost.

**Implementation:**

```rust
// Use existing Redis or in-memory cache
struct QueryCache {
    cache: Arc<Mutex<LruCache<QueryCacheKey, CachedResult>>>,
    ttl: Duration,
}

#[derive(Hash, Eq, PartialEq)]
struct QueryCacheKey {
    query_hash: u64,  // Hash of normalized query + mode + tenant
    graph_version: u64,  // Invalidate on graph update
}
```

**Success Criteria:**

- [ ] Cache hit rate > 20% for common queries
- [ ] Cache invalidated on document ingestion
- [ ] Configurable TTL (default: 1 hour)

**Effort:** 2 days

---

## Phase 3: Medium-Term Goals (P2) - Month 2

### 3.1 Multi-Hop Reasoning

**What:** Follow graph paths for complex queries that require connecting multiple entities.

**Why:** Some queries require "A relates to B, B relates to C, therefore A relates to C" reasoning.

**Implementation:**

```rust
// In query execution:
fn multi_hop_traverse(&self, seed_entities: Vec<EntityId>, max_hops: usize) -> Vec<Path> {
    // BFS/DFS from seed entities
    // Collect paths up to max_hops
    // Score paths by relevance to query
}

// Add to query prompt:
// "Based on the following paths through the knowledge graph..."
```

**Success Criteria:**

- [ ] Handles "How is X connected to Y?" queries correctly
- [ ] Path explanations included in answer
- [ ] Max 2-3 hops to prevent explosion

**Effort:** 1 week

---

### 3.2 Citation Links

**What:** Link specific claims in the answer to specific sources.

**Why:** Users need to verify individual claims, not just the overall answer.

**Implementation:**

```rust
// In LLM prompt:
// "When citing information, use [1], [2], etc. to reference sources."

// Post-process answer:
fn add_citation_links(answer: &str, sources: &[SourceReference]) -> AnnotatedAnswer {
    // Parse [1], [2] markers
    // Map to source IDs
    // Return structured response
}
```

**Success Criteria:**

- [ ] Answer contains [1], [2] style citations
- [ ] Citations map to specific sources in response
- [ ] Web UI renders clickable citation links

**Effort:** 3 days

---

## Phase 4: Long-Term Goals (P3) - Month 3+

### 4.1 Hierarchical Community Detection

**What:** Implement multi-level graph clustering for better global mode.

**Why:** LightRAG uses Leiden algorithm for community detection. This improves global summaries.

**Implementation:**

- Implement Leiden or use existing library
- Compute communities at ingestion time
- Store community assignments in graph
- Query retrieves community summaries for global mode

**Effort:** 1-2 weeks

---

### 4.2 Context Compression

**What:** Compress context when approaching token limits.

**Why:** Allows more information in the same token budget.

**Implementation:**

- Summarize chunks before including in context
- Use smaller model for compression (faster/cheaper)
- Balance compression ratio vs information loss

**Effort:** 1 week

---

### 4.3 Streaming Metrics

**What:** Real-time metrics during streaming responses.

**Why:** Users want to see progress and costs.

**Implementation:**

- Emit metrics events during streaming
- Track: retrieval time, tokens generated, estimated cost
- Show in Web UI

**Effort:** 3 days

---

## Testing Strategy

### Unit Tests

- [ ] `sota_engine.rs`: Test each method with mock providers
- [ ] `keywords/`: Test extraction, caching, intent classification
- [ ] Token budgeting: Test allocation algorithms

### Integration Tests

- [ ] Full query pipeline with test graph data
- [ ] Verify sources are correctly linked
- [ ] Test reranking improves results

### E2E Tests

- [ ] API endpoints return expected structure
- [ ] Web UI displays citations correctly
- [ ] Streaming works with all new features

### Performance Benchmarks

- [ ] Latency before/after reranking
- [ ] Cache hit rates
- [ ] Token usage with budgeting

---

## Success Metrics

| Metric                | Current | Target | How to Measure    |
| --------------------- | ------- | ------ | ----------------- |
| Query Latency (p50)   | ~800ms  | <500ms | API metrics       |
| Query Latency (p99)   | ~2s     | <1.5s  | API metrics       |
| Relevance Score       | N/A     | >0.8   | Manual evaluation |
| Citation Accuracy     | N/A     | >90%   | Manual evaluation |
| Cache Hit Rate        | 0%      | >20%   | Cache metrics     |
| Context Overflow Rate | ~5%     | 0%     | Error logs        |

---

## Immediate Next Steps

1. **Today:** Start Phase 1.1 (Source ID Tracking)

   - Audit current entity storage schema
   - Plan schema migration for `source_chunks`
   - Implement during ingestion

2. **This Week:** Complete Phase 1 (Source Tracking + Token Budgeting)

3. **Next Week:** Phase 2 (Reranking + Caching)

---

## Appendix: File Locations

| Feature         | Files to Modify                                                               |
| --------------- | ----------------------------------------------------------------------------- |
| Source Tracking | `edgequake-storage/src/traits/graph.rs`, `edgequake-pipeline/src/extractors/` |
| Token Budgeting | `edgequake-query/src/sota_engine.rs`, `edgequake-query/src/context.rs`        |
| Reranking       | `edgequake-query/src/reranking/` (may exist), wire into `sota_engine.rs`      |
| Query Caching   | `edgequake-query/src/cache.rs` (new)                                          |
| Multi-Hop       | `edgequake-query/src/traversal.rs` (new)                                      |
| Citations       | `edgequake-query/src/sota_engine.rs`, `edgequake-api/src/handlers/`           |
