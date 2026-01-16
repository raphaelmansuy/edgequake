# OODA-230 Iteration 224: Act

**Date**: January 16, 2026
**Status**: ✅ COMPLETE

## Summary

Fixed critical bug where Hybrid/Local/Global query modes returned 0 document chunks
when using workspace-specific vector storage. The root cause was missing
`source_chunk_ids` logic in the `_with_vector_storage` method variants.

## Changes Made

### 1. Fixed `query_local_with_vector_storage` (sota_engine.rs)

**Before** (buggy):

```rust
// Step 7: Get source chunks using workspace vector storage
let chunk_results = vector_storage
    .query(&embeddings.query, self.config.max_chunks * 2, None)
    .await?;
let chunks = filter_by_type(chunk_results, VectorType::Chunk);
```

**After** (fixed):

```rust
// Step 7: Collect source_chunk_ids from entities and relationships
let mut chunk_ids = std::collections::HashSet::new();
for entity in &context.entities {
    for chunk_id in &entity.source_chunk_ids {
        chunk_ids.insert(chunk_id.clone());
    }
}
for rel in &context.relationships {
    if let Some(chunk_id) = &rel.source_chunk_id {
        chunk_ids.insert(chunk_id.clone());
    }
}

// Retrieve chunks by ID filter
if !chunk_ids.is_empty() {
    let chunk_ids_vec: Vec<String> = chunk_ids.into_iter().collect();
    let results = vector_storage
        .query(&embeddings.low_level, chunk_ids_vec.len(), Some(&chunk_ids_vec))
        .await?;
    for result in results {
        context.add_chunk(build_chunk_from_result(&result));
    }
}
```

### 2. Fixed `query_global_with_vector_storage` (sota_engine.rs)

Same pattern applied to global mode.

### 3. Added E2E Test (e2e_chunk_retrieval.rs)

New test file with 3 tests:

- `test_local_mode_retrieves_chunks_via_source_chunk_ids`
- `test_chunk_retrieval_by_id_filter`
- `test_entity_has_source_chunk_ids`

## Test Results

```
running 3 tests
test test_entity_has_source_chunk_ids ... ok
test test_chunk_retrieval_by_id_filter ... ok
test test_local_mode_retrieves_chunks_via_source_chunk_ids ... ok

test result: ok. 3 passed; 0 failed; 0 ignored
```

## Files Modified

| File                                                              | Changes                                                                                           |
| ----------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| `edgequake/crates/edgequake-query/src/sota_engine.rs`             | Fixed chunk retrieval in `query_local_with_vector_storage` and `query_global_with_vector_storage` |
| `edgequake/crates/edgequake-query/tests/e2e_chunk_retrieval.rs`   | NEW - E2E tests for chunk retrieval                                                               |
| `specs/032-ollama-lmstudio-provider/ooda_loop/iteration_224/*.md` | OODA loop documentation                                                                           |

## Verification Checklist

- [x] `query_local_with_vector_storage` collects source_chunk_ids
- [x] `query_global_with_vector_storage` collects source_chunk_ids
- [x] New E2E test covers chunk retrieval
- [x] All existing tests pass (`cargo test --package edgequake-query`)
- [x] Build succeeds (`cargo build --package edgequake-query`)
- [x] No new clippy warnings introduced

## Next Steps

1. **Manual verification**: Test with real OpenAI workspace to confirm user scenario works
2. **Consider Naive mode**: May need similar fix if it also shows 0 chunks
3. **Performance optimization**: Consider batching chunk ID queries

## Lessons Learned

When creating method variants (e.g., `_with_vector_storage`), ensure all critical
logic is copied, especially:

- Collection of linked IDs (source_chunk_ids, source_document_ids)
- Filtering and scoring logic
- Tenant/workspace isolation checks

## WHY-OODA230 Documentation

Added `WHY-OODA230` comments to the fixed code explaining:

- Why we must use source_chunk_ids instead of semantic search
- Why the old approach returned 0 chunks (entities/relationships score higher)
