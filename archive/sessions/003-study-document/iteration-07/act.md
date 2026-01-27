# Iteration 07: ACT - Fixed source_ids Merge in Async Processing Path

## Date

2025-01-28

## Summary

Extended GAP-07 fix from sync path (OODA-06) to async processing path in `processor.rs`.

## Change Made

**File**: `edgequake/crates/edgequake-api/src/processor.rs`

**Function**: `process_text_insert` (async document processing)

### Before (GAP-07 Vulnerable)

```rust
// Entity properties - NO MERGE
properties.insert("source_ids".to_string(), json!(vec![&document_id]));

// Edge properties - NO MERGE
properties.insert("source_ids".to_string(), json!(vec![&document_id]));
```

### After (Fixed)

```rust
// OODA-07: Pre-fetch existing entities to merge source_ids
let entity_names: Vec<String> = result.extractions
    .iter()
    .flat_map(|e| e.entities.iter().map(|ent| ent.name.clone()))
    .collect();

let existing_entity_source_ids: HashMap<String, HashSet<String>> =
    self.graph_storage.get_nodes_by_ids(&entity_names).await
        .map(|nodes| /* extract source_ids */)
        .unwrap_or_default();

// When building entity properties:
let mut merged_sources: HashSet<String> = existing_entity_source_ids
    .get(&entity.name)
    .cloned()
    .unwrap_or_default();
merged_sources.insert(document_id.clone());
properties.insert("source_ids".to_string(), json!(source_ids_vec));

// Same pattern for edges using get_edge() pre-fetch
```

## Code Changes Summary

| Section          | Lines Changed | Description                                           |
| ---------------- | ------------- | ----------------------------------------------------- |
| Entity pre-fetch | +20 lines     | Batch fetch existing entities with `get_nodes_by_ids` |
| Edge pre-fetch   | +15 lines     | Fetch existing edges with `get_edge` loop             |
| Entity merge     | Modified      | Use HashSet merge instead of direct `vec![doc_id]`    |
| Edge merge       | Modified      | Use HashSet merge for edge source_ids                 |

## Testing

| Test Suite                            | Result          |
| ------------------------------------- | --------------- |
| `cargo build --package edgequake-api` | ✅ Success      |
| `cargo test --package edgequake-api`  | ✅ 30/30 passed |

## Impact Analysis

### Correctness

- ✅ Async path now correctly accumulates source_ids across documents
- ✅ Reference counting works for entities shared between documents
- ✅ Deletion safety maintained when processing via async queue

### Performance

- Minor overhead: One additional `get_nodes_by_ids` query per document
- Minor overhead: Edge pre-fetch loop (could be optimized to batch in future)
- Trade-off acceptable: Correctness > marginal performance

## Verification

Both sync and async paths now have consistent source_ids merge logic:

| Path  | File           | Merge Logic    |
| ----- | -------------- | -------------- |
| Sync  | `documents.rs` | ✅ OODA-06 fix |
| Async | `processor.rs` | ✅ OODA-07 fix |

## WHY Comments Added

```rust
// OODA-07: Pre-fetch existing entities to merge source_ids (GAP-07 fix for async path)
// WHY: Without merge, second document overwrites first's source_ids, breaking reference counting

// OODA-07: Pre-fetch existing edges to merge source_ids
// WHY: Same issue as entities - edges need reference counting for correct deletion
```

## Next Steps

1. Consider batch edge fetch optimization (currently loop-based)
2. Verify PostgreSQL provider works with this pattern
3. Continue OODA iterations for remaining document lifecycle analysis
