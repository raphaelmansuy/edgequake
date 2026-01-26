# Iteration 06: OBSERVE → ORIENT → DECIDE Phase (Combined)

**Date:** 2025-01-26
**Focus:** Fixing GAP-07 (source_ids merge)

## Observation

Found the exact location where the bug occurs:

**File:** `documents.rs` lines 530-541

```rust
properties.insert(
    "source_ids".to_string(),
    serde_json::json!(vec![&document_id]),  // <-- BUG: Only current doc!
);
// ...
state
    .graph_storage
    .upsert_node(&entity.name, properties)
    .await?;  // <-- Replaces ALL properties
```

## Orientation

The fix needs to:
1. Check if entity already exists
2. If exists, get current source_ids
3. Merge new document reference with existing source_ids
4. Upsert with merged source_ids

Same pattern needed for edges (lines 582-595).

## Decision

### CHANGE-IT06-01: Entity source_ids Merge

Before upserting entity, check for existing and merge source_ids:

```rust
// Before properties.insert("source_ids"...)
let merged_source_ids = if let Ok(Some(existing)) = state.graph_storage.get_node(&entity.name).await {
    let mut existing_sources: std::collections::HashSet<String> = existing.properties
        .get("source_ids")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();
    existing_sources.insert(document_id.clone());
    existing_sources.into_iter().collect::<Vec<_>>()
} else {
    vec![document_id.clone()]
};

properties.insert("source_ids".to_string(), serde_json::json!(merged_source_ids));
```

### CHANGE-IT06-02: Edge source_ids Merge

Same pattern for edges.

## Implementation Plan

1. Modify entity storage section (~line 510-541)
2. Modify edge storage section (~line 569-600)
3. Run tests to verify GAP-07 is fixed
4. Log message should change to "GAP-07 NOT PRESENT"
