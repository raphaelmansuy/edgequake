# Iteration 09: ORIENT - Query Safety Verified, Minor Improvement Identified

## Date

2025-01-28

## Analysis Summary

### No Critical Issues

The query process is SAFE after document deletion:

1. Vector storage query with filter gracefully handles missing chunks
2. Graph storage batch fetch returns only existing nodes
3. No exceptions or crashes occur

### Minor Data Staleness Issue

**Property**: `source_chunk_ids` on entities/edges

**Issue**: When an entity is UPDATED (not deleted) during document deletion, its `source_chunk_ids` still references the deleted document's chunks.

**Impact**: LOW

- Query still works
- Just returns fewer chunks (only from existing documents)
- Entity is fully cleaned when ALL referencing documents are deleted

### Root Cause

The cleanup logic updates `source_ids` but does NOT update `source_chunk_ids`:

```rust
// In cleanup_document_graph_data
if remaining_sources.len() < sources.len() {
    // Some sources were removed - update the entity
    let mut updated_props = node.properties.clone();
    updated_props.insert(
        "source_ids".to_string(),
        serde_json::json!(remaining_sources),
    );
    // ❌ source_chunk_ids is NOT updated
    graph_storage.upsert_node(&node.id, updated_props).await?;
}
```

### Fix Options

#### Option A: Clean source_chunk_ids During Deletion (DEFERRED)

Add logic to filter `source_chunk_ids` based on deleted document:

```rust
// Filter out chunk IDs that belong to deleted document
let remaining_chunks: Vec<String> = node.properties
    .get("source_chunk_ids")
    .and_then(|v| v.as_array())
    .map(|arr| arr.iter()
        .filter_map(|v| v.as_str())
        .filter(|s| !s.starts_with(&chunk_prefix))
        .map(String::from)
        .collect())
    .unwrap_or_default();
updated_props.insert("source_chunk_ids".to_string(), json!(remaining_chunks));
```

**Pros**: Cleaner data, smaller entity properties
**Cons**: More complexity, doesn't fix functional issue (none exists)

#### Option B: Leave As-Is (CHOSEN)

Keep current behavior with stale `source_chunk_ids`:

- No functional impact
- Query handles missing chunks gracefully
- Full cleanup happens when entity is deleted

**Pros**: Simple, no regression risk
**Cons**: Minor data staleness

## Decision

**Choose Option B** - The issue is cosmetic, not functional. Adding cleanup logic for `source_chunk_ids` adds complexity without fixing any actual bug.

## Testing Approach

Instead of fixing the staleness issue, we'll add an integration test to VERIFY that queries work correctly after document deletion:

1. Upload 2 documents with shared entity
2. Verify query returns context from both
3. Delete one document
4. Verify query still works with context from remaining document
5. Verify no errors occur

## Files to Create

| File                       | Purpose                                    |
| -------------------------- | ------------------------------------------ |
| `e2e_document_deletion.rs` | Add `test_query_after_deletion_works` test |
| `iteration-09/decide.md`   | This file                                  |
| `iteration-09/act.md`      | Document test addition                     |
