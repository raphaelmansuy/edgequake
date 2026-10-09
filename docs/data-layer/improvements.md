---
title: "SPEC-088 Phase 5-6: improvements, done and proved"
description: "Historical record of the SPEC-088 performance work (2026-07-25): incident root causes, the improvement catalog, measured before and after figures, and the tests that prove them. Some parts predate the typed-table cutover."
---

# SPEC-088 Phase 5-6: improvements, done and proved

> **Historical record.** This log covers the SPEC-088 Phase 5-6 work dated **2026-07-25 and 2026-07-26**. The measured figures and test counts below are kept exactly as recorded. The code has changed since then, so treat them as a snapshot and not as current results.
>
> Parts that are now out of date:
>
> | Topic in this log | What is true now |
> |---|---|
> | Key-value keys such as `wsdoc:` and `{id}-metadata`, staging and final KV, the IMP-075 round-trip work | The `eq_*_kv` tables were drained and dropped by migration 125. Typed tables replaced them. See [postgres.md](./postgres.md#legacy-key-value-store). The IMP-075 entries describe code paths that now run on relational tables. |
> | Partial HNSW by workspace (IMP-001-01), `eq_*_vectors` | These belong to the older per-workspace vector adapter. Typed embedding tables use fixed per-dimension partial indexes. See [pgvector.md](./pgvector.md). |
> | Claim index `idx_tasks_claim_workspace_created` (migration 098) | A database built from all migrations does not have an index with this name. The test creates it itself. Current task indexes are in [indexes.md](./indexes.md#task-indexes). |
> | Migration rules and counts ("97 files", "next free version 099", `EDGEQUAKE_DEV_MODE`) | See [Migration and checksum safety](#migration-and-checksum-safety) below. |

**Status:** Phase 6 **complete** for recommended request-path work (2026-07-25).  
**Follow-on:** [SPEC-090 performance](../../specs/090-performance/README.md) — counter serialization, claim_next O(N), PDF list TOAST, pool GUC leak, halfvec default (Waves 0–4 landed 2026-07-26; RCA cross-ref `specs/090-performance/00-why.md`).  
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
| **List-surface completeness** | Delete success ⇒ no row on merge(KV, SQL, wsdoc) | **Done** — `purge_document_list_surfaces` SSOT |
| **App timeout ≠ kill** | `tokio::timeout` without `SET LOCAL statement_timeout` is a zombie pool factory (LAW-H2) | **Done** — SPEC-089 / GH-336 Wave 1–3 |

---

## Incident RCA — GH-336 health / pool exhaustion (2026-07-25)

### Symptom

At ~9.5k documents, `/health` “task queue statistics” timed out; pool (`DATABASE_POOL_SIZE=15`) saturated; claim/checkpoint failed with acquire timeout.

### Root cause (code is law)

| Step | What happened |
|---|---|
| 1 | Documents list ran P-A3 `node_counts_by_source_prefixes` on **all** zero-count docs **before** pagination |
| 2 | SQL: `CROSS JOIN generate_series(0,255)` → `N × 256` GIN probes (~2.4M at 9.5k docs) |
| 3 | Rust `timeout(400ms)` abandoned the future; Postgres kept running (no `statement_timeout`) |
| 4 | Concurrent pollers stacked zombies → `/health` cheap `get_statistics` starved |

GH-331 had already fixed JOIN locality (`"Node"` + GIN). GH-336 is **query shape + cancel**, not index locality.

### AFTER (SPEC-089)

| Layer | Fix |
|---|---|
| List | Reconcile **after** `paginate_vec` (page only) — LAW-H1 |
| Count SQL | Batch ≤32 + `SET LOCAL statement_timeout=300ms` — LAW-H2 |
| Probe bound | From page `chunk_count` when known |
| Wave 3 | `LocalTimeoutTx` on discovery / task stats / native graph / labels / BFS edges / INV-C; app timeout > PG kill |
| Phase 4 | Reprocess single cascade; workspace stats 3750ms PG kill; interactive budget helper |

**SSOT:** [`specs/089-health-check/`](../../specs/089-health-check/) · tests `e2e_issue336_*` · `contract_spec089_*`

---

## Incident RCA — ghost documents after multi-delete (2026-07-25)

> The list merged key-value metadata with SQL rows. The key-value part no longer exists (migration 125). The lesson still holds: one function must clean every list surface, and cleanup must fail closed.

### Symptom

UI multi-delete of completed MD docs returns success; after refresh the same
docs reappear as **Completed** (e.g. areal `*.md`).

### First-principles chain

| Step | What happened |
|---|---|
| 1 | List = `merge(KV metadata, SQL documents)` + optional `wsdoc:` index scan |
| 2 | Cascade wiped `{id}-metadata` but left `wsdoc:` and/or SQL `documents` |
| 3 | Batch path treated "no metadata/chunks/content" as **success without purge** (`Ok(None)`) |
| 4 | SQL delete was **warn-only**; wsdoc delete gated on `has_metadata` |
| 5 | Refresh re-merged relational/wsdoc → ghost **Completed** rows |

### Similar incomplete-delete paths (fixed via same SSOT)

| Path | Bug | Fix |
|---|---|---|
| `perform_document_deletion` | wsdoc only if `has_metadata`; SQL warn-only | Always call `purge_document_list_surfaces` (fail-closed) |
| `BatchDeletion` `Ok(None)` | Counted success; no SQL/wsdoc cleanup | Orphan purge on already-absent |
| `delete_document_for_reingestion` | KV only; left SQL/wsdoc | Same SSOT after wipe |
| `workspace_document_wipe::purge_one_document_kv` | Skipped wsdoc per doc | Same SSOT (bulk SQL remains at wipe end) |

### Solution (DRY / SOLID)

Single responsibility: **list-surface purge** is one function used by cascade
completion, batch orphan, re-ingest, and wipe. Graph/vector stay on the cascade
order; list identity cleanup is never best-effort.

```text
purge_document_list_surfaces(doc, workspace, tenant):
  delete final+staging metadata/content for id and key_prefix
  delete wsdoc:{ws}:{id} and wsdoc:{ws}:{prefix}
  delete content-hash (+ staging hash) when known
  DELETE FROM documents WHERE id AND workspace AND tenant  -- fail-closed
```

### Tests

- Unit: `purge_list_surfaces_removes_wsdoc_and_metadata_even_when_orphaned`
- Unit: `purge_list_surfaces_handles_key_prefix_mismatch`
- e2e: `batch_delete_purges_orphan_list_surfaces_after_incomplete_cascade`

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

The next diagram compares the two plans. Read each row from left to right.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart LR
    subgraph Before
        B1["Tenant bitmap, about 30k rows"] --> B2["Nested loop with join filter"]
        B2 --> B3["About 1e6 rechecks, about 4 s"]
    end
    subgraph After
        A1["257 probe IDs"] --> A2["GIN index condition"]
        A2 --> A3["Few hits, about 0.1 s"]
    end
```

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

## Migration and checksum safety

This section was rewritten to match the current migration system (SPEC-150). The original text described the sqlx-only rules of mid-2026.

Rules that still hold:

- Never edit a shipped `NNN_*.sql` file. `edgequake/migrations/checksums.lock` pins every file, and `./scripts/check_migration_checksums.sh` fails if one changes.
- Query-plan fixes that need no DDL ship as application code. The cascade GIN fix did this: it changed `scan_ops.rs` and helpers, not a migration.
- Support scripts under `edgequake/migrations/support/` are not scanned by sqlx. Some of them (083, 086, 092) run on every boot to restore indexes and columns.

What is different now:

| Then (this log) | Now |
|---|---|
| 97 migration files | 167 files, versions 001 to 169 |
| New migrations start at `099` | The next version is 170. Add the file, then run `./scripts/update_migration_checksums.sh`. |
| The API applies migrations on start | Only `edgequake migrate` writes the schema. The API checks the schema gate and exits with 78 or waits (`EDGEQUAKE_SCHEMA_GATE`). |
| A checksum mismatch refuses startup; repair needed `EDGEQUAKE_DEV_MODE=true` | Known historical hash variants ("fossils") are listed in `manifest.toml` and accepted automatically. An unknown checksum exits with 65. |

The log recorded `check_migration_checksums.sh` as passing on 97 files with none modified or missing. That count is from 2026-07-25.

A log line such as `documents M041 stat columns missing` means the database has not applied migration 041. Run `edgequake migrate`. Do not edit `041_*.sql`.

Read next: [Upgrading](../operations/upgrading.md), [edgequake/docs/migrations.md](../../edgequake/docs/migrations.md), and [postgres.md](./postgres.md#schema-and-migrations).

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

Checked against the code on the date of the page rewrite. For the full list with the typed-table settings, see [pgvector.md](./pgvector.md#how-a-vector-search-runs).

| Env | Default | Meaning |
|---|---|---|
| `EDGEQUAKE_HNSW_ITERATIVE_SCAN` | `relaxed_order` | filtered ANN iterative mode |
| `EDGEQUAKE_HNSW_MAX_SCAN_TUPLES` | `20000` | iterative scan ceiling |
| `EDGEQUAKE_HNSW_PARTIAL_BY_WORKSPACE` | **on (auto)** | partial HNSW for hot workspaces (older adapter); `0` disables |
| `EDGEQUAKE_HNSW_PARTIAL_MIN_ROWS` | `1000` | threshold before CREATE partial |
| `EDGEQUAKE_NATIVE_GRAPH_WRITES` | **on** | native ON CONFLICT; `0` forces Cypher fallback |
