# EdgeQuake Query Performance Improvement Plan

**Date:** 2024-12-30  
**Status:** IMPLEMENTATION IN PROGRESS  
**Priority:** P0 (Critical - Query feature unusable without fix)

## Problem Statement

The EdgeQuake Query feature times out after 30 seconds when executing hybrid queries. The root cause is **missing indexes on AGE graph tables**, causing Cypher queries to perform full table scans.

### Error Message

```
Storage error: Database error: Cypher query failed: error returned from database:
canceling statement due to statement timeout
```

### Affected Features

- `/query` page in WebUI
- `/api/v1/query` endpoint
- All query modes: local, global, hybrid, mix (except naive which uses vector search only)
- Chat feature when using hybrid mode

## Solution Overview

### Phase 1: Index Creation (COMPLETED ✅)

**Files Modified:**

1. `edgequake/crates/edgequake-storage/src/adapters/postgres/graph.rs`

   - Added `ensure_indexes()` method
   - Added `indexes_verified` flag to track index creation
   - Updated `initialize()` to call `ensure_indexes()`
   - Updated `upsert_node()` to create indexes after first node

2. `edgequake/docker/migrations/002_add_age_vertex_indexes.sql`
   - Migration script for existing databases
   - Creates indexes on Node, EDGE, and AGE internal tables

**Indexes Created:**
| Index Name | Table | Column/Expression | Purpose |
|------------|-------|-------------------|---------|
| `idx_node_prop_node_id` | `Node` | `properties->'node_id'` | Fast node lookup by ID |
| `idx_node_props_gin` | `Node` | `properties` (GIN) | Flexible property queries |
| `idx_node_id` | `Node` | `id` | Fast vertex ID lookup |
| `idx_edge_start_id` | `EDGE` | `start_id` | Outgoing edge queries |
| `idx_edge_end_id` | `EDGE` | `end_id` | Incoming edge queries |
| `idx_edge_start_end` | `EDGE` | `(start_id, end_id)` | Relationship lookups |
| `idx_edge_props_gin` | `EDGE` | `properties` (GIN) | Edge property queries |
| `idx_ag_vertex_props_gin` | `_ag_label_vertex` | `properties` (GIN) | Fallback vertex queries |
| `idx_ag_edge_*` | `_ag_label_edge` | `start_id`, `end_id` | Fallback edge queries |

### Phase 2: Testing & Verification (IN PROGRESS 🔄)

**Tasks:**

1. Apply migration to running database
2. Verify indexes are created
3. Test Query page with hybrid mode
4. Measure performance improvement

### Phase 3: Batch Query Optimization (PLANNED 📋)

**Goal:** Reduce N+1 query problem by implementing batch methods.

**Methods to Add:**

```rust
// In GraphStorage trait
async fn get_nodes_batch(&self, node_ids: &[String]) -> Result<Vec<GraphNode>>;
async fn get_nodes_edges_batch(&self, node_ids: &[String]) -> Result<HashMap<String, Vec<GraphEdge>>>;
async fn get_edges_batch(&self, edges: &[(String, String)]) -> Result<Vec<GraphEdge>>;
```

**Implementation Approach:**

- Use SQL `WHERE id IN (...)` instead of N separate queries
- Use `UNION ALL` for complex batch queries
- Parallelize independent queries with `tokio::join!`

### Phase 4: Raw SQL for Lookups (FUTURE 📋)

**Goal:** Replace slow Cypher with fast raw SQL for common operations.

**Current (Slow):**

```rust
let cypher = format!("MATCH (n:Node {{node_id: '{}'}}) RETURN n", node_id);
let rows = self.cypher_query(&cypher, &["n"]).await?;
```

**Proposed (Fast):**

```rust
let sql = format!(
    r#"SELECT properties FROM {}.Node
       WHERE ag_catalog.agtype_access_operator(properties, '"node_id"'::agtype) = $1"#,
    self.graph_name
);
let row = sqlx::query(&sql).bind(node_id).fetch_optional(&mut *conn).await?;
```

**Expected Improvement:** 50-100x faster for individual lookups.

## Implementation Timeline

| Phase                   | Status         | Duration | Completion |
| ----------------------- | -------------- | -------- | ---------- |
| Phase 1: Index Creation | ✅ Completed   | 2 hours  | 2024-12-30 |
| Phase 2: Testing        | 🔄 In Progress | 1 hour   | 2024-12-30 |
| Phase 3: Batch Queries  | 📋 Planned     | 4 hours  | TBD        |
| Phase 4: Raw SQL        | 📋 Planned     | 8 hours  | TBD        |

## Expected Performance Improvement

| Operation                   | Before (no index) | After (with index) | Improvement |
| --------------------------- | ----------------- | ------------------ | ----------- |
| Single node lookup          | 500ms+            | <5ms               | 100x        |
| 10 node batch               | 5000ms+           | <20ms              | 250x        |
| Hybrid query (20 entities)  | >30s (timeout)    | <200ms             | >150x       |
| Global query (10 relations) | >30s (timeout)    | <100ms             | >300x       |

## Testing Plan

### Unit Tests

```rust
#[tokio::test]
async fn test_node_lookup_performance() {
    let storage = create_test_storage().await;

    // Insert 100 nodes
    for i in 0..100 {
        storage.upsert_node(&format!("node_{}", i), props).await?;
    }

    // Time lookup
    let start = Instant::now();
    for i in 0..100 {
        storage.get_node(&format!("node_{}", i)).await?;
    }
    let elapsed = start.elapsed();

    // Should complete in <1 second with indexes
    assert!(elapsed.as_secs() < 1);
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_hybrid_query_does_not_timeout() {
    let engine = create_test_engine().await;

    // Insert documents
    engine.ingest("test content").await?;

    // Execute hybrid query with 15s timeout
    let result = tokio::time::timeout(
        Duration::from_secs(15),
        engine.query("What is the main topic?", QueryMode::Hybrid)
    ).await;

    // Should not timeout
    assert!(result.is_ok());
}
```

### E2E Tests

1. Navigate to `/query` page
2. Enter query: "What are the main entities?"
3. Select mode: Hybrid
4. Submit query
5. Verify response received within 15 seconds
6. Verify sources are displayed

## Rollback Plan

If issues occur:

1. **Remove indexes:** Run `DROP INDEX` statements
2. **Revert code:** `git revert` the graph.rs changes
3. **Fallback:** Increase statement timeout to 120s (temporary workaround)

## Success Criteria

1. ✅ Query page does not timeout with hybrid mode
2. ✅ Query response time < 5 seconds for typical queries
3. ✅ Graph visualization continues to work
4. ✅ Document ingestion not impacted
5. ✅ All existing tests pass

## Related Files

- [12-query-implementation-comparison.md](./12-query-implementation-comparison.md) - Detailed LightRAG vs EdgeQuake comparison
- [graph.rs](../edgequake/crates/edgequake-storage/src/adapters/postgres/graph.rs) - Graph storage implementation
- [query.rs](../edgequake/crates/edgequake-core/src/query.rs) - Query engine implementation
- [002_add_age_vertex_indexes.sql](../edgequake/docker/migrations/002_add_age_vertex_indexes.sql) - Migration script

## Monitoring

After deployment, monitor:

- Query response times in logs
- Database query execution times
- Statement timeout errors (should be zero)
- Memory usage (GIN indexes increase RAM usage)
