# OODA Loop 3 - ACT: Precision Improvement via Reranker

## Action Taken

### 1. Added MockReranker to SOTAQueryEngine

**File Modified**: `edgequake/crates/edgequake-api/src/state.rs`

**Change**: Added `MockReranker` to both memory and PostgreSQL constructors:

```rust
// Create keyword-based reranker for improved precision
// WHY: MockReranker uses term overlap to boost chunks containing exact query terms,
// preventing model-name confusion (e.g., "2008" vs "3008")
let reranker = Arc::new(edgequake_llm::reranker::MockReranker::new());

// Create SOTA query engine with LightRAG-style enhancements
let sota_engine = Arc::new(SOTAQueryEngine::new(
    SOTAQueryConfig::default(),
    // ... providers ...
).with_reranker(reranker));
```

### 2. Created Test Data

Created 4 Peugeot model test documents in `specs/fix_search/test_data/`:
- `peugeot-2008-envy.md`
- `peugeot-208.md`
- `peugeot-3008.md`
- `peugeot-5008.md`

### 3. Created Precision Test

Created `specs/fix_search/test_precision.py` to validate model discrimination.

## Verification Results

### Precision Test Results

| Query | Expected | First Result | Score | Rerank Score | Status |
|-------|----------|--------------|-------|--------------|--------|
| Prix du Peugeot 2008 ENVY | 2008 | 2008 | 0.800 | 0.810 | ✅ PASS |
| Dimensions de la Peugeot 208 | 208 | 208 | 1.000 | 1.000 | ✅ PASS |
| Équipements du Peugeot 3008 GT | 3008 | 3008 | 0.800 | 0.810 | ✅ PASS |
| Peugeot 5008 7 places | 5008 | 5008 | 1.000 | 1.000 | ✅ PASS |

**Summary**: 4/4 tests passed (100% first-result precision)

### How MockReranker Works

The MockReranker computes term overlap between query and chunk:

```rust
fn score_by_keyword_overlap(&self, query: &str, text: &str) -> f32 {
    let query_terms: HashSet<_> = query.to_lowercase().split_whitespace().collect();
    let text_terms: HashSet<_> = text.to_lowercase().split_whitespace().collect();
    let overlap = query_terms.intersection(&text_terms).count();
    let max_terms = query_terms.len().max(1);
    overlap as f32 / max_terms as f32
}
```

This boosts chunks that contain exact query terms like "2008", preventing the semantic similarity from confusing "2008" with "208".

## Commit

```
git commit: e94dd7c
Message: fix(search): Add MockReranker for precision improvement
```

## Next Steps (OODA Loop 4)

1. Test recall with more complex queries
2. Investigate entity retrieval quality
3. Test hybrid mode performance
4. Consider tuning reranker weights
