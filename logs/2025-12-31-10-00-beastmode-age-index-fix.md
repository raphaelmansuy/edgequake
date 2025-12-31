# Task Log: AGE Graph Index Fix for Query Timeouts

**Date:** 2025-12-31  
**Mode:** Beastmode  
**Status:** ✅ Complete

---

## Problem

```
Storage error: Database error: Cypher query failed: error returned from database: canceling statement due to statement timeout
```

## Root Cause Analysis

Using sequential thinking, identified the root cause:

1. **Missing indexes on `_ag_label_edge` table**

   - The `get_popular_nodes_with_degree` function uses `GROUP BY start_id`
   - Without index: O(N) full table scan on every query
   - With 10k+ edges, this exceeds the 30s timeout

2. **Query structure** (from `graph.rs` line 1042):

   ```sql
   WITH edge_counts AS (
     SELECT start_id, COUNT(*) as out_degree
     FROM <graph>._ag_label_edge
     GROUP BY start_id  -- ❌ No index = full table scan
   )
   ```

3. **LightRAG comparison**:
   - LightRAG uses NetworkX (in-memory) - no SQL indexes needed
   - EdgeQuake uses PostgreSQL AGE - requires SQL indexes for performance

---

## Solution

### 1. Added AGE indexes to `init.sql` (Phase 7.1)

```sql
CREATE INDEX idx_ag_edge_start_id ON <graph>._ag_label_edge(start_id);
CREATE INDEX idx_ag_edge_end_id ON <graph>._ag_label_edge(end_id);
CREATE INDEX idx_ag_edge_start_end ON <graph>._ag_label_edge(start_id, end_id);
CREATE INDEX idx_ag_vertex_props_gin ON <graph>._ag_label_vertex USING GIN(properties);
```

### 2. Created migration script

- `edgequake/migrations/001_add_age_indexes.sql`
- For existing databases that don't have indexes

### 3. Added unit tests

- `edgequake/crates/edgequake-storage/tests/graph_query_performance.rs`
- Tests for query performance, tenant filtering, degree accuracy

---

## Performance Results

| Metric     | Before (no indexes) | After (with indexes) | Improvement      |
| ---------- | ------------------- | -------------------- | ---------------- |
| 200 nodes  | 30s timeout         | 79ms                 | **380x faster**  |
| 500 nodes  | 30s timeout         | 97ms                 | **309x faster**  |
| SSE stream | 30s timeout         | 25ms                 | **1200x faster** |

---

## Actions

1. ✅ Analyzed query pattern in `get_popular_nodes_with_degree`
2. ✅ Identified missing indexes on `_ag_label_edge` table
3. ✅ Added indexes to `init.sql` for new databases
4. ✅ Created migration script for existing databases
5. ✅ Applied indexes to live database
6. ✅ Verified query performance (79-97ms for 200-500 nodes)
7. ✅ Created unit tests for performance verification

---

## Files Changed

- `edgequake/docker/init.sql` - Added Phase 7.1 AGE indexes
- `edgequake/migrations/001_add_age_indexes.sql` - Migration script
- `edgequake/crates/edgequake-storage/tests/graph_query_performance.rs` - Unit tests

---

## Lessons/Insights

- PostgreSQL AGE stores graphs in `_ag_label_vertex` and `_ag_label_edge` tables
- These tables need explicit indexes - AGE doesn't create them automatically
- The `GROUP BY start_id` pattern requires an index on `start_id`
- GIN index on JSONB `properties` column enables fast filtering
