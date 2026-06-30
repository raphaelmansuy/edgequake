# SPEC-034-004: Database Evidence — Live Measurements

> **Lens**: Postgres AGE Expert / pgvector Expert  
> **Version**: 1.0.0 — 2026-06-30  
> **Database**: edgequake (Docker, PostgreSQL with AGE + pgvector)  
> **Measurement date**: 2026-06-30

---

## 1. Database State at Measurement Time

```sql
-- Graph node/edge counts
SELECT COUNT(*) FROM eq_eq_default_graph."Node";   -- 52,915 nodes
SELECT COUNT(*) FROM eq_eq_default_graph."EDGE";   -- 67,636 edges

-- Vector table
SELECT COUNT(*) FROM eq_eq_default_ws_00000000_vectors;  -- 5,898 vectors

-- KV table  
SELECT COUNT(*) FROM eq_eq_default_kv;  -- 1,843 records
```

---

## 2. Table Size Analysis

```
TABLE                                      | HEAP    | TOTAL    | RATIO
─────────────────────────────────────────────────────────────────────────
eq_eq_default_ws_00000000_vectors          | 74 MB   | 1,683 MB | 22.7×
eq_eq_default_kv                           | 760 kB  | 256 MB   | 345×
eq_eq_default_graph."EDGE"                 | 36 MB   | 73 MB    | 2.0×
eq_eq_default_graph."Node"                 | 30 MB   | 63 MB    | 2.1×
─────────────────────────────────────────────────────────────────────────
```

**Interpretation**:

- `vectors` table: 22.7× ratio means indexes are 21.7× the heap data size
- `kv` table: 345× ratio — for every 1 byte stored, 344 bytes of index exist
- `EDGE`/`Node` tables: 2× ratio is acceptable for graph data

---

## 3. Index Breakdown — Vectors Table (Code + Scan Audit)

```
INDEX NAME                                      | SCANS  | SIZE    | CODE PATH
────────────────────────────────────────────────────────────────────────────────
eq_eq_default_ws_00000000_vectors_pkey          | 10,734 | 9.3 MB  | vector ID lookups (upsert)
eq_eq_default_ws_00000000_vectors_embedding_idx |      0 | 909 MB  | ← HNSW — ZERO queries!
eq_eq_default_ws_00000000_vectors_metadata_idx  |      0 | 13 MB   | ← GIN metadata — never queried
eq_eq_default_ws_00000000_vectors_content_tsv_  |      0 | 1.9 MB  | ← FTS — unused in current workload
idx_eq_eq_..._vectors_content_tsv               |      0 | 1.9 MB  | ← DUPLICATE of above
eq_eq_..._vectors_tenant_ws_idx                 |      0 | 1.6 MB  | ← btree — unused in current workload
eq_eq_..._vectors_doc_id_idx                    |      0 | 88 kB   | ← btree — unused in current workload
────────────────────────────────────────────────────────────────────────────────
TOTAL INDEX SIZE                                |        | ~938 MB
HEAP SIZE                                       |        | 74 MB
INDEX/HEAP RATIO                                |        | 12.7×
```

**Key findings**:

1. **HNSW 0 scans in this stat snapshot** — dev workload has not triggered ANN
   similarity queries since last pg_stat reset. The HNSW index IS needed for
   production `ORDER BY embedding <=> $1::vector LIMIT N` queries (confirmed in
   `vector/storage_impl.rs`). The problem is not that it's unneeded — it's that
   it's **disproportionately large** (909 MB vs 74 MB heap = 12.3×) and every
   INSERT via `ON CONFLICT DO UPDATE` must update this 909 MB index incrementally.

2. **Duplicate FTS index** — two identical GIN indexes on `content_tsv` (1.9 MB each).

3. **Metadata GIN never queried** — the 13 MB GIN on metadata JSONB has 0 scans.
   No query in the codebase does containment search on vector metadata — queries
   filter by `metadata->>'document_id'` (uses the `doc_id_idx` btree) or
   `metadata->>'workspace_id'` (uses `tenant_ws_idx`).

This single HNSW index accounts for **40% of the total 2,250 MB database size**.

**HNSW Parameters**:
```sql
-- HNSW index definition
CREATE INDEX ... USING hnsw (embedding vector_cosine_ops) 
  WITH (m='16', ef_construction='64')
-- Embedding dimensions: 1024
-- Each vector: 1024 × 4 bytes = 4,096 bytes = 4 KB
-- 5,898 vectors × 4 KB = 23.4 MB raw
-- HNSW index: 909 MB = 38.8× raw vector data
```

---

## 4. Index Breakdown — AGE Node Table (Code-Verified)

All read queries use `_ag_label_vertex` (parent table). PostgreSQL table
inheritance means those queries plan against `Node` child and use its indexes.

```
INDEX NAME                    | SCANS  | SIZE    | USED BY (code audit)
──────────────────────────────────────────────────────────────────────────────────
idx_node_id                   | 55,944 | 3.0 MB  | AGE internal (Cypher traversal)
idx_node_props_gin            | 55,701 | 19 MB   | Cypher MERGE/MATCH writes
idx_node_prop_node_id         | 6,962  | 5.2 MB  | pg_get_nodes_batch (agtype_access_operator join)
idx_node_prop_node_id_btree   | 2,885  | 3.8 MB  | pg_node_degree, pg_node_degrees_batch,
                              |        |         | pg_get_incident_edges_batch (via inheritance)
idx_node_workspace_id         | 1      | 1.0 MB  | pg_search_nodes, pg_list_nodes_filtered
                              |        |         | (low in dev, will grow in multi-workspace prod)
idx_node_tenant_id            | 0      | 1.0 MB  | pg_search_nodes, pg_list_nodes_filtered
                              |        |         | (multi-tenant; 0 scans = single-tenant dev)
──────────────────────────────────────────────────────────────────────────────────
── _ag_label_vertex PARENT table indexes ────────────────────────────────────────
── All are on the EMPTY parent table. Data lives in "Node" child. ───────────────
── PostgreSQL uses Node child indexes for inherited queries. These ───────────────
── indexes never hold data and are NEVER used → safe to drop. ───────────────────
idx_eq_..._graph_node_id      | 12     | 8192 B  | Scans on empty parent = 0 rows returned
idx_ag_vertex_props_gin       | 0      | 16 kB   | Never used
idx_ag_vertex_tenant_id       | 0      | 8192 B  | Never used
idx_ag_vertex_workspace_id    | 0      | 8192 B  | Never used
...(7 more _ag_label_vertex indexes, all 0 scans, all 8192 bytes)...
──────────────────────────────────────────────────────────────────────────────────
KEEP: 6 indexes on Node (all serving distinct query patterns)
DROP: ~10 indexes on _ag_label_vertex (empty parent table artifacts)
```

**Key discovery from EXPLAIN**: `pg_node_degree` uses `_ag_label_vertex` as the
entry point, but PostgreSQL's Append plan routes through `"Node"` child and uses
`idx_node_prop_node_id_btree` — **confirmed 0.106ms** for a single node lookup.

---

## 4b. Critical New Finding — Incident Edge Query Full Seq Scan

The `pg_get_incident_edges_batch` query (used in every BFS knowledge graph
traversal) does a **full sequential scan of all 67,636 edges**:

```sql
-- Pattern from edges_ops.rs pg_get_incident_edges_batch()
FROM eq_eq_default_graph._ag_label_edge e
JOIN eq_eq_default_graph._ag_label_vertex sv ON e.start_id::text = sv.id::text
WHERE agtype_to_json(sv.properties)->>'node_id' = 'SARAH_CHEN'
```

```
EXPLAIN output:
  Hash Join  (cost=8.47..6269.29)
    Hash Cond: ((e.start_id)::text = (sv.id)::text)
    ->  Seq Scan on "EDGE" e_2  (cost=0.00..5322.36 rows=67636)
                                 ↑ FULL SCAN of 67,636 edges!
    ->  Hash (Index Scan using idx_node_prop_node_id_btree)  ← vertex fast
```

**Root cause**: `idx_edge_start_id` is on `start_id` (graphid type). The join
condition `e.start_id::text = sv.id::text` uses a text cast. PostgreSQL cannot
use the graphid btree for the casted text comparison without an expression index
on `(start_id::text)`.

**Impact**: Every BFS step in `pg_get_knowledge_graph_scoped` calls
`pg_get_incident_edges_batch`. For a 3-level BFS with 100-node frontiers:
- 3 calls × 67,636 full edge scans = **202,908 rows examined**
- Current time: ~200ms per BFS level at current scale
- At 500K edges: 3 × 500,000 = 1.5M rows per query

**Fix required**: Expression index `ON "EDGE" ((start_id::text))` and
`ON "EDGE" ((end_id::text))`.

---

## 4c. EDGE Table Index Code-Verified Breakdown

```
INDEX NAME                    | SCANS  | SIZE    | USED BY (code audit)
──────────────────────────────────────────────────────────────────────────────────
idx_edge_start_id             | 7,048  | 2.7 MB  | AGE internal traversal (graphid btree)
idx_edge_source_id            | 1,600  | 1.8 MB  | pg_get_edges_for_node_set: source_id IN (...)
idx_edge_target_id            | 1,600  | 2.1 MB  | pg_get_edges_for_node_set: target_id IN (...)
idx_edge_end_id               | 0      | 3.0 MB  | NOT used — pg_get_incident_edges_batch does
                              |        |         | Hash Join (seq scan), not Nested Loop + index
                              |        |         | ⚠ Keep: needed when end_id IN filter queries
                              |        |         | are added or planner switches to NL join
idx_edge_props_gin            | 0      | 17 MB   | NO code queries edge GIN → ❌ DROP SAFE
idx_edge_start_end            | 0      | 4.9 MB  | Composite never scanned → ❌ DROP SAFE
idx_edge_source_target_btree  | 0      | 4.8 MB  | Composite never scanned → ❌ DROP SAFE
──────────────────────────────────────────────────────────────────────────────────
── _ag_label_edge PARENT table indexes (all 8192 bytes = empty) ─────────────────
── Same inheritance story as _ag_label_vertex — all safe to drop ────────────────
All 10 _ag_label_edge parent indexes | 0 | 8192 B each | Never used → ❌ DROP ALL
──────────────────────────────────────────────────────────────────────────────────
MISSING INDEX (New Requirement):
  ON "EDGE" ((start_id::text))   — for pg_get_incident_edges_batch Hash Join
  ON "EDGE" ((end_id::text))     — symmetric, for target-side incident lookups
```

**Key insight**: The `pg_get_incident_edges_batch` joins via `e.start_id::text =
sv.id::text`. The existing `idx_edge_start_id` is on raw `graphid` type —
PostgreSQL **cannot use it for the text cast comparison**. Adding expression
indexes on `(start_id::text)` and `(end_id::text)` will convert the Hash Join
(full seq scan) to a Nested Loop + Index Scan.

---

## 5. EXPLAIN ANALYZE Evidence

### 5.1 Cypher MATCH (single node lookup)

```sql
SET search_path = ag_catalog, public; LOAD 'age';
EXPLAIN (ANALYZE, BUFFERS) 
SELECT * FROM cypher('eq_eq_default_graph', $$
  MATCH (n:Node {node_id: 'TEST_ENTITY'}) RETURN n LIMIT 1
$$) AS (n agtype);
```

```
RESULT:
  Seq Scan on "Node" n  (cost=0.00..4446.06 rows=5 width=32)
                        (actual time=12.371..12.371 rows=0 loops=1)
    Filter: (properties @> '{"node_id": "TEST_ENTITY"}'::agtype)
    Rows Removed by Filter: 52,915        ← FULL TABLE SCAN
    Buffers: shared hit=3875
  Execution Time: 12.390 ms
```

**Finding**: Full sequential scan of 52,915 nodes — O(G) not O(log G).

---

### 5.2 Native SQL Equivalent (btree index)

```sql
SELECT id, properties FROM eq_eq_default_graph."Node"
WHERE (ag_catalog.agtype_to_json(properties)->>'node_id') = 'TEST_ENTITY'
LIMIT 1;
```

```
RESULT:
  Index Scan using idx_node_prop_node_id_btree on "Node"
    (cost=0.41..8.43 rows=1 width=475) 
    (actual time=0.064..0.064 rows=0 loops=1)
    Index Cond: ((agtype_to_json(properties) ->> 'node_id') = 'TEST_ENTITY')
    Buffers: shared read=3
  Execution Time: 0.081 ms
```

**Finding**: btree index scan — 0.081 ms vs 12.390 ms = **153× faster**.

---

### 5.3 Cypher MERGE (single node upsert)

```sql
EXPLAIN (ANALYZE, BUFFERS)
SELECT * FROM cypher('eq_eq_default_graph', $$
  MERGE (n:Node {node_id: 'SARAH_CHEN'}) 
  SET n.entity_type = 'PERSON', n.description = 'test'
  RETURN n
$$) AS (n agtype);
```

```
RESULT:
  Custom Scan (Cypher Set) (actual time=5.578..5.580 rows=1 loops=1)
    Buffers: shared hit=1317 read=280 dirtied=10
    -> Custom Scan (Cypher Merge) (actual time=3.634..3.635 rows=1)
         -> Bitmap Heap Scan on "Node" n
              Recheck Cond: (properties @> '{"node_id": "SARAH_CHEN"}'::agtype)
              Buffers: shared hit=3 read=263       ← 263 pages = 2.1 MB I/O
              -> Bitmap Index Scan on idx_node_props_gin
                   Index Cond: (properties @> '{"node_id": "SARAH_CHEN"}'::agtype)
                   Buffers: shared hit=3 read=263
  Execution Time: 5.656 ms
```

**Finding**: Even with the GIN index (not seq scan!), a single MERGE costs 5.6ms
and reads 263 disk pages (2.1 MB of I/O per upsert).

---

### 5.4 UNWIND MERGE — Does It Batch?

```sql
EXPLAIN (ANALYZE, BUFFERS)
SELECT * FROM cypher('eq_eq_default_graph', $$
  UNWIND [{node_id: 'ENT_A', entity_type: 'PERSON'},
          {node_id: 'ENT_B', entity_type: 'ORG'},
          {node_id: 'ENT_C', entity_type: 'CONCEPT'}] AS props
  MERGE (n:Node {node_id: props.node_id})
  SET n.entity_type = props.entity_type
  RETURN n
$$) AS (n agtype);
```

```
RESULT:
  Custom Scan (Cypher Merge) (actual time=1.290..2.913 rows=3 loops=1)
    -> Nested Loop Left Join (actual time=1.053..2.645 rows=3)
         -> ProjectSet (rows=3 loops=1)           ← 3 items from UNWIND
         -> Bitmap Heap Scan on "Node" (loops=3)  ← ONE SCAN PER ROW
              Buffers: shared hit=797 read=1
              -> Bitmap Index Scan on idx_node_props_gin (loops=3)
  Execution Time: 6.644 ms  (for 3 nodes)
  Per-node cost: 6.644/3 = 2.21 ms
```

**Finding**: UNWIND MERGE does N separate GIN index scans (loops=N). The
network overhead is saved (one SQL call), but the database still does N
individual lookups. **Batch size does not help GIN scan cost**.

---

## 6. Extrapolation: Cost at Scale

```
SCENARIO: Store document with N entities into graph with G=52,915 nodes

Node upserts via Cypher MERGE:
  Each MERGE = 5.6ms (GIN path, dominated by 263-page reads)
  N=200  → 1,120 ms = 1.1 seconds
  N=500  → 2,800 ms = 2.8 seconds
  N=2000 → 11,200 ms = 11.2 seconds

Edge upserts (via UNWIND MERGE):
  UNWIND 500 edges, chunk=50 → 10 calls × 50 × 2.21ms = 1,105 ms
  UNWIND 2000 edges → ~4,420 ms

Vector upserts (HNSW, 5898 existing):
  Each INSERT: HNSW update on 909 MB index = ~2-5ms
  N_chunks=50 → 50 × 3.5ms = 175ms (cached)
  N_chunks=200 → 200 × 5ms = 1,000ms (cold cache)

Total estimate for 2000-entity document:
  Node upserts:   11,200 ms
  Edge upserts:    4,420 ms
  Vector upserts:  1,000 ms
  LLM summarize:  ~5,000 ms (optional)
  ─────────────────────────
  TOTAL:          ~21,620 ms = ~21 seconds minimum
  (observed: 8-15 minutes → additional overhead from connection pool,
   context switches, serialized writes, community indexing)
```

---

## 7. Index Usage Audit Query

Run this to identify unused indexes:

```sql
SELECT 
    s.schemaname,
    s.relname AS table_name,
    s.indexrelname AS index_name,
    s.idx_scan AS times_used,
    pg_size_pretty(pg_relation_size(s.indexrelid)) AS index_size,
    'DROP INDEX CONCURRENTLY '||s.indexrelname||';' AS drop_cmd
FROM pg_stat_user_indexes s
WHERE s.schemaname = 'eq_eq_default_graph'
  AND s.idx_scan = 0
ORDER BY pg_relation_size(s.indexrelid) DESC;
```

**Expected result**: ~15 indexes with `idx_scan = 0` that can be safely dropped
after confirming they're not used by any query in the application.
