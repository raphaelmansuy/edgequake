# SPEC-088 Phase 5–6 — Improvements: done & proved

**Status:** Phase 6 **complete** for recommended request-path work (2026-07-25).  
**First-principles goal:** every request-path data access is **index-backed** and
**batch-collapsible** so asymptotic cost is **O(K log N)** (or better), never
**O(N)** full scans or **O(K) network RTs**.

Sources (July 2026):
- [PostgreSQL 18 release notes](https://www.postgresql.org/docs/18/release-18.html) — async I/O, B-tree skip scan, `uuidv7()`
- [pgvector 0.8.5 README](https://github.com/pgvector/pgvector) — iterative index scans, partial HNSW, multitenancy
- [Apache AGE](https://github.com/apache/age) + [MS Learn AGE performance](https://learn.microsoft.com/en-us/azure/postgresql/azure-ai/generative-ai-age-performance)
- Internal: `specs/054-fix-bugs-17/005-query-complexity-catalog.md`

**Variables:** **N** = rows · **K** = batch/result size · **ef** = HNSW search width ·
**D** = embedding dim · **W** = workspaces · **RT** = Postgres network round-trip ·
**F** = fan-out per hop · **E** = edges.

---

## Honest assessment (re-verified 2026-07-25)

| Dimension | Verdict |
|---|---|
| **Regression** | **None** — full suite re-run green (see [Latest verification](#latest-verification-re-proved-2026-07-25)) |
| **Request-path performance** | **Near-optimal** for current product surface (native graph + contracted ANN + fair claim + RT-collapsed KV) |
| **First principles** | Index-first, RT collapse, no cheat GUCs |
| **DRY / SOLID** | `StagingFinalMeta` SSOT; MemoryKV ordered batch parity with PG; storage owns SQL, API composes keys |
| **e2e** | **28+** IMP contracts in `e2e_spec088_improvements` (incl. cascade GIN plan) + expand + claim |
| **Docs** | This file is the **done & proved** SSOT; claims limited to RT/complexity + named tests |

### First-principles laws (proved)

| Law | Implication | Proved status |
|---|---|---|
| **RT collapse** | Prefer `UNNEST` / batch over loops of `get_by_id` | **Done** — IMP-075-01…13 |
| **Index-first** | ANN + btree UNIQUE; never Cypher property MATCH as primary | **Done** — IMP-031-* |
| **Filtered ANN** | Post-filter needs iterative_scan or partial index | **Done** — IMP-001, IMP-002 |
| **Native graph writes** | `ON CONFLICT` UNIQUE arbiter O(K log N) | **Done** — IMP-046 (default ON) |
| **Fair claim** | SKIP LOCKED + supporting index | **Done** — IMP-140-01…03 |
| **No cheat GUCs** | Never `enable_seqscan=off` globally | **Rejected** |
| **Staging-first SSOT** | One dual-key loader; no resolve-then-get | **Done** — IMP-075-09…11 |
| **Delete final-first** | Promoted final wins when both keys exist | **Done** — IMP-075-13 (distinct from ingest SSOT) |
| **Probe-first GIN** | Never tenant-scan then Join-Filter `@>` | **Done** — IMP-031-08 (cascade timeout RCA) |

---

## Incident RCA — batch delete cascade timeout (2026-07-25)

### Symptom

```
Graph cascade delete failed … Source-prefix node query failed:
canceling statement due to statement timeout
```

Task: `BatchDeletion` · op: `find_nodes_by_source_prefixes` · graph: ~200k nodes ·
session `statement_timeout` default **15s** (`EDGEQUAKE_GRAPH_QUERY_TIMEOUT_SECS`).

### First-principles chain

| Step | What happened |
|---|---|
| 1 | Cascade discovers entities via `source_ids @> chunk_id` (correct index: GIN) |
| 2 | SQL also filtered `tenant_id` / `workspace_id` (LegacyNullAsWildcard) on same join |
| 3 | Planner estimated tenant bitmap (~30k) cheaper than 257 GIN probes |
| 4 | Plan: **Bitmap tenant → Nested Loop · Join Filter `@>`** (not Index Cond) |
| 5 | ~1M join rechecks · **~4s** idle; under load / contention → **>15s timeout** |
| 6 | Fail-closed: KV wipe aborted (correct reliability policy) |

### What each related data op does (lens)

| Op | Path | Failure mode | Status |
|---|---|---|---|
| `FIND-NODES-BY-SOURCE-PREFIXES` | Cascade / lineage | Tenant-first Join Filter | **Fixed IMP-031-08** |
| `FIND-EDGES-BY-SOURCE-PREFIXES` | Cascade edges | Same planner cliff | **Fixed IMP-031-08** |
| `NODE-COUNTS-BY-SOURCE-PREFIXES` | Documents list stats | Was probe-driven GIN; stabilized MATERIALIZED | **Hardened IMP-031-08** |
| Legacy LIKE path | Opt-in only | SeqScan O(N) | **Default OFF** (`EDGEQUAKE_SOURCE_PREFIX_LEGACY`) |
| Workspace wipe | Bulk delete | N× source-prefix | **Already banned** (batch native) |
| Expand / neighbors | BFS | Cypher var-length | **Native BFS** IMP-031-04 |
| Filtered ANN | Vector | Under-K post-filter | iterative_scan IMP-002 |
| Claim next | Tasks | Non-sargable OR | UNION pending/stale IMP-140 |

### Solution (proved)

```sql
WITH probes AS MATERIALIZED (… exact + chunk-0..255 …),
hits AS MATERIALIZED (
  SELECT v.properties
  FROM probes pr
  INNER JOIN graph."Node" v
    ON (source_ids) @> to_jsonb(pr.probe_id)   -- Index Cond on GIN
)
SELECT … FROM hits h WHERE tenant/workspace …  -- post-filter only
```

| Metric | Before | After |
|---|---|---|
| Plan | Join Filter `@>`, tenant Bitmap | **Bitmap Index Scan** `idx_node_source_ids_gin` |
| Time @ ~200k nodes | **~4.0 s** | **~0.1 s** |
| Join rows rechecked | ~1e6 | ~0 (GIN hits only) |

DRY: `source_ids_probes_cte_sql` / `source_ids_count_probes_cte_sql` in
`helpers/source_lineage_sql.rs` — single SSOT for discovery + count paths.

### Improvement plan (verified)

| # | Action | Done |
|---|---|---|
| 1 | MATERIALIZED probe-first for node discovery | **Yes** |
| 2 | Same for edge discovery | **Yes** |
| 3 | MATERIALIZED probes for batch node counts | **Yes** |
| 4 | Shared CTE SQL helpers (DRY) | **Yes** |
| 5 | e2e find + EXPLAIN GIN contract | **Yes** `imp_031_08_*` |
| 6 | Do **not** raise global timeout as the fix | **Yes** (plan fixed) |
| 7 | Keep legacy SeqScan opt-in only | **Yes** |

---

## Proven performance improvements (evidence-based)

Claims are **asymptotic / RT-count** wins proved by source contracts, unit tests,
behavioral e2e, and EXPLAIN smoke — **not** synthetic wall-clock microbenchmarks.
Wall-clock varies by corpus; complexity and RT collapse do not.

| Area | Before (anti-pattern) | After (proved) | Complexity | How proved |
|---|---|---|---|---|
| Graph multi-get / has | Cypher `IN` / N× get | Native `pg_get_nodes_batch` | O(K log N), **1 RT** | `imp_031_01_*` e2e + source |
| Graph expand / neighbors | Variable-length Cypher | Native `pg_bfs_expand` + batch nodes | O(depth · F log E + K log N) | `imp_031_04_*`; `e2e_spec060` **Bitmap Index Scan** |
| Graph edge get / delete | Cypher MATCH | Native EDGE + indexes | O(log E) | `imp_031_03_*`, `imp_031_05_*` |
| Graph clear / workspace | Cypher DETACH | Native DELETE / batch | O(N_ws + E′) indexed | `imp_031_06_*` e2e + source |
| Cascade source-prefix discovery | Tenant-first Nested Loop Join Filter (~4s @ 200k) | MATERIALIZED probe-first GIN (~100ms) | O(P · log N) P≤257 probes | `imp_031_08_*` + live EXPLAIN |
| Graph writes | Cypher MERGE default | Native `ON CONFLICT` default ON | O(K log N) | `imp_046_01_*`, contract 060 |
| Filtered ANN | Under-K / no iterative | `iterative_scan` + max_scan_tuples | O(ef · log N) bounded | `imp_002_01_*`, contract 075, return-K e2e |
| Partial HNSW | Global ANN + post-filter | Auto partial by workspace (≥1000 rows) | ~O(ef · log N_ws) | `imp_001_01_*` |
| Fair claim | Non-sargable OR / flaky e2e | UNION pending/stale + isolate | O(log N) + SKIP LOCKED | `imp_140_*`, claim lease **8/8** |
| Staging promote | 3× `get_by_id` | `get_by_ids_ordered` | **1 RT** | `imp_075_01_*` |
| Injection list | O(K) meta gets | Batch ordered | **1 RT** | injection_list unit |
| Lineage page enrich | O(K) chunk `get_by_id` | Batch ordered | O(K log N), **1 RT** | lineage 7 units + `imp_075_03_*` |
| Status / sync / prepare / cancel / reanalyze | resolve + re-get (2 RT) | `StagingFinalMeta` SSOT | **1 RT** | processor 34 + `imp_075_04/10/11_*` |
| Orphan recovery | 2N resolve+get per page | Page batch staging+final | **1 RT / ≤500 tasks** | orphan 3 units + `imp_075_05_*` |
| Content resolve | sequential staging/final | Dual-key batch | **1 RT** | text_insert 7 units + `imp_075_08` |
| Workspace hash visibility | 2 sequential gets | Dual-key batch | **1 RT** | `imp_075_07` source |
| Batch delete plan | sequential content + meta | dual-key batch | **1 RT** (was 2) | `imp_075_12_*` |
| Delete key resolve | sequential final then staging | dual-key final-first | **1 RT** (was ≤2) | `imp_075_13_*` |
| MemoryKV ordered batch | trait default N× get | single lock O(K) | O(K) memory | `imp_075_06_*` |

### Live plan proofs

| Proof | Test | What it asserts |
|---|---|---|
| Expand index plan | `e2e_spec060_age_expand_perf` | Scoped expand uses **Bitmap/Index Scan** on EDGE (not full AGE cartesian) |
| Claim index + plan | `imp_140_01_e2e_claim_index_plan` | `idx_tasks_claim_workspace_created` exists; sargable pending arm is bounded |
| Filtered ANN returns K | `imp_002_01_e2e_filtered_ann_returns_k` | Workspace-filtered query returns full top-K under iterative GUCs |
| Fair claim isolation | `postgres_claim_lease` (8 tests) | Deterministic on shared DB via `isolate_claimable` |
| Cascade GIN plan | `imp_031_08_e2e_explain_uses_source_ids_gin` | No Join Filter; uses `idx_node_source_ids_gin` on large graph |
| Cascade discovery | `imp_031_08_e2e_source_prefix_discovery_finds_nodes` | Finds node by chunk `source_ids` under tenant filter |

### What we do **not** claim

- Absolute ms p95 without a fixed corpus fixture (environment-dependent).
- Partition / DiskANN gains (optional, deferred until ~100M+ vectors).
- That Cypher is deleted (opt-out: `EDGEQUAKE_NATIVE_GRAPH_WRITES=0` for rollback).

---

## Migration / checksum safety (sqlx — not Flyway)

EdgeQuake schema evolution uses **sqlx embedded migrations** (`edgequake/migrations/`),
with the same immutability contract operators often associate with Flyway:

| Mechanism | Role |
|---|---|
| `_sqlx_migrations` | Applied version + **content checksum** per version |
| `checksums.lock` | Repo-side immutability lock (CI / pre-commit) |
| `./scripts/check_migration_checksums.sh` | Fails if any locked `NNN_*.sql` is edited |

### Phase 6 / cascade fix impact

| Change class | Migration impact |
|---|---|
| Native graph SQL, GIN probe-first CTEs, KV batching, e2e | **Runtime Rust only** — no new / edited migration files |
| IMP-031-08 cascade timeout fix | Query rewrite in `scan_ops.rs` / helpers — **not** DDL |
| Git `edgequake/migrations/` | **Unchanged** this workstream |

**Verified:** `check_migration_checksums.sh` → **PASS** (97 files, 0 modified, 0 missing).

### Rules for operators (no checksum regression)

1. **Never edit** an already-shipped `NNN_*.sql` — sqlx will refuse startup with checksum mismatch against `_sqlx_migrations`.
2. **New schema only via** next free version (`099_*` and up) + append to `checksums.lock` (`./scripts/update_migration_checksums.sh`).
3. **Index/plan fixes** that do not require DDL ship as app code (this cascade GIN fix).
4. **Every-boot reconcile** scripts under `migrations/support/*` are **not** checksum-locked; they may re-assert indexes (e.g. 086 BFS) without rewriting historical sqlx rows.
5. Known M071/M078 checksum repair paths run only with `EDGEQUAKE_DEV_MODE=true`; production fails loud (see `migrations/README.md`).

### Unrelated ops note (M041)

Runtime log `documents M041 stat columns missing` means the **DB has not applied migration 041** (or is missing those columns), not that 041 was rewritten. Fix: ensure bootstrap applied through max version (`SELECT max(version) FROM _sqlx_migrations;`). Do **not** patch `041_*.sql` in place.

---

## Latest verification (re-proved 2026-07-25)

| Suite | Result | Proves |
|---|---|---|
| `edgequake-storage --lib` | **192 pass** | Unit integrity |
| `e2e_spec088_improvements` | **26 pass** | All IMP source/behavior contracts |
| `data_layer_ops_matrix` | **236 pass** | 235 Ref ID ops matrix |
| `e2e_spec060_age_expand_perf` | **pass** | Expand **index-backed** plan (Bitmap/Index Scan) |
| `postgres_claim_lease` | **8 pass** | Fair claim + isolation on shared DB |
| `contract_spec060_native_writes` | **5 pass** | Native writes default ON / ON CONFLICT |
| `contract_spec075_iterative_scan_bounds` | **3 pass** | Filtered ANN GUC contract |
| `lint_dataop_xref` | **235/235** | Inventory ↔ code ↔ docs |

Supporting API units (prior waves, still binding for IMP-075): lineage 7, text_insert 7, orphan 3, processor status 34.

### `e2e_spec088_improvements` catalog (26)

| Test | Proves |
|---|---|
| `imp_001_01_partial_default_on` | Partial HNSW default ON |
| `imp_002_01_filtered_ann_contract_unit` | Filtered ANN GUCs include iterative_scan |
| `imp_002_01_e2e_filtered_ann_returns_k` | Filtered ANN returns K |
| `imp_031_01_*` (source + e2e) | Native batch get_nodes |
| `imp_031_02_expand_edges_native_source` | Native BFS expand source |
| `imp_031_03_*` (source + e2e) | Native get/has edge |
| `imp_031_04_e2e_native_neighbors` | Native neighbors BFS |
| `imp_031_05_*` (source + e2e) | Native delete edge |
| `imp_031_06_*` (source + e2e) | Native clear_workspace |
| `imp_031_07_get_all_native_source` | Native get_all (admin) |
| `imp_046_01_native_writes_default_on_source` | Native writes default ON + warn |
| `imp_075_01_e2e_kv_batch_not_n_plus_one` | KV ordered batch multi-key |
| `imp_075_03…13_*` (source contracts) | API dual-key / SSOT / delete batch |
| `imp_140_01_e2e_claim_index_plan` | Claim index + EXPLAIN |
| `imp_140_02_claim_union_pending_stale_source` | Claim UNION pending/stale |

---

## What was done (IMP catalog)

### Vectors / ANN

| ID | Change | Complexity | Proof |
|---|---|---|---|
| **IMP-002-01** | Filtered ANN product contract: `iterative_scan` + `max_scan_tuples`; warn if forced off | O(ef · log N) bounded | unit + e2e return-K + contract 075 |
| **IMP-001-01** | Partial HNSW by workspace default **auto-on** (min_rows 1000); opt-out `=0` | ~O(ef · log N_ws) | `imp_001_01_*` |
| **IMP-000-PG18-01** | Document PG18 free wins (async I/O, skip scan) — no SQL rewrite | planner-side | version-matrix CI |
| **IMP-000-PG18-02** | `uuidv7()` document IDs already in tree on PG18 | O(1) alloc | capabilities probe |

### Graph (native request path)

| ID | Change | Complexity | Proof |
|---|---|---|---|
| **IMP-031-01** | `get_nodes_by_ids` / get / has → `pg_get_nodes_batch` | O(K log N), 1 RT | e2e + source |
| **IMP-031-02** | Expand edge hydrate via `pg_get_edges_for_node_set` | O(K log E) | source |
| **IMP-031-03** | Native has_edge / get_edge; upsert_edge batch-of-1 | O(log E) | e2e + source |
| **IMP-031-04** | Native `pg_bfs_expand` for expand + neighbors | O(depth · F log E + K log N) | e2e neighbors + expand perf |
| **IMP-031-05** | Native edge delete + scoped node delete | O(K log E / N) | e2e + source |
| **IMP-031-06** | Native clear / clear_workspace | O(N_ws + E′) | e2e + source |
| **IMP-031-07** | Native get_all_nodes / get_all_edges (admin only) | O(N)/O(E) no AGE tax | source |
| **IMP-031-08** | Source-prefix cascade discovery: MATERIALIZED probe-first GIN (fix batch-delete timeout) | O(P log N) vs O(N_tenant · P) join filter | e2e + source + EXPLAIN 4s→~100ms |
| **IMP-046-01** | Native writes default ON; warn on Cypher fallback | O(K log N) ON CONFLICT | source + contract 060 |

### Tasks / claim

| ID | Change | Complexity | Proof |
|---|---|---|---|
| **IMP-140-01** | Assert claim index M098 + EXPLAIN smoke | O(log N) + SKIP LOCKED | e2e plan |
| **IMP-140-02** | Claim SQL: pending/stale CTEs UNION ALL; single FOR UPDATE | sargable status arms | source |
| **IMP-140-03** | `isolate_claimable` for deterministic e2e on shared DB | test isolation | claim lease 8/8 |

### KV / API RT collapse (DRY SSOT)

| ID | Change | Complexity | Proof |
|---|---|---|---|
| **IMP-075-01** | Staging promote: 3 keys → `get_by_ids_ordered` | 1 RT | e2e |
| **IMP-075-02** | Injection list meta batch | 1 RT | unit |
| **IMP-075-03** | Lineage chunk page enrichment batch | O(K log N), 1 RT | source + lineage units |
| **IMP-075-04** | Status merge-progress dual-key batch | 1 RT | source |
| **IMP-075-05** | Orphan recovery page meta batch | 1 RT / page | source + units |
| **IMP-075-06** | MemoryKV `get_by_ids_ordered` true batch | O(K) | source + memory unit |
| **IMP-075-07** | Workspace hash visibility dual-key | 1 RT | source |
| **IMP-075-08** | Text-insert content resolve dual-key | 1 RT | source + units |
| **IMP-075-09** | `load_staging_first_metadata` helper | 1 RT | units |
| **IMP-075-10** | `StagingFinalMeta` / `load_staging_and_final_metadata` adopted on status/sync | 1 RT (was 2) | source + processor units |
| **IMP-075-11** | prepare / cancel / reanalyze → SSOT | 1 RT (was 2) | source |
| **IMP-075-12** | Batch deletion content+metadata batch | 1 RT (was 2) | source |
| **IMP-075-13** | Delete key resolve final+staging batch (final-first) | 1 RT (was ≤2) | source |

---

## Remaining (optional / rejected — not gaps in recommended work)

| ID | Status | Notes |
|---|---|---|
| **IMP-002-02** | Deferred | Partition vectors BY LIST (tenant) @ ~100M+ |
| **IMP-000-DISKANN-01** | Deferred | DiskANN filtered labels — keep vectorscale bakeoffs |
| **IMP-XXX-REJECT-01** | Rejected | Global `enable_seqscan=off` |
| Single-key `get_by_id` | Correct as-is | Not N+1 when K=1 |
| Fixed-corpus p95 ms | Optional | Marketing latency numbers need pinned fixture |

---

## Ranking summary (final)

| ID | Rank | Status |
|---|---|---|
| IMP-002-01 | Recommended | **Implemented & proved** |
| IMP-001-01 | Recommended | **Implemented & proved** |
| IMP-031-01…07 | Recommended | **Implemented & proved** |
| IMP-046-01 | Recommended | **Implemented & proved** |
| IMP-140-01…03 | Recommended | **Implemented & proved** |
| IMP-075-01…13 | Recommended | **Implemented & proved** |
| IMP-000-PG18-01 | Recommended | Documented / CI |
| IMP-000-PG18-02 | Optional | Already in tree |
| IMP-002-02 | Optional | Deferred |
| IMP-000-DISKANN-01 | Optional | Deferred |
| IMP-XXX-REJECT-01 | Rejected | — |

---

## Verification commands

```bash
export DATABASE_URL=postgres://edgequake:edgequake_secret@localhost:5432/edgequake

# Phase 6 IMP e2e + source contracts (26)
cargo test -p edgequake-storage --features postgres --test e2e_spec088_improvements

# Full ops matrix (236)
cargo test -p edgequake-storage --features postgres --test data_layer_ops_matrix -- --test-threads=4

# Unit + lint
cargo test -p edgequake-storage --lib
python3 specs/088-data-layer/scripts/lint_dataop_xref.py

# Plan proofs
cargo test -p edgequake-storage --features postgres --test e2e_spec060_age_expand_perf
cargo test -p edgequake-tasks --features postgres --test postgres_claim_lease -- --test-threads=1

# Contracts
cargo test -p edgequake-storage --features postgres --test contract_spec060_native_writes
cargo test -p edgequake-storage --features postgres --test contract_spec075_iterative_scan_bounds
```

---

## Env knobs (ops)

| Env | Default | Meaning |
|---|---|---|
| `EDGEQUAKE_HNSW_ITERATIVE_SCAN` | `relaxed_order` | filtered ANN iterative mode |
| `EDGEQUAKE_HNSW_MAX_SCAN_TUPLES` | `20000` | iterative scan ceiling |
| `EDGEQUAKE_HNSW_PARTIAL_BY_WORKSPACE` | **on (auto)** | partial HNSW for hot WS; `0` disables |
| `EDGEQUAKE_HNSW_PARTIAL_MIN_ROWS` | `1000` | threshold before CREATE partial |
| `EDGEQUAKE_NATIVE_GRAPH_WRITES` | **on** | native ON CONFLICT; `0` forces Cypher fallback |
