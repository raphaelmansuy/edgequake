# Migration 078 — AGE Child Workspace Stats Indexes (SPEC-040 / #262)

Repairs Apache AGE graph read performance when workspace-scoped stats and popular-node queries hit **child label tables** (`"Node"` / `"EDGE"`) while legacy indexes were created on inheritance parents.

**Version:** 1.0.1 — 2026-07-03 (SPEC-041 / [#273](https://github.com/raphaelmansuy/edgequake/issues/273): invalid JSON operator typo fixed)

## Automatic deployment (local & CI)

Migration **078** is a standard sqlx migration — no manual step for normal dev/prod startup.

| Path | Behavior |
|------|----------|
| `make dev` / `make backend-bg` | Backend calls `migration_bootstrap::run_postgres_migrations()` → sqlx applies pending `078_age_child_workspace_stats.sql` |
| Fresh PostgreSQL | Same — runs on first API start after upgrade |
| Idempotent | `IF NOT EXISTS` index checks + `ANALYZE` safe to re-run |

**Verify applied:**

```bash
psql "$DATABASE_URL" -c "SELECT version, description, installed_on FROM _sqlx_migrations WHERE version = 78;"
```

**Startup complement:** `graph_lifecycle.rs` `ensure_graph_indexes()` creates `idx_edge_start_id_text` / `idx_edge_end_id_text` on new graphs (DRY with M078).

## Files

| File | Role |
|------|------|
| `migrations/078_age_child_workspace_stats.sql` | sqlx migration — child indexes + ANALYZE (transaction-safe) |
| `migrations/support/078/concurrent.sql` | Ops-only CONCURRENTLY build for large graphs (>100k nodes) |
| `specs/040-edgequake-issues/e2e/measure_graph_stats_perf.sh` | Performance proof script |
| `specs/040-edgequake-issues/e2e/explain_workspace_graph.sql` | EXPLAIN template |

## Production procedure (large graphs)

When vertex count exceeds ~100k **and** inline migration window is too long:

```bash
# 1. Ensure sqlx marker 078 is recorded (normal upgrade) OR apply marker first via deploy
# 2. Run concurrent index build OUTSIDE transaction:
psql "$DATABASE_URL" -f edgequake/migrations/support/078/concurrent.sql

# 3. Verify indexes + measure performance
./specs/040-edgequake-issues/e2e/measure_graph_stats_perf.sh
```

**Rollback (indexes only, no data loss):**

```sql
-- Per graph schema (example: eq_eq_default_graph)
DROP INDEX IF EXISTS eq_eq_default_graph.idx_node_workspace_id;
DROP INDEX IF EXISTS eq_eq_default_graph.idx_node_tenant_id;
DROP INDEX IF EXISTS eq_eq_default_graph.idx_edge_start_id_text;
DROP INDEX IF EXISTS eq_eq_default_graph.idx_edge_end_id_text;
```

## Performance measurement (2026-07-02 local proof)

Graph: **62,935 nodes**, **81,240 edges** (post-M078)

| Query | Execution time | Target |
|-------|----------------|--------|
| Workspace-filtered Node count (child table) | ~136 ms | < 15,000 ms |
| Degree join (Node + EDGE) | ~146 ms | < 15,000 ms |

Index usage: `idx_edge_start_id_text` / `idx_edge_end_id_text` — 62k+ scans after stats workload.

```bash
./specs/040-edgequake-issues/e2e/measure_graph_stats_perf.sh
```

## Related

- [migrations.md](../migrations.md) — bootstrap overview
- [SPEC-040 implementation plan](../../../specs/040-edgequake-issues/008-implementation-plan.md)
- GitHub [#262](https://github.com/raphaelmansuy/edgequake/issues/262) (closed in v0.13.2)
