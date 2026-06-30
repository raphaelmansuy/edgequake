# SPEC-034-003: Root Cause — 5 WHY Deep Dive

> **Lens**: Engineering Investigation  
> **Version**: 1.0.0 — 2026-06-30  
> **Method**: Toyota 5 WHY — ask "why" recursively until the root cause is reached

---

## Problem Statement for 5 WHY

> *Storing a large document (200+ entities) takes several minutes when the
> knowledge graph already contains ~50,000 nodes.*

---

## WHY Chain 1 — The AGE MERGE Bottleneck

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  WHY #1                                                                     │
│  Q: Why is storing a 200-entity document slow?                              │
│  A: Because each of the 200 entity upserts takes ~5.6ms in the database.   │
│     200 × 5.6ms = 1,120ms just for node writes, before edges or vectors.   │
└─────────────────────────────────────────────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  WHY #2                                                                     │
│  Q: Why does each entity upsert take 5.6ms?                                 │
│  A: Because the Cypher MERGE does a Bitmap Index Scan on the GIN index      │
│     (idx_node_props_gin, 19 MB) instead of the btree index (0.081ms).      │
│     The GIN scan reads ~263 8KB pages = 2.1 MB of I/O per node lookup.     │
└─────────────────────────────────────────────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  WHY #3                                                                     │
│  Q: Why does Cypher use GIN instead of the btree index?                     │
│  A: Because AGE compiles the Cypher property filter {node_id: 'X'} into    │
│     the PostgreSQL expression:                                              │
│       properties @> '{"node_id": "X"}'::agtype                             │
│     This is the GIN containment operator (@>), not an equality operator.   │
│     The btree index is on:                                                  │
│       (agtype_to_json(properties)->>'node_id')                              │
│     These two expressions are NOT equivalent in PostgreSQL's eyes —        │
│     the planner cannot use the btree index for a @> query.                  │
└─────────────────────────────────────────────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  WHY #4                                                                     │
│  Q: Why did the developers add a btree index if Cypher never uses it?       │
│  A: The btree index WAS added as an optimization attempt (SPEC-032 W-01).  │
│     It works perfectly for native SQL queries on the Node table.            │
│     However, all writes go through the AGE cypher() UDF wrapper — there    │
│     is no native SQL write path for AGE graph data currently.               │
│     Reads from edgequake's search_nodes, get_nodes_by_ids etc. use         │
│     native SQL — those are fast. MERGE/upsert still uses Cypher.           │
└─────────────────────────────────────────────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  WHY #5 — ROOT CAUSE                                                        │
│  Q: Why is there no native SQL write path for AGE graph upserts?           │
│  A: Because Apache AGE's graph internal format uses graphid (int64) as      │
│     the primary key and stores data in per-label PostgreSQL tables.         │
│     Writing directly requires:                                              │
│       (a) knowing the correct schema.table for each label                   │
│       (b) generating valid graphid values                                   │
│       (c) maintaining AGE's internal vertex/edge pkey tables                │
│     This was deemed unsafe/complex, so all writes use cypher() instead.    │
│                                                                             │
│  ROOT CAUSE RC-1: The write path is locked behind AGE's cypher() UDF       │
│  which compiles property matching to GIN @> — incompatible with the        │
│  btree indexes that would make lookups 69× faster.                         │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## WHY Chain 2 — HNSW Index Bloat

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  WHY #1                                                                     │
│  Q: Why is the pgvector table slow to insert into?                          │
│  A: Because every INSERT triggers maintenance of a 909 MB HNSW index       │
│     (eq_eq_default_ws_00000000_vectors_embedding_idx).                     │
│     The HNSW graph update is O(M · log N) — ~16 × log(5898) ≈ 198 links  │
│     must be updated or rebalanced per insert.                               │
└─────────────────────────────────────────────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  WHY #2                                                                     │
│  Q: Why is a 909 MB HNSW index needed for only 5,898 vectors?              │
│  A: The vectors are 1024-dimensional. Each vector = 4 KB (1024 × 4 bytes). │
│     5,898 vectors = 23.4 MB raw.                                            │
│     HNSW with m=16, ef_construction=64 stores M*2=32 links per node.       │
│     At 1024 dims, each link record is ~8 bytes (graphid).                  │
│     5898 × 32 = 188,736 link records + multi-level structure = ~909 MB.   │
│     (This is within pgvector's expected behaviour for high dimensions)     │
└─────────────────────────────────────────────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  WHY #3                                                                     │
│  Q: Why use HNSW during bulk loading instead of building it after?         │
│  A: Because the migration that created the HNSW index (028_add_vector_     │
│     materialized_columns.sql or 029) ran before any data existed.           │
│     PostgreSQL created the index immediately on an empty table.             │
│     Subsequent INSERTs incrementally maintain the online HNSW structure.  │
│     There is no "bulk load then index" pattern in the current pipeline.    │
└─────────────────────────────────────────────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  WHY #4                                                                     │
│  Q: Why not use IVFFlat which is cheaper to maintain during inserts?       │
│  A: IVFFlat requires training data (centroids) before it can be used for   │
│     queries. It's not suitable for incrementally growing tables starting   │
│     from 0. HNSW is the recommended approach for online indexing in        │
│     pgvector. The choice was architecturally correct.                       │
└─────────────────────────────────────────────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  WHY #5 — ROOT CAUSE                                                        │
│  Q: Why is the HNSW maintenance cost disproportionate?                     │
│  A: The current pipeline writes vectors one-by-one per entity per chunk.   │
│     For a 200-entity document with 50 chunks = 250 individual INSERT       │
│     operations, each maintaining the 909 MB HNSW index.                    │
│     If all 250 were written as a single COPY/batch then the HNSW update   │
│     would be amortized differently (still O(M·log N) per row, but the     │
│     connection overhead is eliminated and OS buffer cache helps).           │
│                                                                             │
│  ROOT CAUSE RC-2: Incremental online HNSW maintenance on high-dimensional  │
│  vectors is fundamentally expensive; the batch insert size (1 per row) is │
│  suboptimal vs. the maximum supported by pgvector's upsert.                │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## WHY Chain 3 — Index Proliferation

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  WHY #1                                                                     │
│  Q: Why are there 18+ indexes on the AGE Node label table?                 │
│  A: Each migration that addressed a performance issue added new indexes    │
│     without removing the old ones. Migrations 013, 014, 028, 029, 036,    │
│     038 all added indexes to graph tables.                                  │
└─────────────────────────────────────────────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  WHY #2                                                                     │
│  Q: Why were old indexes not removed when new ones were added?             │
│  A: DROP INDEX CONCURRENTLY requires a migration file, and removing an     │
│     index that a query might depend on is risky without profiling.         │
│     The conservative approach was to add, not replace.                      │
└─────────────────────────────────────────────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  WHY #3                                                                     │
│  Q: What is the concrete cost of 18 indexes vs. 3 essential ones?         │
│  A: For every INSERT/UPDATE/DELETE on the Node table:                       │
│       18 indexes × O(log N) updates per index                              │
│     vs. 3 indexes × O(log N) updates per index                             │
│     → 6× write amplification just from index proliferation.                │
└─────────────────────────────────────────────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  WHY #4                                                                     │
│  Q: Are all 18 indexes used by queries?                                    │
│  A: Evidence from EXPLAIN ANALYZE shows:                                    │
│     - Cypher MERGE uses: idx_node_props_gin (GIN containment)              │
│     - Native SQL uses:   idx_node_prop_node_id_btree                       │
│     - _ag_label_vertex indexes: 8192 bytes each = NEVER USED (empty)      │
│     - idx_node_prop_node_id (agtype_access_operator): NOT used by Cypher  │
│     - idx_node_id (internal graphid): only for AGE internal operations    │
└─────────────────────────────────────────────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  WHY #5 — ROOT CAUSE                                                        │
│  Q: Why can't we know in advance which indexes are safe to drop?           │
│  A: There is no pg_stat_user_indexes audit pass in the codebase.           │
│     Without measuring index_scan count from pg_stat_user_indexes, it's    │
│     impossible to know which indexes have 0 scans vs. 1M scans.            │
│                                                                             │
│  ROOT CAUSE RC-5: Absence of index usage audit + no index retirement       │
│  process has led to 6× write amplification from redundant indexes.         │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## WHY Chain 4 — KV GIN Bloat

```
WHY #1: KV upserts are slow for large chunk texts
WHY #2: Each upsert maintains a 112 MB GIN index on the value column
WHY #3: GIN on JSONB indexes every word/path in 61 KB text blobs
WHY #4: The GIN was added for "searchable KV values" but KV is only looked up by key
WHY #5 ROOT CAUSE: GIN index on KV value is dead weight — key is the only access path
```

---

## WHY Chain 5 — UNWIND Doesn't Really Batch

```
WHY #1: Batch UNWIND MERGE is slower than expected for N entities
WHY #2: EXPLAIN shows N separate Bitmap Index Scans (loops=N) inside one Cypher call
WHY #3: AGE compiles UNWIND+MERGE to a Nested Loop + per-row MERGE subplan
WHY #4: Each row needs a separate graph-pattern match before the MERGE decision
WHY #5 ROOT CAUSE: AGE has no true set-based MERGE — UNWIND reduces network
         overhead but NOT the O(N) GIN scans inside the database engine
```

---

## Root Cause Summary Table

| ID   | Root Cause                                         | Impact       | Difficulty |
| ---- | -------------------------------------------------- | ------------ | ---------- |
| RC-1 | AGE Cypher MERGE → GIN O(G) not btree O(log G)     | **CRITICAL** | Medium     |
| RC-2 | HNSW online maintenance at 1024 dims — no batching | **HIGH**     | Low        |
| RC-3 | UNWIND MERGE still N GIN scans per batch           | **HIGH**     | Hard       |
| RC-4 | KV GIN index on non-queryable 61 KB texts          | **MEDIUM**   | Low        |
| RC-5 | 18 redundant indexes — 6× write amplification      | **MEDIUM**   | Low        |
| RC-6 | Community index refresh on every persist call      | **LOW**      | Low        |
