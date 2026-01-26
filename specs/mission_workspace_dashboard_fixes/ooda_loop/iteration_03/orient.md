# OODA Loop - Iteration 03: Orient

## Mission Reminder

**Re-read Mission**: `/Users/raphaelmansuy/Github/03-working/edgequake/specs/mission_workspace_dashboard_fixes/MISSION.md`

---

## Root Cause Confirmed

The stats API has a **critical architectural gap**:

```
Current: get_workspace_stats()
├─> PostgreSQL tables: SELECT COUNT(*) FROM entities/relationships
│   └─> Returns 0 (tables are empty)
└─> KV storage fallback: Aggregates from document metadata
    └─> Returns 0 (metadata doesn't have entity_count field)
```

**MISSING**: No fallback to Apache AGE graph storage!

---

## Architecture Analysis

### Data Flow Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                    Entity/Relationship Storage                       │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  Document Ingestion:                                                │
│  ┌─────────────┐                                                    │
│  │ Document    │                                                    │
│  │  Uploaded   │                                                    │
│  └──────┬──────┘                                                    │
│         │                                                           │
│         ▼                                                           │
│  ┌─────────────┐                                                    │
│  │ Pipeline    │                                                    │
│  │  Processing │                                                    │
│  └──────┬──────┘                                                    │
│         │                                                           │
│         ├──────────────────────┬──────────────────────┐            │
│         ▼                      ▼                      ▼            │
│  ┌────────────┐         ┌────────────┐        ┌────────────┐      │
│  │ KV Storage │         │ AGE Graph  │        │ PostgreSQL │      │
│  │            │         │            │        │   Tables   │      │
│  │ Metadata   │         │ Entities   │        │            │      │
│  │ (no counts)│         │Relationships        │   (EMPTY)  │      │
│  └────────────┘         │ ✅ HAS DATA│        └────────────┘      │
│                         └────────────┘                             │
│                                                                     │
│  Stats Query:                                                       │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │ get_workspace_stats(workspace_id)                            │  │
│  │  1. Query PostgreSQL tables → 0  ❌                          │  │
│  │  2. Query KV storage metadata → 0 ❌                         │  │
│  │  3. Query AGE graph? → NOT IMPLEMENTED! ❌                   │  │
│  └──────────────────────────────────────────────────────────────┘  │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Solution Design

### Add Workspace-Scoped Graph Methods

**Trait Addition** ([graph.rs](edgequake/crates/edgequake-storage/src/traits/graph.rs)):

```rust
/// Get node count for a specific workspace.
async fn node_count_by_workspace(&self, workspace_id: &uuid::Uuid) -> Result<usize> {
    // Default: fallback to global count (not accurate but better than nothing)
    self.node_count().await
}

/// Get edge count for a specific workspace.
async fn edge_count_by_workspace(&self, workspace_id: &uuid::Uuid) -> Result<usize> {
    // Default: fallback to global count
    self.edge_count().await
}
```

**PostgreSQL AGE Implementation**:

```rust
async fn node_count_by_workspace(&self, workspace_id: &uuid::Uuid) -> Result<usize> {
    let escaped_wid = Self::escape_sql_string(&workspace_id.to_string());
    let cypher = format!("MATCH (n:Node) WHERE n.workspace_id = '{}' RETURN count(n)", escaped_wid);
    let count = self.cypher_query_count(&cypher).await?;
    Ok(count as usize)
}

async fn edge_count_by_workspace(&self, workspace_id: &uuid::Uuid) -> Result<usize> {
    let escaped_wid = Self::escape_sql_string(&workspace_id.to_string());
    let cypher = format!(
        "MATCH (n:Node)-[r:EDGE]->(m:Node) WHERE n.workspace_id = '{}' OR m.workspace_id = '{}' RETURN count(r)",
        escaped_wid, escaped_wid
    );
    let count = self.cypher_query_count(&cypher).await?;
    Ok(count as usize)
}
```

### Update Stats Endpoint

Add Method 3 to the fallback chain in `try_kv_storage_stats`:

```rust
// Method 2 failed, try Method 3: Apache AGE graph (most reliable for entities/relationships)
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

## Implementation Priority

| Step | Task | Impact |
|------|------|--------|
| 1 | Add trait methods to `GraphStorage` trait | Foundation |
| 2 | Implement in `PostgresAGEGraphStorage` | Core fix |
| 3 | Provide default impl for `MemoryGraphStorage` | Testing |
| 4 | Update `try_kv_storage_stats` to query graph | Integration |
| 5 | Test with real document | Validation |

---

## Risk Assessment

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Performance impact of Cypher queries | Medium | Use same pattern as `clear_workspace` (proven) |
| Breaking existing tests | Low | Default implementation maintains compatibility |
| Multi-workspace edge counting logic | Medium | Count edge if either endpoint belongs to workspace |

---

## Expected Outcome

After fix:
- Dashboard shows: 13 Entities, ~N Relationships (actual graph data)
- Workspace page shows: accurate counts
- Stats update immediately after document processing
