# SPEC-034: Graph Storage Performance — Document Index

> **Status**: Draft — 2026-06-30  
> **Authors**: EdgeQuake Engineering (Postgres AGE · pgvector · AI · Systems)  
> **Classification**: Performance / Architecture  
> **Branch concern**: Storing a large document into the Knowledge Graph is prohibitively slow

---

## Document Map

| Document                                                             | Title                                                | Lens                           |
| -------------------------------------------------------------------- | ---------------------------------------------------- | ------------------------------ |
| [001-problem-statement.md](001-problem-statement.md)                 | Problem Statement & Observed Symptoms                | Product / User                 |
| [002-first-principles-analysis.md](002-first-principles-analysis.md) | First Principles Analysis — What Must Be True        | Systems / Theory               |
| [003-root-cause-5why.md](003-root-cause-5why.md)                     | Root Cause — 5 WHY Deep Dive                         | Engineering                    |
| [004-database-evidence.md](004-database-evidence.md)                 | Database Evidence — Live Measurements                | Postgres AGE / pgvector Expert |
| [005-complexity-model.md](005-complexity-model.md)                   | Complexity Model — O(N) Analysis                     | Algorithm Expert               |
| [006-battle-test-plan.md](006-battle-test-plan.md)                   | Battle-Test Improvement Plan                         | Full Stack / System Engineer   |
| [007-migration-strategy.md](007-migration-strategy.md)               | Migration Strategy — Preserving Automatic Migrations | DevOps / Migration             |
| [008-implementation-guide.md](008-implementation-guide.md)           | Implementation Guide — Code Changes                  | Rust / LightRAG Expert         |

---

## Quick Summary

```
SYMPTOM:  Storing a 200-entity document into the Knowledge Graph
          takes 30–120 seconds. A 2000-entity document can exceed 10 minutes.

ROOT CAUSES (ranked by impact):
  RC-1  AGE Cypher MERGE uses GIN containment scan — 69× slower than btree
  RC-2  HNSW index 41× the size of the raw vector data — O(M·log N) per insert
  RC-3  UNWIND MERGE still spawns N individual GIN lookups (not O(1) per batch)
  RC-4  KV store GIN index built on non-queryable 61 KB JSONB chunk texts
  RC-5  18+ duplicate indexes per AGE label — each INSERT maintains all of them
  RC-6  Community index refresh triggered on every document persist

IMPACT:   Processing time scales O(N × graph_size) where N = entities extracted
TARGET:   Processing time scales O(N · log G) where G = current graph size
```

---

## Critical Numbers (Measured 2026-06-30 — Code + EXPLAIN Verified)

| Metric | Value |
|---|---|
| Total AGE nodes (`"Node"` child) | 52,915 |
| Total AGE edges (`"EDGE"` child) | 67,636 |
| Cypher MERGE (GIN path) | **5.6 ms** |
| Native btree lookup | **0.081 ms** |
| Cypher vs btree ratio | **69×** |
| Incident edge query plan | **Seq Scan 67,636 rows** per BFS step |
| Missing fix | `ON "EDGE" ((start_id::text))` expression index |
| pgvector rows | 5,898 |
| HNSW index size | **909 MB** (12.3× heap, ef_construction=64) |
| HNSW query scans (dev stat) | 0 (IS used in prod ANN search) |
| Vector metadata GIN scans | 0 — no code uses containment search |
| KV GIN on value scans | 0 — no code queries KV by content |
| Duplicate FTS indexes on vectors | 2 copies (1.9 MB each) |
| Total unused index space | **1,076 MB across 317 indexes** |
| Total database size | **2,250 MB** |
| Unused indexes % of database | **47.8%** |

---

## Index Retention Decision Matrix (Code-Verified)

```
TABLE                  | INDEX                       | KEEP? | CODE PATH CONFIRMED
───────────────────────────────────────────────────────────────────────────────────
"Node"                 | idx_node_id                 | ✅    | AGE internal graphid
"Node"                 | idx_node_props_gin          | ✅    | Cypher MERGE/MATCH
"Node"                 | idx_node_prop_node_id       | ✅    | pg_get_nodes_batch
"Node"                 | idx_node_prop_node_id_btree | ✅    | pg_node_degree, BFS joins
"Node"                 | idx_node_workspace_id       | ✅    | search_nodes, list_nodes
"Node"                 | idx_node_tenant_id          | ✅    | multi-tenant reads
_ag_label_vertex (all) | ~10 parent indexes          | ❌    | Empty parent — data in Node
"EDGE"                 | idx_edge_start_id           | ✅    | AGE traversal
"EDGE"                 | idx_edge_source_id          | ✅    | get_edges_for_node_set
"EDGE"                 | idx_edge_target_id          | ✅    | get_edges_for_node_set
"EDGE"                 | idx_edge_end_id             | ✅    | Keep for end-side queries
"EDGE"                 | idx_edge_props_gin          | ❌    | 0 scans, 17 MB, no code uses it
"EDGE"                 | idx_edge_start_end          | ❌    | 0 scans, 4.9 MB
"EDGE"                 | idx_edge_source_target_btr  | ❌    | 0 scans, 4.8 MB
_ag_label_edge (all)   | ~10 parent indexes          | ❌    | Empty parent — data in EDGE
"EDGE" NEW             | ((start_id::text))          | ➕    | IMP-07: fix incident BFS
"EDGE" NEW             | ((end_id::text))            | ➕    | IMP-07: fix incident BFS
vectors                | embedding_idx (HNSW)        | ✅*   | ANN search (lower ef_construction)
vectors                | metadata_idx (GIN)          | ❌    | 0 scans, 13 MB, no GIN queries
vectors                | content_tsv_idx (dup)       | ❌    | Duplicate, 1.9 MB
kv                     | value_gin                   | ❌    | 0 scans, 112 MB, no code uses it
───────────────────────────────────────────────────────────────────────────────────
```
| KV index ratio                    | **155×** heap      |
