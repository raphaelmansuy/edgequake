# Database Migrations

EdgeQuake uses [SQLx](https://github.com/launchbadge/sqlx) embedded migrations.
**Schema is applied only by `edgequake migrate`** (SPEC-091 LD-15 / SPEC-150).
The API process never auto-applies numbered migrations on start.

## Quick Reference

| Task | Command |
|------|---------|
| Fresh dev stack | `make dev` (runs `edgequake migrate` before backend start) |
| Apply schema | `edgequake migrate` / `edgequake migrate --confirm-drop` |
| Drain data jobs | `edgequake migrate drain` |
| Preview | `edgequake migrate dry-run` |
| Check immutability | `./scripts/check_migration_checksums.sh` |
| Manifest SSOT | `edgequake/migrations/manifest.toml` |

## How Migrations Work

1. **Numbered SQL files** — `NNN_description.sql` in `edgequake/migrations/`
2. **Explicit CLI apply** — `edgequake migrate` (admin pool + reconcile)
3. **Manifest** — phases, fossils, irreversible drops, compat window (SPEC-150)
4. **Immutability lock** — `checksums.lock` is append-only; never edit shipped bodies
5. **Support scripts** — `migrations/support/` ops-only SQL (also locked)

### Serving vs migrate

| Process | Writes schema? | Behavior when behind |
|---------|----------------|----------------------|
| `edgequake migrate` | Yes | Applies expandables; gates drops behind `--confirm-drop` |
| API (`edgequake`) | No | `EDGEQUAKE_SCHEMA_GATE=fail` → exit 78; `wait` → lite `/live` + `/ready` 503 |

**Env vars (SPEC-150):**

| Variable | Default | Purpose |
|----------|---------|---------|
| `EDGEQUAKE_SCHEMA_GATE` | `fail` | `wait` binds lite router until migrate catches up |
| `EDGEQUAKE_SCHEMA_GATE_POLL` | `2` | Poll seconds in wait mode |
| `EDGEQUAKE_MIGRATE_LOCK_DEADLINE` | `60` | Advisory lock wait (exit 75) |
| `EDGEQUAKE_MIGRATE_LOCK_TIMEOUT` | `5s` | Session `lock_timeout` |
| `EDGEQUAKE_MIGRATE_STATEMENT_TIMEOUT` | (per class) | Session `statement_timeout` |
| `EDGEQUAKE_ALLOW_CHECKSUM_REPAIR` | unset | Emergency unknown-hash override |
| `EDGEQUAKE_SERVE_RECONCILE` | unset | One-release escape for serve-time support DDL |
| `EDGEQUAKE_MIGRATION_LARGE_GRAPH_THRESHOLD` | `500000` | Defer inline index repair |

See `specs/150-reliable-migration-system/` for the full design.


## Golden path (upgrade)

```bash
edgequake migrate dry-run
edgequake migrate
edgequake migrate drain --timeout 3600   # optional data cutover
edgequake migrate --confirm-drop         # only when guard GREEN
curl -sf localhost:8080/ready
```

Full runbook: [`specs/150-reliable-migration-system/11-ops-runbook.md`](../../specs/150-reliable-migration-system/11-ops-runbook.md).
Epoch proof: `make spec150-matrix-quick`.
