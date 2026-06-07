# Migration Bootstrap — First Principles

**Decision:** Integrate migration orchestration in **EdgeQuake bootstrap** (`migration_bootstrap.rs`), not a separate tool. "Edgecrab" in prior specs refers to the EdgeQuake Rust backend audit scope.

## Why bootstrap integration is correct

```
PeakRisk(bootstrap) = f(blocking_work, graph_size, concurrency)
```

| Approach | Fresh install | Existing small DB | Existing large DB |
|----------|---------------|-------------------|-------------------|
| sqlx only | ✅ | ✅ | ⚠ blocking `CREATE INDEX` |
| Ops script only | ❌ manual | ❌ easy to skip | ✅ CONCURRENTLY |
| **Bootstrap + ops** | ✅ auto | ✅ verify/repair | ✅ defer + guide |

**Conclusion (P5):** sqlx migration 038 is a **marker only** (no blocking DDL). Bootstrap runs size-aware `support/038/apply.sql`; large graphs defer; `/ready` returns 503 until indexes exist or ops completes CONCURRENTLY apply.

## SOLID layout

| Component | Responsibility |
|-----------|----------------|
| `migration_bootstrap.rs` | Orchestrate sqlx + 038 reconcile (SRP) |
| `migrations/*.sql` | SQL SSOT (DRY via `include_str!`) |
| `apply_038.sh` | Ops path for CONCURRENTLY / rollback |
| `/health` schema | Operator visibility (DIP) |

## Edge cases

| Case | Bootstrap behavior |
|------|-------------------|
| No pending migrations | Log "up to date", still verify 038 |
| 038 pending + small graph | sqlx applies + verify |
| 038 pending + large graph | sqlx may apply (blocking); post-hook warns if incomplete |
| Partial index failure | Inline repair if small; else degraded + ops command |
| No AGE | Skip 038 verify, `indexes_ready: true` |
| Empty graph catalog | Nothing to index, ready |
| Multi-instance start | sqlx advisory lock serializes |
| Restart after defer | Health stays degraded until ops completes CONCURRENTLY |

## Operator signals

1. **Logs** — `target=edgequake.migration`, steps: `preflight`, `pending`, `applied`, `migration_038_*`
2. **Health** — `GET /health` → `schema.source_ids_indexes.ready`
3. **Script** — `operator_action` field points to `apply_038.sh --apply --concurrent --yes`
