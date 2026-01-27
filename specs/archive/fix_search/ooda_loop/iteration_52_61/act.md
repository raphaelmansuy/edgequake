# OODA Loop 52-61: ACT Phase

> **Date:** 2026-01-06  
> **Status:** Implementation Complete - Testing Required

---

## Changes Implemented

### OODA 52: Keyword Extraction in Local Mode ✅

**File:** `edgequake/crates/edgequake-core/src/query.rs`

**Change Summary:**

- Added `KeywordExtractor::new()` call at start of `query_local()`
- Implemented multi-vector search using query + low-level keywords
- Added deduplication logic with `seen_ids` HashSet

**Code Diff (Conceptual):**

```rust
// BEFORE: Single embedding search
let query_embeddings = self.embedding.embed(&[query]).await?;
let entity_results = self.vector_storage.query(query_embedding, params.top_k, None).await?;

// AFTER: Multi-vector search with keywords
let keyword_extractor = KeywordExtractor::new(Arc::clone(&self.llm));
let keywords = keyword_extractor.extract(query).await.unwrap_or_default();

let mut search_texts = vec![query.to_string()];
search_texts.extend(keywords.low_level.iter().take(5).cloned());

let all_embeddings = self.embedding.embed(&search_texts).await?;

// Multi-vector search: search with each embedding
for embedding in &all_embeddings {
    let results = self.vector_storage.query(embedding, per_vector_k, None).await?;
    for result in results {
        if seen_ids.insert(result.id.clone()) {
            all_entity_results.push(result);
        }
    }
}
```

**Why This Works:**

- Low-level keywords like "BYD Seal U", "STLA Medium" get individual embeddings
- Each keyword finds entities that match that specific term
- Deduplication ensures we don't count entities twice
- Top-k sorting preserves highest-scoring entities

---

### OODA 53: Round-Robin Context Merging ✅

**File:** `edgequake/crates/edgequake-core/src/query.rs`

**Change Summary:**

- Added `round_robin_merge_entities()` helper function
- Added `round_robin_merge_relationships()` helper function
- Updated `query_hybrid()` to use round-robin instead of concatenation

**Code Diff (Conceptual):**

```rust
// BEFORE: Simple concatenation (local-first bias)
let mut merged_entities = local_result.context.entities;
for entity in global_result.context.entities {
    if !seen.contains(&entity.name) { merged_entities.push(entity); }
}

// AFTER: Round-robin interleaving
fn round_robin_merge_entities(local: &[Entity], global: &[Entity]) -> Vec<Entity> {
    let mut merged = Vec::new();
    for i in 0..max(local.len(), global.len()) {
        if let Some(e) = local.get(i) { merged.push(e); }  // L1, L2, L3...
        if let Some(e) = global.get(i) { merged.push(e); } // G1, G2, G3...
    }
    merged // Result: [L1, G1, L2, G2, L3, G3, ...]
}

let merged_entities = Self::round_robin_merge_entities(
    &local_result.context.entities,
    &global_result.context.entities,
);
```

**Why This Works:**

- Interleaved ordering ensures global results aren't cut off by top-k limits
- Deduplication by name/key prevents duplicates
- Both local (entity-centric) and global (relationship-centric) results contribute

---

## Build & Test Results

```
✅ cargo check --package edgequake-core: SUCCESS
✅ cargo test --package edgequake-core --lib: 102 passed, 0 failed
```

---

## Next Steps

1. **OODA 54-55:** Run French query test to measure improvement
2. **OODA 56:** Add multi-language keyword normalization
3. **OODA 57:** Implement token-aware truncation
4. **OODA 58-59:** Full system integration test
5. **OODA 60-61:** Measure metrics and document results

---

## Expected Impact

| Metric               | Before    | Expected After | Reason                           |
| -------------------- | --------- | -------------- | -------------------------------- |
| Entity recall        | ~60%      | ~85%           | Multi-vector keyword search      |
| Context diversity    | 0% global | 40% global     | Round-robin merge                |
| Hybrid answer length | 628 chars | >1500 chars    | Better context retrieval         |
| Query latency        | ~200ms    | ~400ms         | Additional LLM call for keywords |

---

## Rollback Plan

If issues occur, revert these changes:

1. Remove keyword extraction from `query_local()` (lines 225-280)
2. Remove round-robin functions (lines 1000-1050)
3. Restore simple concatenation in `query_hybrid()` (lines 817-870)
