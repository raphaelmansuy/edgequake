# Iteration 08: DECIDE - Implement Cleanup Helper Function

## Date

2025-01-28

## Decision

Implement `cleanup_document_graph_data` helper function and integrate it into reprocess endpoints.

## Implementation Plan

### Step 1: Create CleanupStats Struct

```rust
/// Statistics from document graph cleanup.
#[derive(Debug, Default)]
pub struct CleanupStats {
    pub entities_removed: usize,
    pub entities_updated: usize,
    pub relationships_removed: usize,
    pub relationships_updated: usize,
    pub embeddings_deleted: usize,
}
```

### Step 2: Extract cleanup_document_graph_data Function

Location: `documents.rs` near the top of the file (after imports, before handlers)

The function will:

1. Accept document_id, graph_storage, and optional vector_storage
2. Process all nodes - remove document from source_ids
3. Delete nodes with empty source_ids
4. Process all edges - remove document from source_ids
5. Delete edges with empty source_ids OR orphaned (connects to deleted node)
6. Delete entity embeddings for removed entities
7. Return CleanupStats

### Step 3: Modify reprocess_failed

Before requeueing, call:

```rust
let cleanup_stats = cleanup_document_graph_data(
    &doc_id,
    &state.graph_storage,
    Some(&workspace_vector_storage),
).await?;
info!("Cleaned up partial data before reprocess: {:?}", cleanup_stats);
```

### Step 4: Modify recover_stuck

Same pattern as reprocess_failed.

### Step 5: Add Test

New test: `test_reprocess_cleans_partial_data`

1. Upload document, let it partially process
2. Simulate failure (manually set status to "failed")
3. Add some test entities with document's source_id
4. Call reprocess endpoint
5. Verify entities were cleaned up
6. Verify document is requeued

## Files to Modify

| File                       | Changes                                                    |
| -------------------------- | ---------------------------------------------------------- |
| `documents.rs`             | Add `CleanupStats`, `cleanup_document_graph_data` function |
| `documents.rs`             | Modify `reprocess_failed` to call cleanup                  |
| `documents.rs`             | Modify `recover_stuck` to call cleanup                     |
| `e2e_document_deletion.rs` | Add test for reprocess cleanup                             |

## Success Criteria

1. ✅ `cargo build --package edgequake-api` succeeds
2. ✅ All existing 30 tests pass
3. ✅ New test `test_reprocess_cleans_partial_data` passes
4. ✅ Cleanup is logged with stats

## Estimated Effort

- CleanupStats + helper function: 30 min
- Modify reprocess endpoints: 15 min
- Add test: 20 min
- Total: ~1 hour
