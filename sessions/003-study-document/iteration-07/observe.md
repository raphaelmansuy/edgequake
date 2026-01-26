# Iteration 07: OBSERVE - Async Processing Path Analysis

## Date

2025-01-28

## Focus Area

Verification that GAP-07 fix (source_ids merge) applies to async document processing path.

## Observation Method

Code analysis of `processor.rs` async document processing flow.

## Key Findings

### 1. Async vs Sync Processing Paths

EdgeQuake has TWO document processing paths:

| Path      | File           | Trigger                   | Usage                 |
| --------- | -------------- | ------------------------- | --------------------- |
| **Sync**  | `documents.rs` | `async_processing: false` | Direct HTTP upload    |
| **Async** | `processor.rs` | `async_processing: true`  | Background task queue |

### 2. Sync Path (FIXED in OODA-06)

In `documents.rs` lines ~510-555, we added source_ids merge logic:

```rust
// Merge source_ids with existing entity
if let Ok(Some(existing)) = graph_storage.get_node(&entity.name).await {
    if let Some(existing_sources) = existing.properties.get("source_ids") {
        // ... merge using HashSet
    }
}
```

### 3. Async Path (processor.rs) - GAP IDENTIFIED

In `processor.rs` lines 741-750, the async path does NOT merge source_ids:

```rust
properties.insert("source_ids".to_string(), json!(vec![&document_id]));
```

The flow is:

1. Build batch with `nodes_batch.push((entity.name.clone(), properties));`
2. Call `self.graph_storage.upsert_nodes_batch(&nodes_batch).await`
3. Default batch implementation calls `upsert_node` sequentially
4. `upsert_node` does full property replacement
5. **Result**: Existing source_ids are OVERWRITTEN

### 4. Same Issue for Edges

In `processor.rs` lines 765-773:

```rust
properties.insert("source_ids".to_string(), json!(vec![&document_id]));
```

Same problem - edges built without merging existing source_ids.

### 5. Code Evidence

**processor.rs:741-750** (Entity properties):

```rust
let mut properties = std::collections::HashMap::new();
properties.insert("entity_type".to_string(), json!(entity.entity_type));
properties.insert("description".to_string(), json!(entity.description));
properties.insert("importance".to_string(), json!(entity.importance));
properties.insert("source_ids".to_string(), json!(vec![&document_id]));  // <-- NO MERGE!
```

**processor.rs:765-777** (Edge properties):

```rust
let mut properties = std::collections::HashMap::new();
properties.insert("relation_type".to_string(), json!(relationship.relation_type));
// ... other properties
properties.insert("source_ids".to_string(), json!(vec![&document_id]));  // <-- NO MERGE!
```

## Risk Assessment

| Severity | Impact             | Scenario                                                                                            |
| -------- | ------------------ | --------------------------------------------------------------------------------------------------- |
| **HIGH** | Data loss          | When async_processing=true, second document overwrites first's source_ids                           |
| **HIGH** | Incorrect deletion | If doc A processed async, then doc B, deleting doc B removes entity even though doc A referenced it |

## Conclusion

**GAP-07 is STILL PRESENT in the async processing path.** The fix in OODA-06 only addressed the synchronous upload path. We need to apply the same source_ids merge logic to `processor.rs`.
