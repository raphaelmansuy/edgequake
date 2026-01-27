# Iteration 08: ORIENT - Cleanup Helper Function Needed

## Date

2025-01-28

## Gap Analysis

### GAP-08: Reprocess Endpoints Skip Cleanup

**Problem**: When a document fails processing and is reprocessed, the partial entities/edges from the failed attempt remain. The reprocessed document creates NEW entities, leading to:

1. Duplicate entities with same name (if upsert merges) → inflated source_ids
2. Storage bloat from orphaned partial data
3. Incorrect reference counting

**Evidence**:

- `reprocess_failed` (documents.rs:2891-3032) - No cleanup call
- `recover_stuck` (documents.rs:3034-3200) - No cleanup call

### Root Cause

The cleanup logic is tightly coupled to `delete_document`. It's not extracted into a reusable function that can be called from reprocess endpoints.

## Solution Architecture

### Extract Reusable Cleanup Function

```rust
/// Clean up graph data for a document without deleting KV entries.
///
/// This function removes the document from entity/edge source_ids and
/// deletes entities/edges that have no remaining sources.
///
/// WHY: Called before reprocessing to prevent duplicate data accumulation.
///
/// # Parameters
/// - `document_id`: The document ID to clean up
/// - `graph_storage`: Graph storage adapter
/// - `vector_storage`: Vector storage adapter (for entity embeddings)
///
/// # Returns
/// Cleanup statistics (entities_removed, entities_updated, etc.)
async fn cleanup_document_graph_data(
    document_id: &str,
    graph_storage: &Arc<dyn GraphStorage>,
    vector_storage: &Arc<dyn VectorStorage>,
) -> Result<CleanupStats, ApiError> {
    // 1. Delete chunk embeddings from vector storage
    // 2. Process graph nodes - remove document from source_ids
    // 3. Process graph edges - remove document from source_ids
    // 4. Return cleanup stats
}
```

### Integration Points

1. **reprocess_failed** - Call `cleanup_document_graph_data` before requeueing
2. **recover_stuck** - Call `cleanup_document_graph_data` before requeueing
3. **delete_document** - Extract common logic to helper

## Implementation Strategy

### Option A: Full Refactor (Chosen)

1. Create `cleanup_document_graph_data` helper function
2. Refactor `delete_document` to use helper
3. Add cleanup call to `reprocess_failed`
4. Add cleanup call to `recover_stuck`
5. Add tests

**Pros**: DRY, single source of truth
**Cons**: More changes, risk of regression

### Option B: Minimal Inline

1. Copy cleanup logic inline to reprocess endpoints
2. Don't modify delete_document

**Pros**: Less risk
**Cons**: Code duplication, maintenance burden

## Decision

**Use Option A** - Full refactor with helper function.

Reason: The mission explicitly states "Ensure deleting a failed document cleans up all partial data." This requires a robust, testable, and reusable cleanup mechanism.

## Risk Mitigation

1. Run all existing deletion tests after refactor
2. Add specific test for reprocess cleanup
3. Verify cleanup stats are logged
