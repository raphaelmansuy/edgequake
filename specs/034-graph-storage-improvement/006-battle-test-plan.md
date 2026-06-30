# SPEC-034-006: Battle-Test Improvement Plan

> **Lens**: Full Stack Engineer / System Engineer  
> **Version**: 3.0.0 — 2026-06-30 (honest assessment)  
> **Methodology**: Measure → Hypothesis → Implement → Validate → Ship

---

## Honest Assessment — 2026-06-30

### ✅ What is TRUE (verified by code + database)

**Database** — 10 migrations applied, checksums locked in `_sqlx_migrations`:

| Migration | Description                                                         | Verified |
| --------- | ------------------------------------------------------------------- | -------- |
| 067       | native_graph_write_helpers (initial — bugs fixed by 075+076)        | ✅        |
| 068       | drop_kv_gin_value_index — 0 KV GIN rows in pg_indexes               | ✅        |
| 069       | drop_duplicate_fts_index — 0 duplicate FTS rows                     | ✅        |
| 070       | consolidate_age_indexes — Node: 17+ indexes → 5 (measured)          | ✅        |
| 071       | hnsw_optimize — ef_construction='32' confirmed in indexdef          | ✅        |
| 072       | edge_text_cast_indexes — 2 text-cast indexes on EDGE                | ✅        |
| 073       | drop_vector_metadata_gin — metadata GIN gone                        | ✅        |
| 074       | native_upsert_unique_indexes — UNIQUE indexes; 49+118 dupes removed | ✅        |
| 075       | fix_native_graph_write_helpers — g.graphid join + label_id<<48      | ✅        |
| 076       | fix_graphid_cast — ::text::graphid cast                             | ✅        |

**Rust code** (working directory, NOT committed):

| File                     | Change                                     | Verified |
| ------------------------ | ------------------------------------------ | -------- |
| `ingestion_persister.rs` | IMP-06: tokio::spawn for community refresh | ✅        |
| `edges_ops.rs`           | IMP-01: pg_upsert_edges_batch_native()     | ✅        |
| `mod.rs`                 | IMP-01: native_graph_writes_enabled() flag | ✅        |
| `nodes_ops.rs`           | IMP-01: pg_upsert_nodes_batch_native()     | ✅        |

**E2E evidence** — 34/34 tests pass (`e2e/evidence/`):
- Sprint 1+2 improvements verified against live DB
- Sprint 3: ON CONFLICT node upsert verified (insert v1 → update v2, count=1)
- Idempotency: second migrate run = no output

---

### ❌ What is NOT done / Gaps / Risks

**Critical:**
- `❌ NOTHING IS COMMITTED` — 10 migration files (`??` untracked) and 4 Rust files (`M` modified). A `git checkout` would destroy all SPEC-034 work.
- `⚠ IMP-01 IS DISABLED` — `EDGEQUAKE_NATIVE_GRAPH_WRITES=1` is never set. The native path is dead code in production. Flip the flag only after integration testing.

**Data integrity:**
- `⚠ DUPLICATES FOUND` — Migration 074 silently removed 49 duplicate Node groups and 118 duplicate EDGE pairs from the live database. Root cause uninvestigated — may indicate a race condition in Cypher MERGE.

**Performance claims:**
- `⚠ NO BENCHMARK` — The 69× speedup is from EXPLAIN ANALYZE estimates, not from wall-clock ingestion timing. No before/after measurement taken.

**Migration quality:**
- `⚠ FIX-ON-FIX CHAIN` — Migrations 067→075→076 each fix the same 4 helper functions. Correct but messy — each migration is permanent in `_sqlx_migrations` history.

**Pre-existing issues (not from SPEC-034):**
- `⚠ 2 failing tests` — `test_tenant_scoping` and `local_db_with_auth_off_only_warns` fail on every run in `edgequake-api`. Unrelated to SPEC-034.

---

## Next Actions Required

- [ ] **COMMIT** — `git add` all 10 migration files + 4 Rust files + spec directory
- [ ] **INVESTIGATE** — root cause of 49 Node + 118 EDGE duplicates removed by M074
- [ ] **ENABLE** — set `EDGEQUAKE_NATIVE_GRAPH_WRITES=1` in staging first, measure, then production
- [ ] **BENCHMARK** — measure actual document ingestion time before/after the flag
- [ ] **FIX** — pre-existing `test_tenant_scoping` failure in edgequake-api

---

## Implementation Status (All Sprints Complete)

```
SPRINT 1 ✅  IMP-03 Drop KV GIN (068)  IMP-05 Drop dup FTS (069)
             IMP-06 Async community     IMP-08 Drop metadata GIN (073)

SPRINT 2 ✅  IMP-07 Edge text indexes (072)  IMP-02 Index consolidation (070)
             IMP-04 HNSW ef=32 (071)

SPRINT 3 ✅  IMP-01 Native SQL write (067+074+075+076 + Rust)
             → DISABLED in prod until flag is set
```

---

## Key Technical Discoveries (not in spec — found during impl)

```
AGE graphid encoding: (label_id << 48) | seq_val   NOT (label_id << 32)
ag_graph PK column:   g.graphid                     NOT g.oid
bigint→graphid:       no direct cast; use (n::text)::ag_catalog.graphid
jsonb→agtype:         no registered cast; use text::ag_catalog.agtype
CONCURRENTLY:         forbidden inside DO $$ (sqlx transactions); use regular DDL
Duplicate data:       49 Node groups + 118 EDGE pairs in live production data
```

---

## Overview


│    068 drop_kv_gin_value_index                                               │
│    069 drop_duplicate_fts_index                                              │
│    070 consolidate_age_indexes                                               │
│    071 hnsw_optimize                                                         │
│    072 edge_text_cast_indexes                                                │
│    073 drop_vector_metadata_gin                                              │
│    074 native_upsert_unique_indexes (UNIQUE indexes + dedup)                 │
│    075 fix_native_graph_write_helpers (correct join+shift)                   │
│    076 fix_graphid_cast (::text::graphid cast)                               │
│                                                                              │
│  RUST CODE:                                                                  │
│    graph/mod.rs: native_graph_writes_enabled() + 4 feature flag tests        │
│    graph/nodes_ops.rs: pg_upsert_nodes_batch_native() + dispatch             │
│    graph/edges_ops.rs: pg_upsert_edges_batch_native() + dispatch             │
│    ingestion_persister.rs: tokio::spawn for community refresh                │
│                                                                              │
│  E2E EVIDENCE: 34/34 tests pass (specs/034.../e2e/evidence/)                │
│                                                                              │
│  KEY DISCOVERIES during implementation:                                      │
│    - AGE graphid encoding: (label_id << 48) | seq_val  (NOT << 32)          │
│    - ag_graph PK is 'graphid' (NOT 'oid')                                   │
│    - No bigint→graphid cast; use (bigint::text)::graphid                    │
│    - No jsonb→agtype cast; use text::ag_catalog.agtype                      │
│    - DO $$ blocks run in sqlx transactions; CONCURRENTLY forbidden           │
│    - 49 duplicate Node groups + 118 duplicate EDGE pairs found in live data  │
└──────────────────────────────────────────────────────────────────────────────┘
```
│                                                                              │
│  FILES CHANGED:                                                              │
│    edgequake/migrations/067_native_graph_write_helpers.sql    (IMP-01)       │
│    edgequake/migrations/068_drop_kv_gin_value_index.sql       (IMP-03)       │
│    edgequake/migrations/069_drop_duplicate_fts_index.sql      (IMP-05)       │
│    edgequake/migrations/070_consolidate_age_indexes.sql       (IMP-02)       │
│    edgequake/migrations/071_hnsw_optimize.sql                 (IMP-04)       │
│    edgequake/migrations/072_edge_text_cast_indexes.sql        (IMP-07)       │
│    edgequake/migrations/073_drop_vector_metadata_gin.sql      (IMP-08)       │
│    crates/edgequake-storage/.../graph/mod.rs       (IMP-01 feature flag)     │
│    crates/edgequake-storage/.../graph/nodes_ops.rs (IMP-01 native upsert)    │
│    crates/edgequake-storage/.../graph/edges_ops.rs (IMP-01 native upsert)    │
│    crates/edgequake-pipeline/.../ingestion_persister.rs (IMP-06 async)       │
│                                                                              │
│  E2E TESTS:                                                                  │
│    specs/034-graph-storage-improvement/e2e/ (9 test files + runner)          │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Migration Rollout Checklist

- [x] Migration 067 — Native graph write helper functions (eq_next_node_id, etc.)
- [x] Migration 068 — Drop KV GIN value index (112 MB → 0)
- [x] Migration 069 — Drop duplicate FTS index (1.9 MB × N workspaces → 0)
- [x] Migration 070 — AGE index consolidation (17+ → 5-6 per label)
- [x] Migration 071 — HNSW parameter optimization (ef_construction=64 → 32)
- [x] Migration 072 — Edge text-cast expression indexes (fix BFS seq scan)
- [x] Migration 073 — Drop vector metadata GIN index (13 MB → 0)

---

## Overview

This plan is structured as **independent improvement tracks** (IMP-01 to IMP-08),
each of which can be implemented, tested, and rolled back independently.

Each improvement has:
1. A **hypothesis** to test
2. A **benchmark** to run before and after
3. An **acceptance criterion**
4. A **rollback plan**

```
┌──────────────────────────────────────────────────────────────────────────────┐
│  IMPROVEMENT TRACKS                                                          │
│                                                                              │
│  IMP-01 ─ Native SQL Node/Edge Upsert Path (CRITICAL — 69× speedup)        │
│  IMP-02 ─ Index Consolidation (remove 15 redundant indexes)                 │
│  IMP-03 ─ Drop KV GIN Index (155× index/data ratio → 1×)                  │
│  IMP-04 ─ HNSW Batch Optimization (reduce insert overhead)                  │
│  IMP-05 ─ Content-TSV Deduplication (remove duplicate FTS index)            │
│  IMP-06 ─ Async Community Indexing (decouple from store hot path)           │
│  IMP-07 ─ Edge Text-Cast Expression Indexes (fix BFS seq scan — 40×)       │
│  IMP-08 ─ Drop Vector Metadata GIN (13 MB freed, 0 scans)                  │
│                                                                              │
│  DEPENDENCIES:                                                               │
│  IMP-02 depends on IMP-01 (some indexes only needed for Cypher path)       │
│  IMP-06 is independent                                                       │
│  All others are independent                                                  │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## IMP-01: Native SQL Node/Edge Upsert Path

### Status: ✅ IMPLEMENTED

### Problem
AGE Cypher MERGE does O(G) GIN containment scan per node.  
Native SQL with btree index does O(log G) lookup — **69–153× faster**.

### Hypothesis
By writing AGE nodes and edges via native SQL (`INSERT ... ON CONFLICT DO UPDATE`
on the `"Node"` label table, using the btree index on `agtype_to_json(properties)->>'node_id'`),
we can reduce per-node write cost from 5.6ms to ~0.1ms.

### Architecture

```
CURRENT WRITE PATH:
  Rust code
    → UNWIND [...] MERGE (n:Node {node_id: ...})
    → AGE cypher() UDF
    → AGE compiles to: Bitmap Index Scan on GIN
    → O(G) lookups
    
TARGET WRITE PATH:
  Rust code
    → INSERT INTO "graph"."Node" (id, properties)
      SELECT graphid_gen(), jsonb_build_object(...) FROM unnest($ids,$props)
      ON CONFLICT ((agtype_to_json(properties)->>'node_id'))
      DO UPDATE SET properties = EXCLUDED.properties
    → PostgreSQL btree index scan
    → O(log G) lookup
```

### Key Challenge: AGE graphid generation

AGE stores graphids as 64-bit integers where the high 32 bits encode the
label OID and low 32 bits are the sequence. We need to generate valid graphids
for new nodes without going through `cypher()`.

```sql
-- AGE graphid generation pattern
-- Label OID: SELECT oid FROM ag_catalog.ag_label WHERE name = 'Node' AND graph = (graph oid)
-- Sequence: SELECT nextval('"graph"."Node_id_seq"')  
-- graphid = label_oid << 32 | sequence_val
CREATE OR REPLACE FUNCTION eq_next_node_graphid(graph_name text) 
RETURNS ag_catalog.graphid AS $$
DECLARE
  label_oid bigint;
  seq_val bigint;
BEGIN
  SELECT oid::bigint INTO label_oid 
  FROM ag_catalog.ag_label 
  WHERE name = 'Node' 
    AND graph = (SELECT oid FROM ag_catalog.ag_graph WHERE name = graph_name);
  
  SELECT nextval(format('%I.%I', graph_name, '"Node_id_seq"')) INTO seq_val;
  
  RETURN (label_oid << 32 | seq_val)::ag_catalog.graphid;
END;
$$ LANGUAGE plpgsql;
```

### Implementation Steps

```
Step 1: Add helper SQL function eq_next_node_graphid (migration 067)
Step 2: Add unique constraint/index (btree) on node_id expression for ON CONFLICT
Step 3: Implement pg_upsert_nodes_batch_native() in nodes_ops.rs
Step 4: Implement pg_upsert_edges_batch_native() in edges_ops.rs  
Step 5: Feature flag: EDGEQUAKE_NATIVE_GRAPH_WRITES=1 to enable
Step 6: A/B benchmark, then make native path the default
Step 7: Remove old Cypher upsert path (separate migration)
```

### Benchmark

```bash
# Before:
# Time storing a 200-entity test document with graph at 50K nodes
cargo test -p edgequake-pipeline --test perf_graph_write -- --nocapture

# After (with native SQL path):
EDGEQUAKE_NATIVE_GRAPH_WRITES=1 \
cargo test -p edgequake-pipeline --test perf_graph_write -- --nocapture
```

### Acceptance Criteria
- `pg_upsert_nodes_batch_native` with 200 nodes completes in < 500ms
- Query plans for all existing reads continue to use correct indexes  
- All existing unit tests pass
- `cargo test --workspace --lib` passes

### Rollback
Set `EDGEQUAKE_NATIVE_GRAPH_WRITES=0` (env var feature flag).  
The Cypher path is preserved until the flag is removed.

---

## IMP-02: Index Consolidation

### Status: ✅ IMPLEMENTED (Migration 070)

### Problem
17+ indexes on the `Node` label table; 9+ redundant/unused.
Each INSERT must maintain all of them → 6× write amplification.

### Indexes to DROP (confirmed unused via pg_stat_user_indexes)

```sql
-- All _ag_label_vertex indexes (0 bytes = empty, never populated by our Cypher path)
DROP INDEX CONCURRENTLY IF EXISTS eq_eq_default_graph.idx_eq_eq_default_graph_node_id;
DROP INDEX CONCURRENTLY IF EXISTS eq_eq_default_graph.idx_eq_eq_default_graph_tenant_id;
DROP INDEX CONCURRENTLY IF EXISTS eq_eq_default_graph.idx_eq_eq_default_graph_workspace_id;
DROP INDEX CONCURRENTLY IF EXISTS eq_eq_default_graph.idx_eq_eq_default_graph_tenant_workspace;
DROP INDEX CONCURRENTLY IF EXISTS eq_eq_default_graph.idx_eq_eq_default_graph_entity_type;
DROP INDEX CONCURRENTLY IF EXISTS eq_eq_default_graph.idx_eq_eq_default_graph_vertex_source_id;
DROP INDEX CONCURRENTLY IF EXISTS eq_eq_default_graph.idx_eq_eq_default_graph_vertex_source_ids_gin;
DROP INDEX CONCURRENTLY IF EXISTS eq_eq_default_graph.idx_ag_vertex_props_gin;
DROP INDEX CONCURRENTLY IF EXISTS eq_eq_default_graph.idx_ag_vertex_tenant_id;
DROP INDEX CONCURRENTLY IF EXISTS eq_eq_default_graph.idx_ag_vertex_workspace_id;

-- Duplicate node_id btree (agtype_access_operator is superseded by json->>'node_id')
DROP INDEX CONCURRENTLY IF EXISTS eq_eq_default_graph.idx_node_prop_node_id;
```

### Indexes to KEEP (essential)

```
idx_node_props_gin        — used by Cypher MERGE (until IMP-01 is complete)
idx_node_prop_node_id_btree — used by native SQL reads (search_nodes, etc.)
idx_node_id               — AGE internal (required for relationship traversal)
idx_node_tenant_id        — used by tenant isolation in native SQL reads
idx_node_workspace_id     — used by workspace isolation
```

### Benchmark

```sql
-- Measure write time before and after
\timing on
INSERT INTO eq_eq_default_graph."Node" (id, properties) 
VALUES (eq_next_node_graphid('eq_eq_default_graph'), 
        '{"node_id": "BENCH_TEST_1", "entity_type": "PERSON"}'::jsonb::agtype);
```

### Acceptance Criteria
- Index count on Node: 17+ → 5-6
- Index count on EDGE: 9+ → 4-5
- No change in query plan for any read operation (verify with EXPLAIN)
- `pg_stat_user_indexes.idx_scan` for kept indexes stays > 0 after a test run

### Rollback
Re-add removed indexes with `CREATE INDEX CONCURRENTLY`.  
Safe because data was never deleted — only indexes.

---

## IMP-03: Drop KV GIN Index on Value

### Status: ✅ IMPLEMENTED (Migration 068)

### Problem
`eq_eq_default_kv_value_gin`: 112 MB GIN index on JSONB values.  
KV values are 61 KB chunk text blobs — they are NEVER queried by content.  
Every KV upsert maintains a GIN on 61 KB of text for no benefit.

### Migration

```sql
-- Migration 068: Drop KV GIN value index
DO $$
DECLARE
  kv_table_pattern text;
BEGIN
  FOR kv_table_pattern IN
    SELECT tablename FROM pg_tables 
    WHERE tablename LIKE 'eq_%_kv' AND schemaname = 'public'
  LOOP
    EXECUTE format('DROP INDEX CONCURRENTLY IF EXISTS %I', 
                   kv_table_pattern || '_value_gin');
    RAISE NOTICE 'Dropped value GIN on %', kv_table_pattern;
  END LOOP;
END $$;
```

### Acceptance Criteria
- KV table total size: 256 MB → ~1 MB (760 kB heap + 192 kB pkey + 168 kB rev_key)
- KV upsert time: < 1ms per record
- No KV query in the codebase uses GIN content search (verified by grep)

### Verification Query

```bash
# Verify no code queries KV by value content
grep -r "kv.*value.*GIN\|kv.*CONTAINS\|kv.*@>" \
  edgequake/crates/ edgequake_webui/
# Expected: zero matches
```

### Rollback
`CREATE INDEX CONCURRENTLY eq_eq_default_kv_value_gin ON eq_eq_default_kv USING gin(value);`

---

## IMP-04: HNSW Batch Optimization

### Status: ✅ IMPLEMENTED (Migration 071 — CONCURRENTLY, zero downtime)

### Problem
HNSW online maintenance at dim=1024 costs ~3.5ms per vector insert.
For a 200-chunk document: 50 inserts × 3.5ms = 175ms (acceptable).
For a 2000-entity document: 500 inserts × 3.5ms = 1,750ms.

### Approach A: Reduce ef_construction (Quick Win)

```sql
-- Current: ef_construction=64 (high quality, slow build)
-- Target:  ef_construction=32 (still excellent quality, ~2× faster build)
-- NOTE: Must reindex — cannot ALTER HNSW parameters on existing index

-- Step 1: Create new index with lower ef_construction
CREATE INDEX CONCURRENTLY eq_ws_vectors_embedding_idx_new 
ON eq_eq_default_ws_00000000_vectors 
USING hnsw (embedding vector_cosine_ops) 
WITH (m='16', ef_construction='32');

-- Step 2: Drop old index (in maintenance window)
DROP INDEX CONCURRENTLY eq_eq_default_ws_00000000_vectors_embedding_idx;

-- Step 3: Rename new index  
ALTER INDEX eq_ws_vectors_embedding_idx_new 
RENAME TO eq_eq_default_ws_00000000_vectors_embedding_idx;
```

### Approach B: Deferred Index Build (Larger Win, More Complex)

```
1. Drop HNSW index at start of large document ingestion
2. Write all vectors to table (fast, no index)
3. Rebuild HNSW after all vectors written (amortized cost)
4. Signal is: N_new_vectors > BATCH_THRESHOLD (e.g. 100)

Cost:
  Build HNSW on V+N vectors once: O((V+N) × M × log(V+N))
  vs. N incremental inserts: O(N × M × log(V))
  
Savings: when N is large, one-shot build is ~2× faster due to better
         cache locality and sequential access patterns.
```

### Acceptance Criteria
- HNSW index size: 909 MB → ~600 MB (ef_construction=32 vs 64)
- Query recall@10 within 5% of baseline (measure with held-out test set)
- Vector insert time: 3.5ms → ~2ms average

---

## IMP-05: Duplicate FTS Index Removal

### Status: ✅ IMPLEMENTED (Migration 069)

### Finding

Two identical GIN indexes on `content_tsv` exist on the vectors table:
```
eq_eq_default_ws_00000000_vectors_content_tsv_idx  (1.9 MB)
idx_eq_eq_default_ws_00000000_vectors_content_tsv  (1.9 MB)
```

### Fix

```sql
-- Migration 069: Drop duplicate FTS index on vectors
DO $$
DECLARE 
  tbl text;
BEGIN
  FOR tbl IN 
    SELECT tablename FROM pg_tables 
    WHERE tablename LIKE 'eq_%_vectors' AND schemaname = 'public'
  LOOP
    EXECUTE format(
      'DROP INDEX CONCURRENTLY IF EXISTS idx_%s_content_tsv', 
      replace(tbl, '.', '_')
    );
  END LOOP;
END $$;
```

### Impact
- Frees 1.9 MB × N_workspaces of index space
- Reduces vector insert write amplification by 1 index

---

## IMP-06: Async Community Indexing

### Status: ✅ IMPLEMENTED (ingestion_persister.rs — tokio::spawn)

### Problem

After every document persist, `schedule_community_index_refresh()` is called
synchronously. This blocks the persist path and causes latency spikes.

### Current Code

```rust
// ingestion_persister.rs
edgequake_storage::schedule_community_index_refresh(
    graph_storage.clone(),
    ctx.workspace_id.clone(),
).await;
```

### Fix

```rust
// Move to background task — fire and forget
tokio::spawn(async move {
    edgequake_storage::schedule_community_index_refresh(
        graph_storage_clone,
        workspace_id_clone,
    ).await;
});
```

### Acceptance Criteria
- Persist path no longer waits for community index refresh
- Community index is still refreshed within 30 seconds of persist completing
- No data loss on server restart (community index is a read model, rebuilt from graph)

## IMP-07: Edge Text-Cast Expression Indexes (NEW — from Code Audit)

### Status: ✅ IMPLEMENTED (Migration 072)

### Problem

`pg_get_incident_edges_batch` (used in EVERY BFS traversal) does a full
sequential scan of 67,636 edges due to a type cast mismatch:

```sql
-- Current join (can't use idx_edge_start_id which is on graphid, not text):
JOIN _ag_label_edge e ON e.start_id::text = sv.id::text
```

EXPLAIN confirms: `Seq Scan on "EDGE" (rows=67636)` for every BFS call.

### Hypothesis

Adding expression indexes `ON "EDGE" ((start_id::text))` and
`ON "EDGE" ((end_id::text))` enables PostgreSQL to use Nested Loop +
Index Scan instead of Hash Join + Seq Scan for the incident edge lookup.

### Migration

```sql
-- Migration 072: Add text-cast expression indexes on EDGE start/end IDs
-- WHY: pg_get_incident_edges_batch joins on e.start_id::text = sv.id::text
--      The existing idx_edge_start_id is on raw graphid, cannot be used for
--      the text cast comparison. New expression indexes fix this.
DO $$
DECLARE g_name text;
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age') THEN
    RAISE NOTICE 'AGE not installed — skipping'; RETURN;
  END IF;
  FOR g_name IN SELECT name FROM ag_catalog.ag_graph LOOP
    EXECUTE format(
      'CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_%s_edge_start_id_text '
      'ON %I."EDGE" ((start_id::text))',
      replace(g_name,'.','_'), g_name
    );
    EXECUTE format(
      'CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_%s_edge_end_id_text '
      'ON %I."EDGE" ((end_id::text))',
      replace(g_name,'.','_'), g_name
    );
    RAISE NOTICE 'Created text-cast edge indexes for graph: %', g_name;
  END LOOP;
END $$;
```

### Acceptance Criteria
- EXPLAIN for `pg_get_incident_edges_batch` shows Nested Loop/Index Scan (NOT Seq Scan)
- BFS knowledge graph traversal for a 3-hop query: < 500ms at 67K edges
- No change to query results (pure index optimization)

### Expected Impact
```
Current: Hash Join + Seq Scan of 67,636 edges = ~200ms per BFS level
Target:  Nested Loop + Index Scan (~100 rows) = ~5ms per BFS level
Speedup: ~40×
```

---

## IMP-08: Drop Metadata GIN on Vectors Table

### Status: ✅ IMPLEMENTED (Migration 073)

### Problem
`eq_eq_default_ws_00000000_vectors_metadata_idx`: 13 MB GIN on JSONB metadata.  
**0 scans**. No code path does containment search on vector metadata.

Code audit confirms: all metadata queries use `metadata->>'key' = value`
(equality on extracted text) which benefits from btree indexes (`doc_id_idx`,
`tenant_ws_idx`) — not GIN containment.

### Migration

```sql
-- Migration 073: Drop vector metadata GIN index
DO $$
DECLARE tbl text;
BEGIN
  FOR tbl IN SELECT tablename FROM pg_tables
             WHERE tablename LIKE 'eq_%_vectors' AND tablename NOT LIKE '%_stats'
             AND schemaname='public'
  LOOP
    EXECUTE format('DROP INDEX CONCURRENTLY IF EXISTS public.%I',
                   tbl||'_metadata_idx');
    RAISE NOTICE 'Dropped metadata GIN on %', tbl;
  END LOOP;
END $$;
```

### Acceptance Criteria
- All existing metadata queries still use `doc_id_idx` or `tenant_ws_idx`
- 13 MB freed per workspace vectors table
- No scan count regression on kept indexes

---

## Updated Implementation Priority

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  PRIORITY ORDER (by impact × difficulty ratio)  — UPDATED WITH CODE AUDIT  │
│                                                                             │
│  SPRINT 1 (Quick wins — 1 day each):                                       │
│    ✅ IMP-03: Drop KV GIN index         (112 MB freed)                     │
│    ✅ IMP-05: Drop duplicate FTS index  (1.9 MB freed)                     │
│    ✅ IMP-06: Async community indexing  (decouple store hot path)           │
│    ✅ IMP-08: Drop vector metadata GIN  (13 MB freed, 0 scans confirmed)   │
│                                                                             │
│  SPRINT 2 (Medium effort — 2-3 days):                                      │
│    🔧 IMP-07: Edge text-cast indexes    (40× BFS speedup)                  │
│    🔧 IMP-02: Index consolidation       (drop 27+ redundant indexes)       │
│    🔧 IMP-04: HNSW ef_construction=32   (requires CONCURRENTLY reindex)    │
│                                                                             │
│  SPRINT 3 (High impact, high effort — 1-2 weeks):                          │
│    🚀 IMP-01: Native SQL write path     (69× write speedup target)         │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

### Scenario 1: Baseline Regression

```bash
# Measure current baseline (before any changes)
cargo test -p edgequake-pipeline --test integration -- storage_perf --nocapture
```

Record: time, EXPLAIN ANALYZE for 3 key queries, index sizes.

### Scenario 2: Concurrent Writers

```bash
# Simulate 5 concurrent document uploads
for i in 1 2 3 4 5; do
  cargo run --example ingest_test -- --entities 200 &
done
wait
# Verify: no deadlocks, no duplicate nodes, correct edge count
```

### Scenario 3: Growing Graph

```bash
# Add 10,000 nodes to the graph, then measure store time for a new document
cargo run --example generate_graph_size -- --nodes 10000
# Then store a 200-entity document
cargo test -p edgequake-pipeline --test integration -- storage_perf_large_graph
```

### Scenario 4: Deduplication Correctness

After IMP-01 (native SQL path), verify entity deduplication still works:

```bash
# Upload same document twice
# Assert: node count increases by 0 (all entities deduplicated)
# Assert: source_chunk_ids accumulates (both uploads referenced)
cargo test -p edgequake-pipeline --test integration -- dedup_correctness
```

### Scenario 5: Index Drop Safety

After IMP-02 (index consolidation):

```bash
# Run full test suite
cargo test --workspace --lib
# Run EXPLAIN on all critical queries and verify no Seq Scans appear
cargo test -p edgequake-storage --test index_coverage
```

---

## Implementation Priority

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  PRIORITY ORDER (by impact × difficulty ratio)                              │
│                                                                              │
│  SPRINT 1 (Quick wins — 1-2 days each):                                    │
│    ✅ IMP-03: Drop KV GIN index       (155× index bloat → 1×)              │
│    ✅ IMP-05: Drop duplicate FTS index (2 → 1 vector FTS indexes)          │
│    ✅ IMP-06: Async community indexing (decouple from hot path)             │
│                                                                              │
│  SPRINT 2 (Medium effort — 3-5 days):                                      │
│    🔧 IMP-02: Index consolidation     (after audit with pg_stat_indexes)   │
│    🔧 IMP-04: HNSW ef_construction    (requires reindex)                   │
│                                                                              │
│  SPRINT 3 (High impact, high effort — 1-2 weeks):                          │
│    🚀 IMP-01: Native SQL write path   (CRITICAL — 69× speedup target)      │
└─────────────────────────────────────────────────────────────────────────────┘
```
