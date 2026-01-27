# OODA Loop - Iteration 03: Observe

## Mission Reminder

**Re-read Mission**: `/Users/raphaelmansuy/Github/03-working/edgequake/specs/mission_workspace_dashboard_fixes/MISSION.md`

**CRITICAL BUG**: Dashboard shows 0 Entities/Relationships/Chunks despite document having 13 entities.

---

## Observations from User Screenshots

### Screenshot 1: Workspace Page

- Documents: 1
- Entities: 0 ❌ INCORRECT
- Relationships: 0 ❌ INCORRECT
- Chunks: 0 ❌ INCORRECT

### Screenshot 2: Dashboard Page

- Documents: 1
- Entities: 0 ❌ INCORRECT
- Relationships: 0 ❌ INCORRECT
- Entity Types: 1

### Screenshot 3: Documents Page

- Document: `scienti_2601.16282v1.md`
- Entities: 13 ✅ CORRECT VALUE
- Status: Completed

### Screenshot 4: Workspace Selector

- Shows "Default Workspace" and "TennantZZ" tenant

**Conclusion**: The document processing worked correctly (13 entities extracted), but the aggregated stats are showing 0.

---

## Root Cause Analysis

### Investigation Steps

1. **Backend Stats Endpoint**: [workspaces.rs](edgequake/crates/edgequake-api/src/handlers/workspaces.rs#L924-1200)

2. **Stats Retrieval Flow**:

   ```
   get_workspace_stats()
   └─> fetch_workspace_stats_uncached()
       ├─> try_postgres_stats() ← FAILS (empty tables)
       └─> try_kv_storage_stats() ← RETURNS 0 (no entity_count in metadata)
   ```

3. **PostgreSQL Query**: [workspace_service_impl.rs](edgequake/crates/edgequake-core/src/workspace_service_impl.rs#L641-645)

   ```sql
   SELECT COUNT(*) FROM entities WHERE workspace_id = $1
   SELECT COUNT(*) FROM relationships WHERE workspace_id = $1
   ```

   **Problem**: These tables are likely empty - entities are stored in Apache AGE graph, not PostgreSQL relational tables

4. **KV Storage Fallback**: [workspaces.rs](edgequake/crates/edgequake-api/src/handlers/workspaces.rs#L1077-1092)

   ```rust
   // Sum entity counts from metadata
   if let Some(count) = obj.get("entity_count").and_then(|v| v.as_u64()) {
       entity_count += count;
   }
   ```

   **Problem**: Document metadata doesn't have `entity_count` field populated

5. **Missing Fallback**: No fallback to Apache AGE graph queries for entity/relationship counts

---

## Apache AGE Graph Storage Methods Available

From [graph.rs](edgequake/crates/edgequake-api/src/handlers/graph.rs):

```rust
state.graph_storage.node_count().await?  // Returns total entity count
state.graph_storage.edge_count().await?  // Returns total relationship count
```

**But**: These are global counts, not workspace-scoped!

---

## Solution Required

Need to add workspace-scoped graph queries:

1. **Option A**: Add `node_count_by_workspace()` and `edge_count_by_workspace()` methods to graph storage trait
2. **Option B**: Query AGE graph directly with Cypher:
   ```cypher
   MATCH (n:Entity) WHERE n.workspace_id = 'workspace-uuid' RETURN COUNT(n)
   MATCH ()-[r:RELATED_TO]->() WHERE r.workspace_id = 'workspace-uuid' RETURN COUNT(r)
   ```

---

## File Locations

| Component           | File                                           | Lines    |
| ------------------- | ---------------------------------------------- | -------- |
| Stats endpoint      | `edgequake-api/src/handlers/workspaces.rs`     | 924-1200 |
| Workspace service   | `edgequake-core/src/workspace_service_impl.rs` | 620-700  |
| Graph storage trait | `edgequake-storage/src/graph.rs`               | ?        |
| PostgreSQL AGE impl | `edgequake-storage/src/age/`                   | ?        |
