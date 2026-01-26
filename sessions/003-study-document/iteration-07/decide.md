# Iteration 07: DECIDE - Fix source_ids Merge in Async Path

## Date
2025-01-28

## Decision
Implement pre-fetch merge pattern in `processor.rs` to match the sync path fix from OODA-06.

## Implementation Plan

### Step 1: Collect Entity Names Before Batch Construction

Before building `nodes_batch`, collect all entity names:
```rust
let entity_names: Vec<String> = result.extractions
    .iter()
    .flat_map(|e| e.entities.iter().map(|ent| ent.name.clone()))
    .collect();
```

### Step 2: Batch Fetch Existing Nodes

Use `get_nodes_by_ids` to fetch all existing entities in one query:
```rust
let existing_nodes = self.graph_storage
    .get_nodes_by_ids(&entity_names)
    .await
    .unwrap_or_default();

let existing_source_ids: HashMap<String, HashSet<String>> = existing_nodes
    .into_iter()
    .map(|node| {
        let sources = node.properties.get("source_ids")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        (node.id, sources)
    })
    .collect();
```

### Step 3: Merge source_ids When Building Properties

When constructing entity properties:
```rust
// Merge with existing source_ids
let mut merged_sources: HashSet<String> = existing_source_ids
    .get(&entity.name)
    .cloned()
    .unwrap_or_default();
merged_sources.insert(document_id.clone());
let source_ids: Vec<String> = merged_sources.into_iter().collect();

properties.insert("source_ids".to_string(), json!(source_ids));
```

### Step 4: Same Pattern for Edges

1. Collect edge keys (source, target pairs)
2. Batch fetch existing edges using a loop or map
3. Merge source_ids when building edge properties

### Step 5: Testing

The existing test `test_source_ids_accumulates_across_documents` in `e2e_document_deletion.rs` uses sync path. We should add an async-specific test, but since the memory provider doesn't have a true async queue, we can verify by code inspection.

## Code Locations

- **File**: `edgequake/crates/edgequake-api/src/processor.rs`
- **Function**: `process_text_insert`
- **Entity batch construction**: Lines ~720-765
- **Edge batch construction**: Lines ~767-790

## Success Criteria

1. Build passes (`cargo build --package edgequake-api`)
2. Existing tests pass (`cargo test --package edgequake-api`)
3. Code review confirms merge logic matches OODA-06 sync path pattern
