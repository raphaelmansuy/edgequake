# SPEC-011 — Iteration 02 Audit (Production Evidence)

> **Status**: Active  
> **Trigger**: New production query log [`specs/012-performance/data/queryedgeQuake.csv`](../012-performance/data/queryedgeQuake.csv)  
> **Method**: Map every recurring query in production to its code origin, classify by O(N) risk, list remaining gaps after Phase 1/2.

Cross-refs:
- [README.md](./README.md) — document map
- [QUERY_CATALOG.md](./QUERY_CATALOG.md) — complete SQL inventory
- [BRUTAL_ASSESSMENT.md](./BRUTAL_ASSESSMENT.md) — honest limits acknowledged in phase 2
- [IMPROVEMENT_PLAN.md](./IMPROVEMENT_PLAN.md) — phased plan (phases 1, 2, 3 done; this iteration is phase 4)
- [ITERATION_02_PLAN.md](./ITERATION_02_PLAN.md) — fix plan for the gaps documented here
- [ITERATION_02_PROOF.md](./ITERATION_02_PROOF.md) — measurements after implementation

---

## 1. WHY this iteration exists

Phase 1 closed the **13 s** `/health` incident.
Phase 2 closed the `keys()` full-key download in document handlers.
Phase 3 made KV `count()` O(1) via a maintained counter table.

The production log captured on **2026-05-11** still shows a 30 s polling cycle that hits four full-table scans every interval:

```text
sqlx_s_2  SELECT COUNT(*)::bigint FROM eq_eq_default_graph."_ag_label_vertex"     7.7–36.4 ms
sqlx_s_2  SELECT COUNT(*)        FROM public.eq_eq_default_vectors                0.1–0.2  ms
sqlx_s_3  SELECT COUNT(*) as count FROM public.eq_eq_default_kv                   1.6–13.7 ms
sqlx_s_3  SELECT COUNT(*) FILTER (WHERE success = true) ... FROM _sqlx_migrations 0.05–0.2 ms
```

The KV row is the **count fallback** path — see [`kv.rs#L407`](../../edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs). It fires only when the maintained `_kv_stats` table is missing. The fact that we see this every 30 s in production tells us:

1. The cluster runs a build **without** the SPEC-011 stats table initialised on the existing KV table, **or**
2. A caller still invokes `kv_storage.count()` on a hot path (dashboard polling), forcing the fallback.

Either way, the symptom is the same shape as the original incident: linear scans on growing tables, repeated forever, dominating `pg_stat_statements`.

For vector and graph, **there is no SPEC-011 fix yet**. Both `count()` methods are still raw `SELECT COUNT(*)`. As the dataset grows (current vectors are tiny — sub-millisecond — but graph vertex count already reaches **36 ms**) these become the next incident.

## 2. First-principles framing

A health/dashboard poll is a **liveness signal**, not a data query. Its job is to answer "is component X reachable and roughly populated?". The required information bits are O(1): a connection, a relation existence check, optionally a coarse cardinality.

Whenever a liveness signal is implemented as `SELECT COUNT(*) FROM heap`, the **information cost** (a single integer) is paid in O(N) **work cost** (every visible row touched). The asymmetry only manifests when N grows. For an indefinitely growing table polled every 30 s, the integral is unbounded:

```
Work(t) = ∑_{i=0..t/30s} N(i)        — linear in time × table size
```

The fix is to **separate liveness from cardinality**:
- Liveness: `ping()` — O(1), uses one indexed read.
- Coarse cardinality (dashboard "≈ 12k entities"): `pg_class.reltuples` — O(1), planner estimate refreshed by autovacuum.
- Exact cardinality (audit/test): maintained counter or relaxed to an admin-only endpoint with explicit cost.

## 3. Production-query → code origin map

| # | Query (production log) | Caller (code) | Method | Adapter | Today's complexity | Iteration-02 target |
|---|------------------------|---------------|--------|---------|--------------------|---------------------|
| 1 | `SELECT COUNT(*) as count FROM public.eq_eq_default_kv` | KV `count()` fallback ([`kv.rs#L407`](../../edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs)) | `KVStorage::count` | Postgres KV | **O(N)** (fallback) | Already O(1) primary; eliminate fallback callers on hot path |
| 2 | `SELECT COUNT(*) FROM public.eq_eq_default_vectors` | Vector `count()` ([`vector.rs#L682`](../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs)) | `VectorStorage::count` | pgvector | **O(N)** | **O(1)** via `vectors_stats` table + triggers (Fix A) |
| 3 | `SELECT COUNT(*)::bigint FROM eq_eq_default_graph."_ag_label_vertex"` | Graph `node_count()` ([`graph/mod.rs#L1107`](../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs)) called from [`graph_stream.rs#L82`](../../edgequake/crates/edgequake-api/src/handlers/graph/graph_stream.rs), `popular.rs`, `traversal.rs` | `GraphStorage::node_count` | AGE | **O(N)** native scan | **O(1)** via `node_count_fast()` using `pg_class.reltuples` for polling (Fix B) |
| 4 | `SELECT COUNT(*)::bigint FROM eq_eq_default_graph."_ag_label_edge"` | Graph `edge_count()` ([`graph/mod.rs#L1126`](../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs)) | `GraphStorage::edge_count` | AGE | **O(N)** native scan | **O(1)** via `edge_count_fast()` (Fix B) |
| 5 | `SELECT COUNT(*) FILTER (WHERE success=true), MAX(version) … FROM _sqlx_migrations` | `get_schema_health` ([`health.rs#L146`](../../edgequake/crates/edgequake-api/src/handlers/health.rs)) | `/health` | shared pool | **O(M)** with M ≈ migration count (~ tens) | Acceptable — bounded, sub-ms; cache in `AppState` once per process (Fix D, optional) |

In addition, the workspace-stats handler ([`workspaces/stats.rs#L160`](../../edgequake/crates/edgequake-api/src/handlers/workspaces/stats.rs)) is invoked from the dashboard with `staleTime: 30000` ([`webui/src/app/page.tsx#L41`](../../edgequake_webui/src/app/page.tsx)) and triggers:

| # | Query pattern | Caller | Today's complexity | Iteration-02 target |
|---|---------------|--------|--------------------|---------------------|
| 6 | `SELECT key FROM eq_eq_default_kv WHERE key LIKE '%-metadata'` | `keys_like("%-metadata")` | **O(N)** — leading `%` defeats B-tree | **O(K)** via reverse-index + `keys_with_suffix` (Fix C) |
| 7 | `SELECT key FROM eq_eq_default_kv WHERE key LIKE '%-chunk-%'` | `keys_like("%-chunk-%")` | **O(N)** — leading `%` | Already a soft target; out of iteration-02 scope (separate `chunks` index needed) |

(Item 6 is the documented "did not fix" in [BRUTAL_ASSESSMENT.md §1](./BRUTAL_ASSESSMENT.md). Iteration 02 closes this gap.)

## 4. O(N) blast-radius matrix

| Endpoint | Trigger | Calls | Rows touched per call (today) |
|----------|---------|-------|-------------------------------|
| `GET /health` | k8s probes + UI poll | every 5–30 s | `_sqlx_migrations` only (after phase 1) — bounded |
| `GET /api/v1/workspaces/{id}/stats` | UI dashboard poll | every 30 s | `keys_like("%-metadata")` + `keys_like("%-chunk-%")` = **2 × full kv scan** |
| `GET /api/v1/graph/stream` | UI graph load | once per page view | `node_count` + `edge_count` = **2 × full vertex/edge scan** |
| `GET /api/v1/graph/stats` | UI graph dashboard | every 30 s | same as above |
| `GET /api/v1/graph/popular` | UI suggestions | once per page | `node_count` |

Multiplied by tenants × workspaces × users polling concurrently, this is precisely the access pattern that produced the original 13 s incident.

## 5. Where the existing fixes are correct

To avoid wasted work, the following remain valid and need **no further change** in this iteration:

- KV `count()` primary path → reads `eq_{prefix}_kv_stats.row_count` (O(1)). Verified at [`kv.rs#L388–L404`](../../edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs).
- KV `ping()` → `SELECT 1 FROM kv LIMIT 1` (O(1)). Verified at [`kv.rs#L416–L426`](../../edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs).
- Vector `is_empty()` → `EXISTS (... LIMIT 1)` (O(1)). Verified at [`vector.rs#L663`](../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs).
- Vector `ping()` → `SELECT 1 FROM vectors LIMIT 1` (O(1)). Verified at [`vector.rs#L692`](../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs).
- KV batch upsert via `unnest` (O(1) round-trip per 1 000 rows). Verified at [`kv.rs#L320–L348`](../../edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs).
- Shared `PostgresPool::from_existing` in `AppState`.

These are kept untouched; iteration 02 only **adds** new methods and new indexes, never removes the proven-correct ones.

## 6. What iteration 02 commits to fix

Concrete deliverables (each implemented and gated by a non-regression test):

- **Fix A** — `PgVectorStorage::count()` becomes O(1) via maintained counter table `eq_{prefix}_vectors_stats`, mirroring the KV pattern. New method `PgVectorStorage::ensure_row_count_stats`. Trigger-driven counters survive `upsert`, `delete`, `delete_entity*`, `clear`, `clear_workspace`.
- **Fix B** — Add trait methods `GraphStorage::node_count_fast(&self) -> Result<usize>` and `GraphStorage::edge_count_fast(&self) -> Result<usize>` returning a **best-effort estimate** (Postgres adapter reads `pg_class.reltuples` from the AGE `_ag_label_vertex` / `_ag_label_edge` relations; memory/mock adapters delegate to the exact counts which are already O(1)). Wire `graph_stream`, `graph/popular`, `graph/traversal` to the `_fast` variants. Exact `node_count`/`edge_count` remain unchanged for tests and admin tools.
- **Fix C** — Add trait method `KVStorage::keys_with_suffix(suffix: &str) -> Result<Vec<String>>`. Postgres adapter creates an expression index `CREATE INDEX IF NOT EXISTS … ON kv (reverse(key) text_pattern_ops)` once, and serves the method as `SELECT key FROM kv WHERE reverse(key) LIKE reverse($1) || '%'` → B-tree range scan. Memory adapter filters in-process.
- **Fix D** *(optional)* — Cache `SchemaHealth` in `AppState` (refresh every 60 s) so `/health` stops touching `_sqlx_migrations` on every probe. Deferred to next iteration unless trivial.

Migration to the new methods on dashboard hot paths:

- [`workspaces/stats.rs#L160`](../../edgequake/crates/edgequake-api/src/handlers/workspaces/stats.rs): replace `keys_like("%-metadata")` with `keys_with_suffix("-metadata")`.
- [`graph_stream.rs#L82`](../../edgequake/crates/edgequake-api/src/handlers/graph/graph_stream.rs): replace `node_count` / `edge_count` with `node_count_fast` / `edge_count_fast`.
- [`graph_query/popular.rs#L33`](../../edgequake/crates/edgequake-api/src/handlers/graph/graph_query/popular.rs): same.
- [`graph_query/traversal.rs#L260–L261`](../../edgequake/crates/edgequake-api/src/handlers/graph/graph_query/traversal.rs): same.

## 7. Hard non-regression contract

For each fix, the following must hold:

1. The exact-cardinality public method (`count`, `node_count`, `edge_count`, `keys_like`) **keeps its exact semantics** — proven by reusing the existing test suites (`provider_storage_compat`, `e2e_storage_backends`, `dimension_migration`, `e2e_dashboard_stats_issue81`).
2. New `*_fast` / `*_with_suffix` methods are covered by **new** dedicated tests that assert: (a) correctness against a seeded dataset, (b) latency budget under load (memory backend, since CI lacks Postgres).
3. Existing E2E SLO test `e2e_storage_performance_spec011` continues to pass unchanged.
4. `cargo clippy -p edgequake-storage -p edgequake-api --features postgres -- -D warnings` stays clean.

If any of these break, the change is reverted (each commit is independently revertable, no schema rollback needed — every new index uses `CREATE INDEX IF NOT EXISTS`).

## 8. What this iteration explicitly does **not** address

Following the discipline from BRUTAL_ASSESSMENT.md, the following remain known gaps tracked for a future iteration; no claim of fix is made here:

- `keys_like("%-chunk-%")` still scans linearly even with the reverse index (suffix `chunk-%` is an interior wildcard). Long-term fix is a dedicated `chunks` normalized table.
- `node_count_by_workspace` (Cypher per-workspace) still O(N) in workspace data — needs an indexed property column on the AGE vertex table.
- PDF list COUNT queries ([`pdf_storage_impl.rs`](../../edgequake/crates/edgequake-storage/src/adapters/postgres/pdf_storage_impl.rs)) are paginated and bounded by workspace filter; out of scope until a separate perf incident occurs.
- `keys()` calls in `main.rs`, `orchestrator/deletion.rs`, `pdf_processing.rs`, `injection.rs`, `tasks.rs`, `workspace_crud.rs`, `recovery/*` — already enumerated in [IMPLEMENTATION_PROOF.md](./IMPLEMENTATION_PROOF.md) "Remaining `keys()` call sites".
