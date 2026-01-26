# Iteration 05: ORIENT Phase

**Date:** 2025-01-26
**Focus:** Analyzing Source_ids Overwrite Gap

## Critical Finding: GAP-07 Confirmed

### The Problem

When uploading multiple documents that contain the same entity:

1. **Document A uploaded:** Entity "ALICE" created with `source_ids = ["doc-a-chunk-0"]`
2. **Document B uploaded:** Entity "ALICE" upserted with `source_ids = ["doc-b-chunk-0"]`
3. **Result:** `source_ids = ["doc-b-chunk-0"]` (Document A reference LOST!)

### Evidence

#### Memory Implementation (graph.rs line 96-113)
```rust
async fn upsert_node(
    &self,
    node_id: &str,
    properties: HashMap<String, serde_json::Value>,
) -> Result<()> {
    let mut nodes = self.nodes.write()...;
    nodes.insert(node_id.to_string(), properties);  // FULL REPLACE!
    ...
}
```

#### PostgreSQL Implementation (graph.rs line 750-780)
```rust
let cypher = format!(
    "MERGE (n:Node {{node_id: '{}'}}) SET n = {}",  // SET = replaces ALL props!
    escaped_id, props_cypher
);
```

### Impact Analysis

| Scenario | Expected | Actual | Impact |
|----------|----------|--------|--------|
| Two docs share entity | source_ids = [a, b] | source_ids = [b] | HIGH - Data loss |
| Delete doc A | Entity preserved | Entity preserved | OK (accidentally) |
| Delete doc B | Entity has source_ids = [a] | Entity has source_ids = [] → DELETED | CRITICAL - Entity wrongly deleted |

### Why This Wasn't Caught Before

1. **Single document tests:** Most tests upload only one document
2. **Deletion tests:** Created entities manually with correct source_ids
3. **No integration test:** Upload doc A, upload doc B, verify source_ids merged

### Severity Assessment

**CRITICAL** - This breaks the core assumption of the reference counting system:
- Entities can be prematurely deleted when only one of multiple documents is deleted
- The deletion fix in OODA-01 is undermined by this gap
- Data integrity at risk

## Root Cause

The `upsert_node` method is designed for simple key-value replacement, not for property merging. The `source_ids` field requires special handling:

1. Check if entity exists
2. If exists: merge source_ids arrays
3. If new: set source_ids

## Options for Fix

### Option A: Fix in upsert_node (Storage Layer)
- Modify `upsert_node` to merge specific properties (source_ids)
- Pro: Centralized fix, all callers benefit
- Con: Complex change, may affect other properties

### Option B: Fix in upload handler (API Layer)
- Before upserting, fetch existing entity and merge source_ids
- Pro: Targeted fix, doesn't affect storage abstraction
- Con: Performance hit (extra read per entity)

### Option C: Create new method upsert_node_merge
- Add new storage method specifically for entity upsert with merge semantics
- Pro: Clean separation of concerns
- Con: More API surface

## Recommendation

**Option B** is recommended for immediate fix:
- Low risk of regression
- Clear intent
- Can be optimized later

## Next Steps

1. Create test that proves GAP-07
2. Implement fix in upload handler
3. Verify with test
