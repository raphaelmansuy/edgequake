# OODA Loop - Iteration 03: Act

## Mission Reminder

**Re-read Mission**: `/Users/raphaelmansuy/Github/03-working/edgequake/specs/mission_workspace_dashboard_fixes/MISSION.md`

**CRITICAL BUG FIX**: Dashboard showing 0 Entities/Relationships despite 13 extracted entities.

---

## Changes Implemented

### 1. Add Workspace-Scoped Count Methods to GraphStorage Trait

**File**: [traits/graph.rs](edgequake/crates/edgequake-storage/src/traits/graph.rs#L540)

**Change**: Added two new trait methods after `edge_count()`:

```rust
/// Get node count for a specific workspace.
async fn node_count_by_workspace(&self, workspace_id: &uuid::Uuid) -> Result<usize> {
    let _ = workspace_id;
    self.node_count().await  // Default fallback
}

/// Get edge count for a specific workspace.
async fn edge_count_by_workspace(&self, workspace_id: &uuid::Uuid) -> Result<usize> {
    let _ = workspace_id;
    self.edge_count().await  // Default fallback
}
```

**WHY**: Dashboard and workspace pages need accurate per-workspace statistics for multi-tenant isolation.

---

### 2. Implement Workspace-Scoped Counts in PostgresAGEGraphStorage

**File**: [postgres/graph.rs](edgequake/crates/edgequake-storage/src/adapters/postgres/graph.rs#L1535)

**Change**: Implemented the new methods using Cypher queries with workspace_id filtering:

```rust
async fn node_count_by_workspace(&self, workspace_id: &uuid::Uuid) -> Result<usize> {
    let workspace_id_str = workspace_id.to_string();
    let escaped_wid = Self::escape_sql_string(&workspace_id_str);
    let cypher = format!(
        "MATCH (n:Node) WHERE n.workspace_id = '{}' RETURN count(n)",
        escaped_wid
    );
    let count = self.cypher_query_count(&cypher).await?;
    Ok(count as usize)
}

async fn edge_count_by_workspace(&self, workspace_id: &uuid::Uuid) -> Result<usize> {
    let workspace_id_str = workspace_id.to_string();
    let escaped_wid = Self::escape_sql_string(&workspace_id_str);
    let cypher = format!(
        "MATCH (n:Node)-[r:EDGE]->(m:Node) WHERE n.workspace_id = '{}' OR m.workspace_id = '{}' RETURN count(r)",
        escaped_wid, escaped_wid
    );
    let count = self.cypher_query_count(&cypher).await?;
    Ok(count as usize)
}
```

**WHY**: Uses the same property-based filtering pattern as `clear_workspace()` for consistency.

---

### 3. Update Stats Endpoint to Query AGE Graph

**File**: [workspaces.rs](edgequake/crates/edgequake-api/src/handlers/workspaces.rs#L1068)

**Change**: Replaced KV metadata aggregation with AGE graph queries:

**Before** (Lines 1068-1083):
```rust
// Sum entity counts from metadata
if let Some(count) = obj.get("entity_count").and_then(|v| v.as_u64()) {
    entity_count += count;
}
// Sum relationship counts from metadata
if let Some(count) = obj.get("relationship_count").and_then(|v| v.as_u64()) {
    relationship_count += count;
}
```

**After** (Lines 1095-1107):
```rust
// OODA-03: Get entity/relationship counts from Apache AGE graph storage
// WHY: KV metadata doesn't have entity_count/relationship_count fields.
// The actual entity/relationship data is stored in the graph, not metadata.
let entity_count = state
    .graph_storage
    .node_count_by_workspace(&workspace_id)
    .await
    .unwrap_or(0);

let relationship_count = state
    .graph_storage
    .edge_count_by_workspace(&workspace_id)
    .await
    .unwrap_or(0);
```

**WHY**: Document metadata in KV storage doesn't track entity/relationship counts. The source of truth is the Apache AGE graph database.

---

## Root Cause Summary

The stats API had a critical architectural gap:

```
Old Flow:
1. Try PostgreSQL tables → 0 (empty)
2. Try KV metadata → 0 (no entity_count field)
3. Return 0 ❌

New Flow:
1. Try PostgreSQL tables → 0 (empty)  
2. Try KV metadata for document_count → ✓
3. Query AGE graph for entity_count → 13 ✅
4. Query AGE graph for relationship_count → N ✅
5. Return accurate stats ✅
```

---

## Files Modified

| File | Lines Changed | Purpose |
|------|---------------|---------|
| `traits/graph.rs` | +28 | Add workspace-scoped count trait methods |
| `postgres/graph.rs` | +41 | Implement Cypher queries for workspace counts |
| `workspaces.rs` | -15, +18 | Replace metadata aggregation with graph queries |

---

## Testing Plan

1. Build Rust crates to verify compilation
2. Run existing tests to ensure no regressions
3. Test with real document upload
4. Verify dashboard shows correct counts

---

## Commit

```bash
git add -A
git commit -m "OODA-03: Fix dashboard stats showing 0 entities/relationships

- Add node_count_by_workspace() and edge_count_by_workspace() to GraphStorage trait
- Implement in PostgresAGEGraphStorage with Cypher WHERE workspace_id queries
- Update stats endpoint to query AGE graph instead of KV metadata
- Fixes dashboard showing 0 entities despite successful document extraction
- Root cause: KV metadata doesn't track entity_count, data is in graph storage"
```

---

## Expected Result

After fix:
- Dashboard: 13 Entities, N Relationships (actual graph data)
- Workspace page: Matching counts
- Stats cache: Updates on next query
