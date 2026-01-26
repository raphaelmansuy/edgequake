# Iteration 08: ACT - Implemented Cleanup Helper for Reprocess Endpoints

## Date

2025-01-28

## Summary

Implemented GAP-08 fix: Added `cleanup_document_graph_data` helper function and integrated it into `reprocess_failed` and `recover_stuck` endpoints.

## Changes Made

### 1. Added CleanupStats Struct (documents.rs:254-267)

```rust
#[derive(Debug, Default, Clone)]
pub struct CleanupStats {
    pub entities_removed: usize,
    pub entities_updated: usize,
    pub relationships_removed: usize,
    pub relationships_updated: usize,
    pub embeddings_deleted: usize,
}
```

### 2. Added cleanup_document_graph_data Function (documents.rs:269-396)

Extracted reusable cleanup logic from `delete_document`:

- Process all nodes - remove document_id from source_ids
- Delete nodes with empty source_ids
- Process all edges - remove document_id from source_ids
- Delete edges with empty source_ids OR orphaned (connects to deleted node)
- Delete entity embeddings for removed entities
- Returns CleanupStats for logging

### 3. Modified reprocess_failed (documents.rs:3142-3177)

Added cleanup call before requeueing:

```rust
// OODA-08: Clean up partial graph data from failed attempt BEFORE requeueing
match cleanup_document_graph_data(doc_id, &state.graph_storage, None).await {
    Ok(stats) => {
        tracing::info!(
            document_id = %doc_id,
            entities_removed = stats.entities_removed,
            "Cleaned up partial data before reprocessing"
        );
    }
    Err(e) => {
        tracing::warn!(
            document_id = %doc_id,
            error = %e,
            "Failed to cleanup partial data, continuing anyway"
        );
    }
}
```

### 4. Modified recover_stuck (documents.rs:3343-3368)

Same cleanup pattern applied.

### 5. Added Tests (e2e_document_deletion.rs:1377-1645)

| Test                                           | Description                                                         | Status  |
| ---------------------------------------------- | ------------------------------------------------------------------- | ------- |
| `test_reprocess_cleans_partial_graph_data`     | Verifies partial entities are cleaned before requeueing failed docs | ✅ PASS |
| `test_recover_stuck_cleans_partial_graph_data` | Verifies partial entities are cleaned before recovering stuck docs  | ✅ PASS |
| `test_reprocess_preserves_shared_entities`     | Verifies shared entities (with other completed docs) are preserved  | ✅ PASS |

## Testing Results

| Test Suite                                                        | Result                        |
| ----------------------------------------------------------------- | ----------------------------- |
| `cargo build --package edgequake-api`                             | ✅ Success                    |
| `cargo test --package edgequake-api --lib`                        | ✅ 421 passed                 |
| `cargo test --package edgequake-api --test e2e_document_deletion` | ✅ 19 passed (was 16, +3 new) |

## WHY Comments Added

```rust
// OODA-08: Clean up partial graph data from failed attempt BEFORE requeueing
// WHY: Without cleanup, reprocessing creates duplicate entities and corrupts source_ids
//
// Scenario without cleanup:
//   T1: Document processed 60% → entities A, B created with source_ids = [doc]
//   T2: Processing fails
//   T3: reprocess_failed called
//   T4: Document reprocessed → entities A, B upserted with source_ids = [doc]
//   T5: Now entities have inflated source_ids (double reference)
//   T6: Delete document → entities still exist (incorrect)
//
// With cleanup:
//   T1-T2: Same as above
//   T3: reprocess_failed cleans up A, B (deletes them since source_ids = [doc])
//   T4: Document reprocessed → entities A, B created fresh
//   T5: source_ids correctly = [doc]
//   T6: Delete document → entities properly deleted
```

## Impact Analysis

### Before (GAP-08)

```
Document fails → Partial entities remain
Reprocess → Duplicate entities created
source_ids = [doc, doc] (incorrect)
Delete document → Entities still exist
```

### After (Fixed)

```
Document fails → Partial entities remain
Reprocess → Cleanup runs first
Partial entities deleted OR source_ids updated
Reprocess creates fresh entities
source_ids = [doc] (correct)
Delete document → Entities properly deleted
```

## Next Steps

1. Continue OODA iterations for remaining areas:
   - Query process after deletion
   - PostgreSQL provider integration
   - Bulk deletion operations
