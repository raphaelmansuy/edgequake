# SPEC-032-002: Performance Analysis — O(N) Bottlenecks

**Parent:** [SPEC-032](000-index.md)  
**Cross-refs:** `F-01..F-10` from [001](001-current-architecture.md) · `P-G4` · AGE 1.6.0

---

## 1. Expert Lenses: O(N) Analysis

### 1.1 Lens: O(N) Expert — UNWIND Literal Body Size (F-01)

**Code path:**  
`edgequake-storage/src/adapters/postgres/graph/nodes_ops.rs`  
`pg_upsert_nodes_batch()` · `pg_upsert_edges_batch()`

```
For a document with E entities, each with P properties:

  Cypher literal body = Σ properties_to_cypher(entity)
                      ≈ E × P × avg_value_len

  With E=500, P=8, avg_value_len=50 chars  → ~200 KB per UNWIND query
  With E=500, P=12, avg_value_len=200 chars → ~1.2 MB per UNWIND query

The AGE SQL wrapper:
  SELECT * FROM cypher('edgequake_graph', $$ UNWIND [...] AS props
                                              MERGE (n:Node {node_id: props.node_id})
                                              SET n.key = props.key ... $$) AS (...)

PostgreSQL must:
  1. Parse the entire Cypher literal as a string constant
  2. Pass it through AGE's Cypher planner
  3. Expand UNWIND into row-level MERGE operations

AGE's UNWIND MERGE complexity: O(E × index_lookup_cost)
At 100K existing nodes, MERGE (n:Node {node_id: x}) does:
  → b-tree lookup on _ag_label_vertex (agtype property scan)
  → AGE 1.6.0 lacks btree-on-property index (only GiST/GIN on whole agtype)
  → Degrades to O(N) table scan per MERGE at scale without property index
```

**5-Why Root Cause Chain:**

```
Symptom: GraphStorage takes 10–30 min at 100K nodes
  Why 1: UNWIND MERGE does table scan per entity
  Why 2: No btree index on Node.node_id property in AGE
  Why 3: AGE 1.6.0 indexes agtype blobs, not sub-properties
  Why 4: Property-level indexes need a functional index on agtype extraction
  Why 5: No migration adds this; code assumes index_from_node_id() is fast
Fix:    Add functional btree index on extracted node_id property (migration)
```

### 1.2 Lens: Postgres Expert — Property Index on AGE Vertex

AGE stores vertices in `_ag_label_vertex` table:

```sql
-- Current state (from schema.rs / migrations/014_add_graph_indexes.sql):
CREATE INDEX IF NOT EXISTS idx_ag_node_id
  ON _ag_label_vertex USING GIN (properties);

-- Problem: GIN on agtype is for containment (@>), NOT equality on node_id
-- A MERGE (n:Node {node_id: 'X'}) must scan all vertices matching the label,
-- then test properties -> O(N_label) per MERGE

-- Fix: Functional btree on extracted property
CREATE INDEX IF NOT EXISTS idx_ag_vertex_node_id
  ON _ag_label_vertex
  USING btree ((properties->>'node_id'));

-- Similarly for edges (source_id, target_id):
CREATE INDEX IF NOT EXISTS idx_ag_edge_source_target
  ON _ag_label_edge
  USING btree (
    (properties->>'source_id'),
    (properties->>'target_id')
  );
```

**Caveat:** AGE uses its own SQL-level function `cypher()` which may not use
these functional indexes depending on the version. Alternative: move to AGE's
`agtype_access_operator` functional indexes as described in AGE docs §Index.

### 1.3 Lens: O(N) Expert — `get_nodes_batch` Scope (F-02)

**Code path:**  
`edgequake-pipeline/src/merger/entity.rs:merge_entities_batch()`

```rust
// Current: called ONCE PER ExtractionResult in merger/mod.rs:merge()
for result in results {
    self.merge_entities_batch(entities, &mut stats).await?;
    // Each call does: get_nodes_batch(chunk_entity_keys)
    // Then: upsert_nodes_batch(updated_nodes)
}
```

```
Document has C chunks × E_avg entities/chunk:

  Round trips = C × 2  (one get_nodes_batch + one upsert_nodes_batch per chunk)

  With C=50 chunks, 2 round trips each = 100 DB round trips
  At 5ms per AGE round trip = 500ms overhead just from round-trip count

Fix: Collect ALL entities across ALL ExtractionResults first,
     then do ONE global get_nodes_batch + ONE upsert_nodes_batch.
```

**Current entity vector batch is correct** (`collect_entity_vector_batch` in
`merger/mod.rs` iterates all results before the loop) — the graph batch must
match this pattern.

### 1.4 Lens: LightRAG Expert — Merge Semantics and LLM Summarizer (F-07)

**Code path:**  
`edgequake-pipeline/src/merger/entity.rs:update_entity_node()`

```rust
// For EACH existing entity, potentially calls:
summarizer.merge_entity_descriptions(&entity.name, &descriptions).await
```

```
With 1000 entities updated (typical in a doc with 100 existing overlaps):
  → 1000 sequential LLM API calls
  → At 500ms each = 500 seconds (8+ minutes!) just from LLM summarization

This is the DOMINANT cost for the GraphStorage phase when the KG is large.

LightRAG reference implementation uses:
  - Batch summarization where possible
  - Skip summarization if descriptions are near-identical (cosine > 0.95)
  - Truncate-and-append as fallback when LLM is slow
```

**First Principle:** Merge descriptions only when they add information.  
**Fix:** Similarity gate before LLM call + async batch summarization.

### 1.5 Lens: Systems Engineer — Saga Compensation Timing (SC2)

```
persist_processing_result_impl():
  1. KV upsert         → fast, ~10ms
  2. Vector upsert     → fast, ~50ms (UNNEST bulk insert)
  3. Merger.merge()    → SLOW (10–30 min at 100K nodes)
     ├─ entity vector batch  → fast
     ├─ entity graph batch   → SLOW (UNWIND × chunks)
     ├─ rel vector batch     → fast per chunk
     └─ rel graph batch      → SLOW per chunk

If merger.merge() times out or panics:
  - SC2 compensation: delete chunk vectors for this doc_id
  - But if the partial merger completed some graph writes,
    those are NOT rolled back → orphaned graph nodes with no vectors
  - And if compensation itself fails (network) → orphaned vectors

Gap: No idempotency key / retry token for partial graph merges.
```

### 1.6 Lens: Graph Expert — AGE MERGE vs. INSERT (F-01 supplement)

AGE `MERGE (n:Label {key: val})` is semantically:

```
1. Lock label table row (ShareRowLock)
2. Scan for existing vertex matching {key: val}
3. If found: UPDATE properties (per SET clause)
4. If not found: INSERT new vertex

For 100K existing nodes with no btree on node_id:
  Step 2 = sequential scan of label table
  Lock contention: if multiple workers run concurrently, deadlocks possible
```

**AGE-specific limitation (1.6.0):** The `UNWIND [...] AS props MERGE`
pattern acquires a `ShareRowLock` on the label table for the duration
of the UNWIND expansion. With 500-row batches this is acceptable, but
slow when the table has 100K rows and no property index.

---

## 2. pgvector Performance Analysis

### 2.1 HNSW Index at Scale

```
With 100K entity vectors + 500K chunk vectors:

  HNSW index parameters (migration 028):
    m = 16, ef_construction = 64  (defaults)

  Search performance:
    ef_search = 40  (from search_tuning.rs)
    recall ≈ 95%    (acceptable)

  Index build time on INSERT:
    Each HNSW insert = O(M × ef_construction) graph updates
    = 16 × 64 = 1024 operations per vector
    At 600K vectors: ~600M operations during index build

  Recommendation: Use HNSW m=32 for graph entities (higher connectivity)
                  Use HNSW m=16 for chunks (lower connectivity, faster search)
  Or: Defer HNSW build with CONCURRENTLY during off-peak
```

### 2.2 Vector Upsert (QW2) — Already Optimized

```rust
// storage_impl.rs — QW2 UNNEST batch upsert (already correct)
INSERT INTO {table} (id, embedding, metadata, tenant_id, workspace_id)
SELECT UNNEST($1::text[]), UNNEST($2::vector[]), UNNEST($3::jsonb[]), ...
ON CONFLICT (id) DO UPDATE SET embedding = EXCLUDED.embedding, ...
```

This is O(batch_size) and correct. No change needed here.

---

## 3. Summary: Bottleneck Priority Matrix

```
┌──────────────────────────────────────────────────────────────────────┐
│ BOTTLENECK                     │ Impact │ Frequency │ Fix effort     │
├────────────────────────────────┼────────┼───────────┼────────────────┤
│ LLM summarizer per entity      │ HIGH   │ Always    │ Medium         │
│  (F-07)                        │        │           │                │
├────────────────────────────────┼────────┼───────────┼────────────────┤
│ Missing node_id btree index    │ HIGH   │ >10K nodes│ Low (migration)│
│  (F-01, AGE)                   │        │           │                │
├────────────────────────────────┼────────┼───────────┼────────────────┤
│ get_nodes_batch per chunk       │ MEDIUM │ Always    │ Medium         │
│  (F-02)                        │        │           │                │
├────────────────────────────────┼────────┼───────────┼────────────────┤
│ No progress events in merge    │ MEDIUM │ Always    │ Low            │
│  (F-03)                        │        │           │                │
├────────────────────────────────┼────────┼───────────┼────────────────┤
│ UNWIND literal body size       │ MEDIUM │ >100 prop │ Low            │
│  (F-01, literal)               │        │           │                │
├────────────────────────────────┼────────┼───────────┼────────────────┤
│ Relationship batch per chunk   │ LOW    │ Always    │ Medium         │
│  (F-09)                        │        │           │                │
└──────────────────────────────────────────────────────────────────────┘
```

---

## 4. PostgreSQL Tuning Recommendations

```sql
-- For a server dedicated to EdgeQuake with 100K+ entity graph:

-- WAL tuning (reduces checkpoint stalls during bulk merge):
ALTER SYSTEM SET wal_buffers = '256MB';
ALTER SYSTEM SET checkpoint_completion_target = 0.9;
ALTER SYSTEM SET max_wal_size = '4GB';

-- Memory tuning (needed for HNSW index scans):
ALTER SYSTEM SET work_mem = '256MB';  -- per-connection, raise carefully
ALTER SYSTEM SET maintenance_work_mem = '1GB';  -- for VACUUM/index builds

-- Parallel query (AGE does not use parallel workers in Cypher, but SQL does):
ALTER SYSTEM SET max_parallel_workers_per_gather = 4;

-- Autovacuum tuning for high-churn vectors table:
ALTER TABLE eq_default_vectors SET (
  autovacuum_vacuum_scale_factor = 0.01,  -- vacuum after 1% dead tuples
  autovacuum_analyze_scale_factor = 0.005
);
```
