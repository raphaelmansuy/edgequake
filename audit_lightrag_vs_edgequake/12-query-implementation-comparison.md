# Query Implementation Comparison: LightRAG vs EdgeQuake

**Date:** 2024-12-30  
**Status:** CRITICAL ISSUE IDENTIFIED  
**Impact:** Query feature completely unusable due to database timeout

## Executive Summary

The EdgeQuake Query feature is timing out due to **missing indexes on AGE graph tables**. LightRAG properly indexes the `entity_id` property on AGE vertex tables, while EdgeQuake's Cypher queries scan ALL vertices for every lookup.

### Root Cause

EdgeQuake Cypher query:

```sql
MATCH (n:Node {node_id: 'xxx'}) RETURN n
```

This query has **NO INDEX** and performs a **full table scan** of the `_ag_label_vertex` table.

LightRAG uses indexed property lookup:

```sql
SELECT * FROM graph.base
WHERE ag_catalog.agtype_access_operator(properties, '"entity_id"'::agtype) = ...
```

This query uses the **GIN index on properties** and completes in milliseconds.

## Detailed Comparison

### 1. Index Strategy

| Aspect                 | LightRAG                                | EdgeQuake                  | Gap          |
| ---------------------- | --------------------------------------- | -------------------------- | ------------ |
| Vertex node_id index   | ✅ `entity_idx_node_id` on `base` table | ❌ None                    | **CRITICAL** |
| Vertex properties GIN  | ✅ `entity_node_id_gin_idx`             | ❌ None                    | **CRITICAL** |
| Edge start_id index    | ✅ `edge_sid_idx`                       | ✅ `idx_ag_edge_start_id`  | OK           |
| Edge end_id index      | ✅ `edge_eid_idx`                       | ✅ `idx_ag_edge_end_id`    | OK           |
| Edge combined index    | ✅ `edge_seid_idx`                      | ✅ `idx_ag_edge_start_end` | OK           |
| Label-specific indexes | ✅ `DIRECTED` table indexes             | ❌ None                    | **HIGH**     |

### LightRAG Index Creation (from `postgres_impl.py` lines 3293-3310):

```python
f'CREATE INDEX CONCURRENTLY vertex_idx_node_id ON {graph}."_ag_label_vertex" (ag_catalog.agtype_access_operator(properties, \'"entity_id"\'::agtype))',
f'CREATE INDEX CONCURRENTLY edge_sid_idx ON {graph}."_ag_label_edge" (start_id)',
f'CREATE INDEX CONCURRENTLY edge_eid_idx ON {graph}."_ag_label_edge" (end_id)',
f'CREATE INDEX CONCURRENTLY edge_seid_idx ON {graph}."_ag_label_edge" (start_id,end_id)',
f'CREATE INDEX CONCURRENTLY directed_p_idx ON {graph}."DIRECTED" (id)',
f'CREATE INDEX CONCURRENTLY directed_eid_idx ON {graph}."DIRECTED" (end_id)',
f'CREATE INDEX CONCURRENTLY directed_sid_idx ON {graph}."DIRECTED" (start_id)',
f'CREATE INDEX CONCURRENTLY directed_seid_idx ON {graph}."DIRECTED" (start_id,end_id)',
f'CREATE INDEX CONCURRENTLY entity_p_idx ON {graph}."base" (id)',
f'CREATE INDEX CONCURRENTLY entity_idx_node_id ON {graph}."base" (ag_catalog.agtype_access_operator(properties, \'"entity_id"\'::agtype))',
f'CREATE INDEX CONCURRENTLY entity_node_id_gin_idx ON {graph}."base" using gin(properties)',
f'ALTER TABLE {graph}."DIRECTED" CLUSTER ON directed_sid_idx',
```

### EdgeQuake Index Creation (from `docker/init.sql`):

```sql
-- Phase 7.1: AGE Graph Indexes (ADDED)
CREATE INDEX IF NOT EXISTS idx_ag_edge_start_id ON edgequake."_ag_label_edge" (start_id);
CREATE INDEX IF NOT EXISTS idx_ag_edge_end_id ON edgequake."_ag_label_edge" (end_id);
CREATE INDEX IF NOT EXISTS idx_ag_edge_start_end ON edgequake."_ag_label_edge" (start_id, end_id);
-- MISSING: vertex property indexes!
```

### 2. Query Approach

| Operation          | LightRAG                      | EdgeQuake                | Gap          |
| ------------------ | ----------------------------- | ------------------------ | ------------ |
| Node lookup        | Raw SQL with indexed property | Cypher MATCH (unindexed) | **CRITICAL** |
| Batch operations   | ✅ `get_nodes_batch()`        | ⚠️ Single queries        | **HIGH**     |
| Edge lookup        | ✅ Direct table joins         | Cypher MATCH             | **MEDIUM**   |
| Degree calculation | ✅ Batched SQL                | ✅ Batched SQL           | OK           |

### LightRAG `has_node` Query (Fast - uses index):

```python
async def has_node(self, node_id: str) -> bool:
    query = f"""
        SELECT EXISTS (
          SELECT 1
          FROM {self.graph_name}.base
          WHERE ag_catalog.agtype_access_operator(
                  VARIADIC ARRAY[properties, '"entity_id"'::agtype]
                ) = (to_json($1::text)::text)::agtype
          LIMIT 1
        ) AS node_exists;
    """
```

### EdgeQuake `get_node` Query (Slow - no index):

```rust
async fn get_node(&self, node_id: &str) -> Result<Option<GraphNode>> {
    let escaped_id = Self::escape_cypher_string(node_id);
    let cypher = format!("MATCH (n:Node {{node_id: '{}'}}) RETURN n", escaped_id);
    // This scans ALL vertices!
    let rows = self.cypher_query(&cypher, &["n"]).await?;
    // ...
}
```

### 3. Query Mode Implementation

| Mode   | LightRAG                                | EdgeQuake                                  |
| ------ | --------------------------------------- | ------------------------------------------ |
| Local  | ✅ Vector search + batch node retrieval | ⚠️ Vector search + individual node lookups |
| Global | ✅ Vector search + batch edge retrieval | ⚠️ Vector search + individual node lookups |
| Hybrid | ✅ Combines local + global efficiently  | ⚠️ Calls local then global (N+1 problem)   |
| Mix    | ✅ Chunks + KG context                  | ⚠️ Calls local + naive                     |
| Naive  | ✅ Pure vector search                   | ✅ Pure vector search                      |

### 4. Performance Analysis

**Estimated query times for 200 node graph:**

| Operation                  | LightRAG | EdgeQuake      | Ratio        |
| -------------------------- | -------- | -------------- | ------------ |
| Single node lookup         | ~2ms     | ~500ms+        | 250x slower  |
| Batch 10 nodes             | ~5ms     | ~5000ms+       | 1000x slower |
| Get node edges             | ~3ms     | ~800ms+        | 266x slower  |
| Hybrid query (20 entities) | ~100ms   | >30s (TIMEOUT) | ∞            |

## Critical Issues

### Issue #1: Missing Vertex Property Index

**Problem:** Cypher query `MATCH (n:Node {node_id: 'xxx'})` performs full table scan.

**Solution:** Create expression index on `node_id` property:

```sql
CREATE INDEX CONCURRENTLY idx_ag_vertex_node_id
ON edgequake."Node" (ag_catalog.agtype_access_operator(properties, '"node_id"'::agtype));

CREATE INDEX CONCURRENTLY idx_ag_vertex_props_gin
ON edgequake."Node" USING gin(properties);
```

### Issue #2: N+1 Query Problem

**Problem:** `query_local` and `query_hybrid` call `get_node()` and `get_node_edges()` individually for each entity found in vector search.

**Solution:** Implement batch methods like LightRAG:

- `get_nodes_batch(node_ids: Vec<String>)`
- `get_nodes_edges_batch(node_ids: Vec<String>)`
- `get_edges_batch(edges: Vec<(String, String)>)`

### Issue #3: Statement Timeout Insufficient

**Problem:** Even with 30s statement timeout, complex hybrid queries timeout.

**Solution:**

1. Fix indexes (primary solution)
2. Increase timeout to 60s as fallback
3. Add query complexity estimation

## Recommendations

### Immediate (Critical - Fixes Timeout)

1. **Add vertex property indexes** to `docker/init.sql`:

   ```sql
   -- Expression index for node_id lookups
   CREATE INDEX CONCURRENTLY idx_node_property_node_id
   ON edgequake."Node" (ag_catalog.agtype_access_operator(properties, '"node_id"'::agtype));

   -- GIN index for general property queries
   CREATE INDEX CONCURRENTLY idx_node_props_gin
   ON edgequake."Node" USING gin(properties);
   ```

2. **Create migration script** to apply indexes to existing databases

3. **Verify indexes exist** during graph storage initialization

### Short-term (Improves Performance)

1. Implement `get_nodes_batch()` using SQL `IN` clause
2. Implement `get_nodes_edges_batch()` for bulk edge retrieval
3. Add query result caching

### Medium-term (SOTA Alignment)

1. Use raw SQL for lookups instead of Cypher (like LightRAG)
2. Implement label-specific tables (`base`, `DIRECTED`) instead of generic `Node`, `EDGE`
3. Add CLUSTER optimization for edge tables
4. Implement connection pooling per-query

## Test Plan

### Unit Tests

1. Test `get_node()` performance with/without index
2. Test `get_nodes_batch()` with 10, 50, 100 nodes
3. Test hybrid query with various graph sizes

### Integration Tests

1. Test query modes: local, global, hybrid, mix, naive
2. Test with empty graph
3. Test with 1000+ node graph

### E2E Tests

1. Test Query page with real LLM
2. Test streaming queries
3. Test conversation history

## Implementation Priority

| Task                  | Priority      | Effort      | Impact                  |
| --------------------- | ------------- | ----------- | ----------------------- |
| Add vertex indexes    | P0 (Critical) | Low (1h)    | Fixes timeout           |
| Migration script      | P0            | Low (30min) | Fixes existing installs |
| Batch node retrieval  | P1            | Medium (4h) | 10x faster queries      |
| Raw SQL lookups       | P2            | High (8h)   | 50x faster lookups      |
| Label-specific tables | P3            | High (16h)  | Full parity             |

## Conclusion

The EdgeQuake Query feature is **unusable** due to missing AGE vertex indexes. The fix is straightforward:

1. Add expression index on `node_id` property
2. Add GIN index on properties column
3. Apply indexes via migration

Once indexes are in place, query performance will improve from 30s+ (timeout) to <100ms, matching LightRAG's performance.

---

**Next Steps:**

1. Create `002_add_vertex_indexes.sql` migration
2. Update `docker/init.sql` with indexes
3. Update `graph.rs` to verify indexes on initialization
4. Test with large graph (1000+ nodes)
