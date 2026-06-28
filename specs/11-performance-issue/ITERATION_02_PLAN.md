# SPEC-011 — Iteration 02 Implementation Plan

> **Cross-refs**: [ITERATION_02_AUDIT.md](./ITERATION_02_AUDIT.md), [IMPROVEMENT_PLAN.md](./IMPROVEMENT_PLAN.md), [BRUTAL_ASSESSMENT.md](./BRUTAL_ASSESSMENT.md), [PERFORMANCE_GUARANTEE.md](./PERFORMANCE_GUARANTEE.md)
> **Branch**: `fix/spec-011-storage-performance`
> **Risk philosophy**: every change is **additive** (new method, new index, new trigger). No existing public method changes semantics. Each phase is independently revertable.

---

## Fix A — Vector `count()` becomes O(1)

### Design

Mirror the proven KV pattern in [`kv.rs#L115–L230`](../../edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs):

- New table `eq_{prefix}_vectors_stats(id smallint primary key default 1 check (id = 1), row_count bigint default 0)`.
- Trigger functions `eq_{prefix}_vectors_stats_insert/_delete` plpgsql, increment / decrement (`GREATEST(row_count - 1, 0)`).
- `AFTER INSERT FOR EACH ROW` and `AFTER DELETE FOR EACH ROW` triggers on the vectors table.
- `count()` reads the single counter row; fallback to `SELECT COUNT(*)` only if stats row missing.
- `clear()` runs `TRUNCATE` and resets the counter (mirrors KV).
- `delete_entity` and `delete_entity_relations` already issue `DELETE FROM vectors WHERE …` — row triggers fire, no extra code.
- `ensure_dimension` drops + recreates the table — must call `ensure_row_count_stats` afterwards.

### Algorithmic guarantee

`SELECT row_count FROM stats WHERE id = 1` → primary-key lookup, **O(1)** (single index tuple).

### Edge cases

| Edge case                                      | Mitigation                                                                                                                                          |
| ---------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| Existing deployment without stats table        | `ensure_row_count_stats` is idempotent (`CREATE TABLE IF NOT EXISTS`); backfill via `INSERT … SELECT COUNT(*) … ON CONFLICT DO NOTHING` runs once.  |
| `TRUNCATE` does not fire row triggers          | Explicit `UPDATE … SET row_count = 0` after truncate (mirrors KV `clear()`).                                                                        |
| `delete_entity` deletes by JSONB filter        | Row triggers fire per deleted row — counter stays correct.                                                                                          |
| `ensure_dimension` recreates table             | Call `ensure_row_count_stats(pool)` after `drop_table` + `create_table`.                                                                            |
| Concurrent `INSERT` / `DELETE` race            | `UPDATE` is row-level locked on the single stats row — serialised, correct count. Throughput impact negligible (one extra UPDATE per row, same TX). |
| `clear_workspace` deletes a subset             | Row triggers fire → counter decremented per row. Same behaviour as KV.                                                                              |
| Partial trigger failure on existing prod table | `CREATE OR REPLACE` on functions and `DROP TRIGGER IF EXISTS` before `CREATE TRIGGER` make initialise idempotent.                                   |

### Non-regression test plan

- Reuse `tests/provider_storage_compat.rs::count` and `e2e_storage_backends.rs::count` (Postgres-gated) — exact-count contract.
- Add `tests/performance_storage.rs::vector_count_is_constant_time` — seed 10k vectors, assert `count()` < 10 ms (memory backend stays fast; Postgres gated by `POSTGRES_PASSWORD`).

---

## Fix B — Graph fast counts (estimate) for polling endpoints

### Design

Exact `node_count` / `edge_count` are correct and used by tests; keep them.  
Add **new** trait methods with explicit "_fast" / estimate semantics:

```rust
#[async_trait]
pub trait GraphStorage: Send + Sync {
    // … existing methods …

    /// Best-effort node count (estimate). O(1) on Postgres via planner stats.
    /// Adapters without an estimate source MUST delegate to `node_count`.
    async fn node_count_fast(&self) -> Result<usize> { self.node_count().await }

    /// Best-effort edge count (estimate). Same semantics as `node_count_fast`.
    async fn edge_count_fast(&self) -> Result<usize> { self.edge_count().await }
}
```

Postgres override (AGE adapter):

```sql
SELECT GREATEST(0, COALESCE(c.reltuples, 0))::bigint
FROM pg_class c
JOIN pg_namespace n ON n.oid = c.relnamespace
WHERE n.nspname = $1 AND c.relname = $2
```

with `$1 = self.graph_name`, `$2 = '_ag_label_vertex'` (resp. `_ag_label_edge`).  
On missing stats (`reltuples = 0` but table not empty), fall back to the exact `node_count()` once and trust the next autovacuum.

### Algorithmic guarantee

`pg_class` is an in-memory catalog row; lookup by `(nspname, relname)` is **O(1)**. No heap touched.

### Edge cases

| Edge case                                    | Mitigation                                                                                                           |
| -------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| `reltuples = -1` (never analysed)            | Treat as unknown → run exact `node_count()` once (rare; only on first deploy).                                       |
| Estimate drifts after large delete           | Acceptable for dashboard ("~ 12k entities" is informational). Autovacuum refreshes on threshold.                     |
| Estimate > 0 but table empty after `clear()` | UI may briefly show stale count; pair with `clear()` triggering `ANALYZE` in the AGE adapter (cheap on empty table). |
| AGE not loaded                               | Returns 0 — same as today's `node_count` failure mode (caller already uses `unwrap_or(0)` in graph_stream).          |
| Schema name with quotes/specials             | `nspname` is bound as parameter — safe.                                                                              |

### Non-regression test plan

- Add `tests/graph_count_fast.rs` (memory backend) — default impl delegates to exact; identical results.
- Add Postgres-gated `tests/e2e_storage_backends.rs::node_count_fast_returns_estimate` — seed graph, assert fast count is within ±10 % of exact, latency < 5 ms.

---

## Fix C — `keys_with_suffix` + reverse-index for workspace stats

### Design

Add trait method:

```rust
#[async_trait]
pub trait KVStorage {
    /// Return keys ending with `suffix` (index-friendly on Postgres).
    /// Default impl filters in-process; Postgres adapter uses a reverse-string
    /// expression index for B-tree range scan.
    async fn keys_with_suffix(&self, suffix: &str) -> Result<Vec<String>> {
        let pattern = format!("%{suffix}");
        self.keys_like(&pattern).await
    }
}
```

Postgres adapter:

1. In `create_table`, add `CREATE INDEX IF NOT EXISTS eq_{prefix}_kv_reverse_key_idx ON {table} (reverse(key) text_pattern_ops)`.
2. Override `keys_with_suffix` with:

```sql
SELECT key FROM {table} WHERE reverse(key) LIKE $1
-- $1 = format!("{}%", suffix.chars().rev().collect::<String>())
```

Postgres uses the reverse expression index → B-tree range scan, O(K + log N).

### Algorithmic guarantee

`reverse('foo-metadata') = 'atadatem-oof'` → query becomes `WHERE reverse(key) LIKE 'atadatem-%'` → prefix scan on the reverse-key index. **O(K + log N)** where K = matches.

### Edge cases

| Edge case                         | Mitigation                                                                                                                                                                                        |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Suffix contains `%` or `_`        | Escape before reverse: replace `%` → `\%`, `_` → `\_`. Suffix is server-controlled today (`"-metadata"`, `"-chunk-summary"`), but escape defensively.                                             |
| Index missing on existing prod DB | `CREATE INDEX IF NOT EXISTS` at adapter init; falls back to `LIKE '%suffix'` (existing slow path) until index built.                                                                              |
| Index build on large table        | `CREATE INDEX` blocks writes briefly; for prod with > 1 M rows, switch to `CREATE INDEX CONCURRENTLY` in a follow-up migration (out of scope — Memory and small Postgres deployments unaffected). |
| Memory adapter                    | Default impl scans keys + filters — acceptable for tests and small workloads.                                                                                                                     |
| Multi-byte / UTF-8 keys           | Postgres `reverse()` is byte-aware on ASCII; current keys are ASCII (UUIDs + ASCII suffixes). Document the constraint in the trait doc.                                                           |

### Caller migration

[`workspaces/stats.rs#L160`](../../edgequake/crates/edgequake-api/src/handlers/workspaces/stats.rs):

```rust
// before
let keys = kv.keys_like("%-metadata").await?;
// after
let keys = kv.keys_with_suffix("-metadata").await?;
```

Behaviour is identical; method name documents intent and unlocks the indexed query path.

`keys_like("%-chunk-%")` is **not** a suffix query (interior wildcard); it stays on the slow path and is documented as a known gap in [ITERATION_02_AUDIT.md §8](./ITERATION_02_AUDIT.md).

### Non-regression test plan

- `tests/provider_storage_compat.rs::keys_with_suffix_matches_keys_like` — every backend returns the same set as the old `keys_like("%suffix")` call for a seeded dataset.
- `tests/performance_storage.rs::keys_with_suffix_is_sublinear` — memory backend correctness; Postgres gated by env.

---

## Hot-path caller migrations

| File                                                                                                                          | Before                                      | After                                                 |
| ----------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------- | ----------------------------------------------------- |
| [`handlers/workspaces/stats.rs`](../../edgequake/crates/edgequake-api/src/handlers/workspaces/stats.rs)                       | `kv.keys_like("%-metadata")`                | `kv.keys_with_suffix("-metadata")`                    |
| [`handlers/graph/graph_stream.rs`](../../edgequake/crates/edgequake-api/src/handlers/graph/graph_stream.rs)                   | `graph.node_count()` / `graph.edge_count()` | `graph.node_count_fast()` / `graph.edge_count_fast()` |
| [`handlers/graph/graph_query/popular.rs`](../../edgequake/crates/edgequake-api/src/handlers/graph/graph_query/popular.rs)     | `graph.node_count()`                        | `graph.node_count_fast()`                             |
| [`handlers/graph/graph_query/traversal.rs`](../../edgequake/crates/edgequake-api/src/handlers/graph/graph_query/traversal.rs) | `graph.node_count()` / `graph.edge_count()` | `graph.node_count_fast()` / `graph.edge_count_fast()` |

`traversal.rs` returns the values to the user — switching to estimates is acceptable because the call already accepts `unwrap_or(0)`-style failures and the values are displayed as "approximate totals". This is documented in the response field comment.

## Non-regression gates

```bash
# unit
cargo test -p edgequake-storage --lib

# compat (memory + postgres-gated)
cargo test -p edgequake-storage --test provider_storage_compat

# e2e backends (postgres-gated)
cargo test -p edgequake-storage --test e2e_storage_backends

# SLO
cargo test -p edgequake-storage --test performance_storage
cargo test -p edgequake-api --test e2e_storage_performance_spec011

# dashboard stats
cargo test -p edgequake-api --test e2e_dashboard_stats_issue81

# lint
cargo clippy -p edgequake-storage -p edgequake-api --features postgres -- -D warnings
```

All must pass before commit. If Postgres tests are skipped (no `POSTGRES_PASSWORD`), the memory-backend tests still cover the trait semantics; Postgres-specific tests run in the integration job.

## Rollback

Each fix is a self-contained commit:

1. `feat(storage): vector O(1) count via maintained counter (SPEC-011 iter02 Fix A)` — revert removes the stats table init + counter triggers; `count()` reverts to raw `COUNT(*)`. No data loss.
2. `feat(graph): node_count_fast / edge_count_fast estimates (Fix B)` — revert removes the new trait methods + callers; reverts to `node_count` / `edge_count`. Tests unaffected.
3. `feat(storage): keys_with_suffix + reverse-key index (Fix C)` — revert removes the index + new method; `workspaces/stats` reverts to `keys_like`. Zero data risk.

No database migration is required to revert — every new object is created with `IF NOT EXISTS`, and the old code path still works alongside the new objects.
