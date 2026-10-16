# SPEC-164 — Prior Decisions (dated, cited)

Numbers below are **historical**. They are not SPEC-164 results. SPEC-164 results live only in [`measurements/results.json`](measurements/results.json).

---

## Locked until this study reopens them

| Id | Statement | Source | Status for 164 |
|----|-----------|--------|----------------|
| LD-04 (091) | AGE is traversal authority; do not move graph work to recursive SQL | [SPEC-091](../091-simplify-data-layer/05-target-specification.md) | **Reopened** for measurement |
| LAW-148-1 | Cloud SQL / AlloyDB must not be sole DB while `age` missing | [SPEC-148](../148-gcloud-hosting/01-first-principles.md) | Remains true until migrate ships |
| D4 (149) | Relational `entities`/`relationships` are not traversal until replay proven | [SPEC-149](../149-data-access-improvements/design/why.md) | Still true; not the ARM-B schema |
| 034 non-goal | Do not replace Apache AGE | [034-001](../034-graph-storage-improvement/001-problem-statement.md) | **Reopened** by this study’s charter |

---

## SPEC-034 measurements (2026-06-30)

Database: Docker EdgeQuake Postgres. Row counts: **52,915** `"Node"`, **67,636** `"EDGE"`, **5,898** vectors. DB size **2,250 MB**. Source: [`004-database-evidence.md`](../034-graph-storage-improvement/004-database-evidence.md).

| Claim | Value | Kind |
|-------|-------|------|
| Cypher `MATCH {node_id}` | Seq scan, **12.390 ms**, 3,875 buffers | EXPLAIN ANALYZE |
| Native btree node_id | Index scan, **0.081 ms**, 3 buffers | EXPLAIN ANALYZE |
| Ratio vs seq scan | **153×** | derived |
| Cypher `MERGE` one node | GIN bitmap, **5.656 ms**, 263 pages | EXPLAIN ANALYZE |
| Cypher vs btree | **69×** (5.656 / 0.081) | derived |
| UNWIND MERGE ×3 | **6.644 ms** (~2.21 ms/node) | EXPLAIN ANALYZE |
| Incident BFS join | Seq Scan `"EDGE"` 67,636 rows | EXPLAIN (pre-072) |
| Single node degree | **0.106 ms** | EXPLAIN ANALYZE |
| Wall-clock ingest before/after | **not measured** | gap |

Shipped follow-ups: migrations 067–076, 083, 086; `native_graph_writes_enabled()` default **on**.

---

## Current product path (code, not 034)

| Fact | Locus |
|------|-------|
| Expand is native BFS, not Cypher `[*1..n]` | `query_ops/expand.rs` |
| Depth clamp 1..=3, max neighbors 500 | `pg_get_neighbors` |
| Incident edges chunk size 200 | `pg_get_incident_edges_batch` |
| Cypher write only if `EDGEQUAKE_NATIVE_GRAPH_WRITES=0` | `graph/mod.rs` |
| Boot fail-closed on missing AGE | `connection.rs` + `EDGEQUAKE_ALLOW_NO_GRAPH` |
| Dead fallback tables | `graph_nodes` / `graph_edges` in migration 013 — **unused** |
| CQRS projection | `entities` / `relationships` — different key; optional sync |

---

## What SPEC-164 must re-prove

1. Traversal latency of plain adjacency vs AGE native SQL at **200k** nodes (decision scale).  
2. Correct neighbor sets under the same caps.  
3. Level-set CTE vs iterative hop SQL (query shape).  
4. Path-set `UNION ALL` and Hub degree-1000 as negative controls.  
5. Batch upsert write cost without `agtype` / `graphid` helpers.  
6. Whether any used invariant requires AGE types (LAW-164-05).

---

## Industry context (not EdgeQuake measurements)

As of mid-2026 industry surveys: AGE available on Azure Flexible Server / some niches; **not** on AWS RDS/Aurora, GCP Cloud SQL, Neon, Supabase. PostgreSQL 19 SQL/PGQ is core fixed-depth graph syntax — **out of scope** (product pin PG18). Recursive CTE level-set vs path-set explosion is a known Postgres behavior; Hub arm is the product’s control for that.
