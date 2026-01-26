# Iteration 07: ORIENT - Async Path source_ids Merge Gap

## Date

2025-01-28

## Gap Identification

**GAP-07 (Async Variant)**: `processor.rs` creates entities/edges with `source_ids = [current_doc]` without merging with existing source_ids.

## Root Cause Analysis

### Why This Wasn't Caught in OODA-06

1. OODA-06 focused on `documents.rs` which handles sync upload
2. The async path in `processor.rs` was not examined
3. Tests use sync path by default (no task queue in memory tests)

### Why Async Path is Different

| Aspect         | Sync Path (documents.rs)                      | Async Path (processor.rs)                |
| -------------- | --------------------------------------------- | ---------------------------------------- |
| Entity Storage | Individual `upsert_node` with pre-fetch merge | Batch `upsert_nodes_batch` without merge |
| Edge Storage   | Individual `upsert_edge` with pre-fetch merge | Batch `upsert_edges_batch` without merge |
| Merge Logic    | ✅ Implemented in OODA-06                     | ❌ Missing                               |

### Code Structure Challenge

The async path uses batch operations for performance:

```rust
// Build batch
for entity in &extraction.entities {
    nodes_batch.push((entity.name.clone(), properties));
}

// Single batch upsert
self.graph_storage.upsert_nodes_batch(&nodes_batch).await
```

This is efficient BUT requires merge logic before batch construction.

## Fix Strategy Options

### Option A: Pre-fetch Merge Before Batch (Chosen)

- Fetch existing nodes/edges before building batch
- Merge source_ids in property construction
- Call batch upsert with merged properties
- **Pros**: Minimal changes, matches sync path pattern
- **Cons**: Additional queries before batch

### Option B: Modify GraphStorage Trait

- Add `upsert_nodes_batch_with_merge` method
- Move merge logic into storage layer
- **Pros**: Cleaner API, storage-level optimization possible
- **Cons**: More invasive, trait changes affect all implementations

### Option C: Post-Process Merge

- Call batch upsert first
- Then fetch and re-merge affected nodes
- **Pros**: Batch remains fast
- **Cons**: Race conditions, double writes

## Decision

**Use Option A** - Pre-fetch merge before batch construction.

This matches the pattern used in OODA-06 for the sync path and minimizes code changes.

## Implementation Notes

1. Before building `nodes_batch`, fetch existing nodes using `get_nodes_by_ids`
2. Create a HashMap of existing source_ids per entity
3. When building properties, merge with existing source_ids
4. Same pattern for edges

## Risk Mitigation

- The additional queries (get_nodes_by_ids, get_edges batch) add latency
- For first document (no existing entities), this is essentially a no-op query
- For subsequent documents with shared entities, the merge is essential for correctness
- Trade-off: Correctness over marginal performance
