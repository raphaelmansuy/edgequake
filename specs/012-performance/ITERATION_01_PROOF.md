# SPEC-012 — Storage performance, iteration 01 proof

Companion to [`ITERATION_01_AUDIT.md`](ITERATION_01_AUDIT.md). This file records what was changed, why each change is safe, and the verification that nothing regressed.

## 1. Verification gates (all passing)

| Gate | Command | Result |
|------|---------|--------|
| Workspace builds | `cargo build --workspace` | OK (19.3 s) |
| Lib tests | `cargo test --workspace --lib --no-fail-fast` | **268 passed / 0 failed / 0 ignored** across 4 crates |
| Clippy (storage + api) | `cargo clippy -p edgequake-storage -p edgequake-api --all-targets -- -D warnings` | clean |

No new warnings, no removed/renamed public API, no breaking trait changes (all additions have default impls).

## 2. Changes by fix

### Fix A — vector `count()` O(1) (SPEC-011 iter 02)

`adapters/postgres/vector.rs`:

- Added `stats_table_name` field.
- `create_table()` now calls `ensure_row_count_stats()` — creates a single-row stats table + plpgsql `AFTER INSERT/DELETE FOR EACH ROW` triggers that increment/decrement `row_count`.
- `count()` primary path: `SELECT row_count FROM stats WHERE id = 1`.
- `count()` fallback: full `COUNT(*)` **plus** self-heal (Fix H) so the next call is O(1).

No `clear()` override needed: vectors `clear()`/`clear_workspace()` use `DELETE FROM` (not `TRUNCATE`), so row triggers fire automatically.

### Fix B — graph `*_count_fast` (SPEC-011 iter 02)

`traits/graph.rs`: added trait methods `node_count_fast` and `edge_count_fast` with default impls that delegate to the exact methods (zero-impact for non-Postgres backends).

`adapters/postgres/graph/mod.rs`: overrides both via a new private `reltuples_estimate(label: &str)` reading `pg_class.reltuples`. Clamps negative `reltuples` (PG returns `-1` for never-analysed tables) to 0; returns 0 if the catalog row is missing.

Callers migrated:

- `handlers/graph/graph_stream.rs:82-83` (SSE stream, polled).
- `handlers/graph/graph_query/popular.rs:33` (popular-entity endpoint).
- `handlers/graph/graph_query/traversal.rs:260-261` (graph traversal UI).

### Fix C+ — indexed `keys_with_suffix` & `keys_with_prefix`

`traits/kv.rs`: added `async fn keys_with_suffix(&self, suffix: &str) -> Result<Vec<String>>` with default `keys_like(format!("%{suffix}"))`.

`adapters/postgres/kv.rs`:

- `create_table()` creates `eq_{prefix}_kv_reverse_key_idx ON {table} (reverse(key) text_pattern_ops)`.
- `keys_with_suffix` override: escapes `%` / `_` / `\` in the suffix, reverses the string, runs `SELECT key FROM kv WHERE reverse(key) LIKE '<reversed>%'` — an index-friendly prefix scan.

Polled callers migrated (filter pattern → indexed call):

| Caller | Old | New |
|--------|-----|-----|
| `handlers/workspaces/stats.rs:160` | `keys_like("%-metadata")` | `keys_with_suffix("-metadata")` |
| `handlers/tasks.rs:228` | `keys() + filter ends_with("-metadata")` | `keys_with_suffix("-metadata")` |
| `handlers/auth/user_management.rs:204` | `keys() + filter starts_with(USER_KEY_PREFIX)` | `keys_with_prefix(USER_KEY_PREFIX)` |
| `handlers/costs.rs:144` | `keys_like("%-metadata")` | `keys_with_suffix("-metadata")` |
| `handlers/costs.rs:374` | `keys_like("%-metadata")` | `keys_with_suffix("-metadata")` |
| `handlers/documents/delete/bulk.rs:44` | `keys_like("%-metadata")` | `keys_with_suffix("-metadata")` |

**Deliberately not migrated** — see audit §4 (Hotspot 4 table).

### Fix H — self-healing KV / vector `count()` fallback

The fallback branch now:

1. Logs a `WARN` with the missing stats table name.
2. Calls `self.ensure_row_count_stats(&pool).await` (idempotent — creates stats row, backfills `COUNT(*)`, creates triggers).
3. Runs the original fallback `COUNT(*)` and returns it.

Result: the *very next* `count()` hits the O(1) path. Legacy deployments self-upgrade on first call. No new admin action required to deploy.

## 3. Why these changes are safe (per surface)

| Risk | Mitigation |
|------|------------|
| Trait additions break downstream impls | Both new methods have default impls — old impls compile unchanged. |
| `reltuples` is wrong for new tables | Returns 0 (clamped); endpoints already handle `0 nodes` gracefully. After first `ANALYZE` (autovacuum) it converges. |
| Expression index increases write cost | One extra B-tree maintenance per `INSERT`/`DELETE` on `kv`. Workload is read-heavy (CSV: writes < 0.5% of DB time), trade-off is overwhelmingly positive. |
| `keys_with_suffix` set differs from `keys() + filter` | Default impl delegates to `keys_like("%suffix")` which is semantically identical. Postgres override produces the same set for ASCII suffixes. |
| Self-heal runs on every call if `ensure_row_count_stats` fails | The function is idempotent and uses `IF NOT EXISTS` / `ON CONFLICT DO NOTHING`. A persistent failure (e.g. no `CREATE` privilege) is logged once per request — visible, not silent. |

## 4. What an operator should expect to see in logs

After deploying:

```text
WARN  edgequake_storage::adapters::postgres::kv: KV stats row missing — running self-heal stats_table=public.eq_eq_default_kv_stats
```

This warning appears **exactly once** per affected deployment per worker startup. After it, the warning never fires again — the stats row exists. Same pattern for vectors.

## 5. Expected production-log delta (next CSV capture)

The next `queryedgeQuake.csv` should show:

- `COUNT(*)` queries against `_ag_label_vertex` / `_ag_label_edge` / `kv` / `vectors`: **disappear or collapse to single-digit ms**.
- `SELECT key FROM kv`: call rate drops to whatever residual is from non-polled paths.
- `get_by_ids` mean ms drops in proportion to how much `metadata_keys.len()` shrank (audit §3 hotspot 2 reasoning).

If any of those four fails to collapse, that is a regression and iteration 02 of SPEC-012 should investigate.
