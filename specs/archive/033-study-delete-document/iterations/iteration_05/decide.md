# Iteration 05: DECIDE Phase

**Date:** 2025-01-26
**Focus:** Implementing Source_ids Merge Fix

## Decision

### Priority: P0 - CRITICAL

GAP-07 undermines the entire reference counting system. Without fixing this, the OODA-01 edge deletion fix is incomplete.

## Implementation Plan

### CHANGE-IT05-01: Add Test to Prove GAP-07

First, create a test that demonstrates the gap:

```rust
#[tokio::test]
async fn test_source_ids_accumulates_across_documents() {
    // Upload doc A with entity "SHARED_TEST"
    // Upload doc B with entity "SHARED_TEST"
    // Verify source_ids contains BOTH document references
}
```

**Expected Result:** Test FAILS (proving the gap exists)

### CHANGE-IT05-02: Implement Source_ids Merging

Modify the entity storage logic in `documents.rs` to:

1. Before upserting entity, fetch existing entity (if any)
2. If exists: merge source_ids arrays (deduplicated)
3. Upsert with merged source_ids

```rust
// Before storing entity, check for existing and merge source_ids
if let Some(existing) = state.graph_storage.get_node(&entity.name).await? {
    if let Some(existing_sources) = existing.properties.get("source_ids") {
        let mut merged: HashSet<String> = existing_sources
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        merged.insert(document_id.clone());
        properties.insert("source_ids".to_string(), json!(merged.into_iter().collect::<Vec<_>>()));
    }
}
```

### CHANGE-IT05-03: Re-run Test to Verify Fix

After implementing the fix, re-run the test:

- **Expected Result:** Test PASSES

### Acceptance Criteria

- [ ] Test exists that proves GAP-07
- [ ] Source_ids is correctly merged when same entity appears in multiple documents
- [ ] Deleting one document preserves entity (still referenced by other)
- [ ] All 14+ existing tests still pass

## Risk Assessment

| Risk                                | Mitigation                             |
| ----------------------------------- | -------------------------------------- |
| Performance regression (extra read) | Accept for correctness; optimize later |
| Breaking existing tests             | Run full test suite after change       |
| Edge case: concurrent uploads       | Test concurrent scenario               |

## Files to Modify

1. `e2e_document_deletion.rs` - Add test
2. `documents.rs` - Add merge logic in entity storage section (~line 530)
