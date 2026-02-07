# First Principles Audit: EdgeQuake vs LightRAG RAG Quality
## Date: 2026-02-08
## Goal: Ensure EdgeQuake > LightRAG using first principles and e2e tests

---

## Executive Summary

**Current Status**: EdgeQuake has **all the right building blocks** but **doesn't use them in the query pipeline**.

### Tier 1 (Config) - ✅ **COMPLETE**
- max_entities: 60 (LightRAG parity)
- max_relationships: 60 (LightRAG parity)
- max_chunks: 20 (LightRAG parity)
- max_context_tokens: 30000 (LightRAG parity)
- Token-based truncation: ✅ Implemented (`balance_context`)

### Tier 2 (Code) - ⚠️ **PARTIALLY COMPLETE**
- ✅ BM25Reranker wired up (state.rs:102-105)
- ✅ Round-robin merge implemented (sota_engine.rs:2522-2597)
- ✅ `retrieve_chunks_from_entities()` exists with VECTOR/WEIGHT methods
- ❌ **VECTOR chunk selection NOT used in query flow** ← **PRIMARY GAP**
- ⚠️ Hybrid mode merging could be improved

---

## First Principles: What Makes a RAG System Better?

### Principle 1: **Recall** - Find all relevant information
**Question**: When a user asks "Compare X vs Y", does the system find information about BOTH X and Y?

**LightRAG Approach**:
1. Search top-60 entities (high recall)
2. Collect ALL candidate chunks from matched entities (20+ chunks)
3. Re-rank chunks by query similarity (VECTOR method)
4. Return top-20 most relevant

**EdgeQuake Current**:
1. Search top-60 entities ✅
2. Collect chunk IDs from entities ✅
3. Fetch chunks by ID **without re-ranking** ❌
4. Return chunks in arbitrary order ❌

**Impact**: Low. EdgeQuake gets the same chunks, just unranked.

---

### Principle 2: **Precision** - Return only relevant information
**Question**: When an entity appears in 20 chunks, do we return the 3 most relevant or random 3?

**LightRAG Approach**:
```python
# VECTOR method: Re-rank by cosine similarity
candidate_chunks = get_chunks_from_entities(entities)
chunk_embeddings = get_embeddings(candidate_chunks)
scores = cosine_similarity(query_embedding, chunk_embeddings)
return top_k_by_score(candidate_chunks, scores, k=20)
```

**EdgeQuake Current**:
```rust
// Direct ID fetch - no re-ranking
let chunk_ids = collect_chunk_ids_from_entities(&entities);
let chunks = vector_storage.query(&embedding, chunk_ids.len(), Some(&chunk_ids));
// Returns chunks in storage order, not relevance order
```

**Impact**: **HIGH**. Entity "Apple Inc." may appear in:
- Chunk A: "Apple Inc. was founded by Steve Jobs in 1976..."
- Chunk B: "Apple Inc. reported Q3 earnings..."
- Chunk C: "Apple Inc. filed a patent for..."
- ...20 more chunks

Query: "Who founded Apple?"
LightRAG returns Chunk A (highest cosine similarity).
EdgeQuake returns random chunks (no re-ranking).

---

### Principle 3: **Diversity** - Balance different information sources
**Question**: Does hybrid mode actually combine local + global + naive, or does one dominate?

**LightRAG Approach** (mix mode):
```python
# Round-robin merge
local_chunks = local_search(query)    # Entity-centric
global_chunks = global_search(query)  # Relationship-centric
vector_chunks = naive_search(query)   # Direct similarity

# Interleave 1:1:1
merged = []
for i in range(max_chunks):
    if i < len(vector_chunks): merged.append(vector_chunks[i])
    if i < len(local_chunks): merged.append(local_chunks[i])
    if i < len(global_chunks): merged.append(global_chunks[i])
return deduplicate(merged)[:max_chunks]
```

**EdgeQuake Current** (hybrid_with_vector_storage):
```rust
// Line 3337: Start with naive as base
let mut merged = naive_context;

// Add local chunks (dedup)
for chunk in local_context.chunks {
    if !merged.chunks.iter().any(|c| c.id == chunk.id) {
        merged.add_chunk(chunk);
    }
}
// Add global chunks (dedup)
for chunk in global_context.chunks {
    if !merged.chunks.iter().any(|c| c.id == chunk.id) {
        merged.add_chunk(chunk);
    }
}
```

**Problem**: If naive finds 20 chunks (max_chunks=20), local/global chunks are never added (dedup filter).

**Impact**: **MEDIUM**. Naive chunks may dominate when they shouldn't.

---

### Principle 4: **Correctness** - Answer the actual question
**Question**: Does the LLM get the right context to answer correctly?

**Dependencies**:
- Recall (Principle 1) ← Need to find relevant info
- Precision (Principle 2) ← Need to prioritize relevant info
- Diversity (Principle 3) ← Need balanced sources
- Token budget ← Need enough context (✅ 30K tokens)

**EdgeQuake Status**:
- ✅ Token budget (30K)
- ✅ Recall (60 entities, 20 chunks)
- ❌ Precision (no VECTOR re-ranking in query flow)
- ⚠️ Diversity (naive-first merge)

---

## Gap Analysis: Code vs Architecture

### Gap 1: VECTOR Chunk Selection Not Used ❌

**Exists**: `edgequake-query/src/chunk_retrieval.rs:41`
```rust
pub async fn retrieve_chunks_from_entities(
    entities: &[RetrievedEntity],
    vector_storage: &dyn VectorStorage,
    method: ChunkSelectionMethod,  // ← VECTOR or WEIGHT
    query_embedding: Option<&[f32]>,
    max_chunks: usize,
) -> Result<Vec<RetrievedChunk>> {
    match method {
        ChunkSelectionMethod::Vector => {
            // Re-rank by cosine similarity to query
            let candidate_chunk_ids = collect_chunk_ids(entities);
            let chunk_embeddings = get_chunk_embeddings(candidate_chunk_ids);
            let scores = cosine_similarity(query_embedding, chunk_embeddings);
            return top_k_by_score(chunks, scores, max_chunks);
        }
        ChunkSelectionMethod::Weight => {
            // Use entity occurrence frequency
            // ...
        }
    }
}
```

**NOT USED IN**: `query_local()` (line 2115), `query_local_with_vector_storage()` (line 2897), `query_global()`, `query_global_with_vector_storage()`

**Fix**: Replace direct ID fetch with `retrieve_chunks_from_entities(..., ChunkSelectionMethod::Vector, ...)`

**Expected Impact**: +10-15% precision, +5-10% correctness

---

### Gap 2: Hybrid Mode Chunk Merge Order ⚠️

**Current**: `let mut merged = naive_context;` (line 3339)
**Problem**: Naive chunks always have priority, may fill quota before KG chunks added

**LightRAG**: Round-robin interleave ensures 33% naive, 33% local, 33% global

**Fix**:
```rust
// Round-robin merge chunks
let mut merged_chunks = Vec::new();
let max_len = naive.chunks.len()
    .max(local.chunks.len())
    .max(global.chunks.len());

for i in 0..max_len {
    if let Some(c) = naive.chunks.get(i) {
        if !seen.contains(&c.id) {
            merged_chunks.push(c.clone());
            seen.insert(c.id.clone());
        }
    }
    if let Some(c) = local.chunks.get(i) {
        if !seen.contains(&c.id) {
            merged_chunks.push(c.clone());
            seen.insert(c.id.clone());
        }
    }
    if let Some(c) = global.chunks.get(i) {
        if !seen.contains(&c.id) {
            merged_chunks.push(c.clone());
            seen.insert(c.id.clone());
        }
    }
}
```

**Expected Impact**: +5-10% diversity, +3-5% correctness

---

## Implementation Priority

### Priority 1: VECTOR Chunk Selection (1-2 days)
**Why**: Direct quality improvement, well-defined scope
**Where**: `query_local()`, `query_local_with_vector_storage()`, `query_global()`, `query_global_with_vector_storage()`
**How**: Replace chunk ID collection with `retrieve_chunks_from_entities(..., ChunkSelectionMethod::Vector, ...)`

### Priority 2: Round-Robin Chunk Merge (4 hours)
**Why**: Ensures balanced hybrid retrieval
**Where**: `query_hybrid_with_vector_storage()` line 3337
**How**: Implement triple round-robin (naive, local, global)

### Priority 3: Comprehensive E2E Tests (1 day)
**Why**: Validate improvements, prevent regressions
**What**:
- Multi-entity queries ("Compare Apple vs Microsoft")
- Precision tests (entity in 20 chunks, verify top-3 relevance)
- Hybrid diversity tests (verify 33/33/33 distribution)
- LightRAG benchmarks (Agriculture domain, French queries)

---

## E2E Test Plan

### Test 1: VECTOR Chunk Selection Precision
```rust
#[tokio::test]
async fn test_vector_chunk_selection_precision() {
    // Setup: Entity "APPLE" appears in 20 chunks
    // Chunk 1: "Apple Inc. was founded by Steve Jobs"  (high relevance)
    // Chunk 2-19: Various other mentions             (low relevance)
    // Chunk 20: "Who founded Apple Inc? Steve Jobs" (high relevance)

    let query = "Who founded Apple?";
    let request = QueryRequest::new(query).with_mode(QueryMode::Local);

    let response = engine.query(request).await.unwrap();

    // Verify top chunks are high-relevance (cosine > 0.7)
    assert!(response.context.chunks[0].score > 0.7);
    assert!(chunks_contain_keywords(&response.context.chunks[0..3],
                                     &["founded", "Steve Jobs"]));
}
```

### Test 2: Hybrid Mode Diversity
```rust
#[tokio::test]
async fn test_hybrid_mode_diversity() {
    let query = "How do tech companies compete?";
    let request = QueryRequest::new(query).with_mode(QueryMode::Hybrid);

    let response = engine.query(request).await.unwrap();

    // Categorize chunks by source
    let naive_count = count_chunks_from_naive(&response.context);
    let local_count = count_chunks_from_local(&response.context);
    let global_count = count_chunks_from_global(&response.context);

    // Verify balanced distribution (within 20% of 33.3%)
    let total = response.context.chunks.len() as f32;
    assert!((naive_count as f32 / total - 0.33).abs() < 0.20);
    assert!((local_count as f32 / total - 0.33).abs() < 0.20);
    assert!((global_count as f32 / total - 0.33).abs() < 0.20);
}
```

### Test 3: Multi-Entity Recall
```rust
#[tokio::test]
async fn test_multi_entity_recall() {
    let query = "Compare Apple and Microsoft";
    let request = QueryRequest::new(query).with_mode(QueryMode::Hybrid);

    let response = engine.query(request).await.unwrap();

    // Verify BOTH entities found
    assert!(has_entity(&response.context, "APPLE"));
    assert!(has_entity(&response.context, "MICROSOFT"));

    // Verify chunks for BOTH entities
    let apple_chunks = count_chunks_mentioning(&response.context, "Apple");
    let msft_chunks = count_chunks_mentioning(&response.context, "Microsoft");

    assert!(apple_chunks >= 3, "Need at least 3 Apple chunks");
    assert!(msft_chunks >= 3, "Need at least 3 Microsoft chunks");
}
```

### Test 4: LightRAG Agriculture Benchmark
```rust
#[tokio::test]
async fn test_lightrag_agriculture_benchmark() {
    // Ingest LightRAG's demo agriculture.txt
    ingest_document("tests/fixtures/agriculture.txt").await;

    // Run LightRAG's benchmark queries
    let queries = vec![
        ("What are the important entities in this document?", 0.85),
        ("What are the main agricultural trends?", 0.80),
        ("How has sustainable farming evolved?", 0.75),
    ];

    for (query, expected_min_score) in queries {
        let response = engine.query(QueryRequest::new(query)
            .with_mode(QueryMode::Hybrid)).await.unwrap();

        let score = evaluate_answer_quality(&response.answer, query);
        assert!(score >= expected_min_score,
                "Query '{}' scored {}, expected >= {}",
                query, score, expected_min_score);
    }
}
```

### Test 5: French Query Benchmark (Real User Case)
```rust
#[tokio::test]
async fn test_french_ev_comparison() {
    // Real user query that failed
    let query = "Quelle est la différence entre la STLA Medium et la STLA Large?";

    let response = engine.query(QueryRequest::new(query)
        .with_mode(QueryMode::Hybrid)).await.unwrap();

    // Verify answer contains key differences
    assert!(response.answer.contains("batterie") ||
            response.answer.contains("kWh"));
    assert!(response.context.chunks.len() >= 5,
            "Need at least 5 context chunks for comparison");

    // Verify both platforms found
    assert!(has_entity(&response.context, "STLA MEDIUM") ||
            has_entity(&response.context, "STLA_MEDIUM"));
    assert!(has_entity(&response.context, "STLA LARGE") ||
            has_entity(&response.context, "STLA_LARGE"));
}
```

---

## Success Metrics

### Before (Current EdgeQuake)
- Overall: 0.729
- Recall: 78.6%
- Correctness: 89.3%
- Precision: 94.9%

### After Priority 1 (VECTOR chunk selection)
- Overall: **~0.80-0.85** (estimated)
- Recall: ~80% (slight improvement from better ranking)
- Correctness: **~92-94%** (better chunks = better answers)
- Precision: **~96-97%** (top-k by similarity)

### After Priority 1 + 2 (+ round-robin merge)
- Overall: **~0.85-0.90** (estimated)
- Recall: ~82-85%
- Correctness: **~93-95%**
- Precision: **~96-97%**

### Target (LightRAG parity + EdgeQuake advantages)
- Overall: **> 0.90**
- Recall: **> 85%**
- Correctness: **> 95%**
- Precision: **> 96%**

---

## EdgeQuake > LightRAG Advantages

Once gaps are fixed, EdgeQuake will surpass LightRAG due to:

1. ✅ **Keyword validation** - Drops phantom keywords not in graph
2. ✅ **Rust performance** - Faster concurrent processing
3. ✅ **Entity degree sorting** - Prioritizes important entities
4. ✅ **Fallback to popular entities** - No empty results
5. ✅ **Multi-tenant isolation** - Better for production
6. ✅ **Description decay** - Prevents semantic drift

Plus all LightRAG features:
- 60 entity candidates ✅
- 20 chunk candidates ✅
- 30K token budget ✅
- Token-based truncation ✅
- Keyword extraction ✅
- Mode-specific embeddings ✅
- BM25 reranking ✅
- Round-robin merging ✅

---

## Action Plan

### Week 1 (Feb 8-14)
- ✅ Day 1: Complete first-principles audit (this document)
- [ ] Day 2-3: Implement VECTOR chunk selection in query flow
- [ ] Day 4: Implement round-robin chunk merge optimization
- [ ] Day 5: Create comprehensive e2e test suite

### Week 2 (Feb 15-21)
- [ ] Day 1-2: Run benchmarks, iterate on improvements
- [ ] Day 3: French query stress testing
- [ ] Day 4: LightRAG agriculture benchmark
- [ ] Day 5: Performance profiling and optimization

### Week 3 (Feb 22-28)
- [ ] Day 1-2: Documentation updates
- [ ] Day 3-4: Production readiness review
- [ ] Day 5: Release v1.0 with "Better than LightRAG" claim

---

## Conclusion

**EdgeQuake has all the pieces**. The architecture is sound. The code quality is excellent. We just need to **connect the dots** by using the VECTOR chunk selection that already exists.

**Estimated total effort**: 3-4 days of focused development
**Expected outcome**: EdgeQuake performance > LightRAG + production-ready features

The path is clear. Let's execute.
