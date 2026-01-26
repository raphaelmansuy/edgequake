# OODA Loop - Iteration 03: Decide

## Mission Reminder

**Re-read Mission**: `/Users/raphaelmansuy/Github/03-working/edgequake/specs/mission_workspace_dashboard_fixes/MISSION.md`

---

## Decision: Implement Workspace-Scoped Graph Queries

**Approach**: Add workspace-scoped count methods to graph storage trait and implement AGE graph fallback in stats endpoint.

---

## Implementation Plan

### Phase 1: Extend GraphStorage Trait

**File**: `edgequake/crates/edgequake-storage/src/traits/graph.rs`

**Change**: Add two new methods after `edge_count()` (line ~540):

```rust
/// Get node count for a specific workspace.
/// 
/// Default implementation falls back to global count (not workspace-scoped).
/// Implementations should override for accurate workspace statistics.
async fn node_count_by_workspace(&self, workspace_id: &uuid::Uuid) -> Result<usize> {
    let _ = workspace_id;
    self.node_count().await
}

/// Get edge count for a specific workspace.
///
/// Default implementation falls back to global count (not workspace-scoped).
/// Implementations should override for accurate workspace statistics.
async fn edge_count_by_workspace(&self, workspace_id: &uuid::Uuid) -> Result<usize> {
    let _ = workspace_id;
    self.edge_count().await
}
```

---

### Phase 2: Implement in PostgresAGEGraphStorage

**File**: `edgequake/crates/edgequake-storage/src/adapters/postgres/graph.rs`

**Change**: Add after `edge_count()` method (line ~1535):

```rust
async fn node_count_by_workspace(&self, workspace_id: &uuid::Uuid) -> Result<usize> {
    // WHY: Filter nodes by workspace_id property for multi-tenant isolation
    // Uses same property-based filtering pattern as clear_workspace()
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
    // WHY: Count edges where either endpoint belongs to the workspace
    // This matches the deletion logic in clear_workspace() for consistency
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

---

### Phase 3: Update Stats Endpoint

**File**: `edgequake/crates/edgequake-api/src/handlers/workspaces.rs`

**Change**: Modify `try_kv_storage_stats()` function (lines 1035-1145) to add AGE graph fallback for entity/relationship counts:

Replace lines ~1077-1092 (entity/relationship counting from metadata):

```rust
// Sum entity counts from AGE graph (more reliable than metadata)
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

---

## Commit Strategy

```bash
git add -A
git commit -m "OODA-03: Add workspace-scoped graph queries for accurate stats

- Add node_count_by_workspace() and edge_count_by_workspace() to GraphStorage trait
- Implement in PostgresAGEGraphStorage with Cypher queries
- Update stats endpoint to fallback to AGE graph for entity/relationship counts
- Fixes dashboard showing 0 entities/relationships despite document extraction"
```

---

## Testing Plan

1. Run existing Rust tests to ensure no regressions
2. Upload a document and verify stats update
3. Check dashboard and workspace pages show correct counts
4. Verify stats cache invalidation works

---

## Success Criteria

- [ ] Dashboard shows correct entity count (13)
- [ ] Dashboard shows correct relationship count (> 0)
- [ ] Workspace page shows matching counts
- [ ] All Rust tests pass
