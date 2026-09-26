# 04 — Current architecture (as-is)

Parent: [README](README.md) · Principles: [01](01-first-principles.md) · Defects: [05](05-root-cause-analysis.md)

sqlx **0.8.6** (`edgequake/Cargo.lock`). Embedded source: `sqlx::migrate!("../../migrations")` at [`mod.rs:277`](../../edgequake/crates/edgequake-api/src/state/migration_bootstrap/mod.rs). `migration_bootstrap/mod.rs` is **2803 lines**.

## Process entry

Binary: `edgequake` (`edgequake/src/main.rs`).

```text
edgequake argv[1]
  |
  +-- healthcheck | pre-stop ---- container_ops (no Tokio, no DB)
  |
  +-- migrate [...verbs...] ---- dispatch_migrate  (main.rs:986)
  |                                 sets EDGEQUAKE_MIGRATE_CLI=1
  |                                 PgPoolBundle + run_postgres_migrations
  |                                 verbs: (apply), dry-run, status, console,
  |                                        plan, guard, family, pause/resume/cancel
  |
  +-- (serve) ------------------- AppState::new_postgres
                                    bootstrap_for_serving (verify)
                                    workers...
                                    Server::new  <-- FIRST HTTP BIND (main.rs:1898)
```

There is **no** `EDGEQUAKE_SKIP_MIGRATIONS`. `EDGEQUAKE_ALLOW_BOOT_MIGRATE` is warn-and-ignore (`mod.rs:957-976`). `EDGEQUAKE_MIGRATE_CLI` is set only by the CLI (`migrate_cli_mode()`, `mod.rs:979`).

## Serving boot (verify-only for sqlx, not for all SQL)

```text
AppState::new_postgres                         postgres.rs:161-274
  PgPoolBundle (query/ingest/queue/admin)
    after_connect: application_name, search_path=public,
                   idle_in_transaction_session_timeout
                   NO statement_timeout, NO lock_timeout
                   connection.rs:83-106
  extension probe (vector, uuid-ossp) -- log only
  bootstrap_for_serving(admin_pool)            postgres.rs:259-260
    if applied_max > embedded_max -> BOOT_GATE_REFUSAL NEWER
    if pending not empty:
         pending_ok_to_serve? (only 125/126/131 and maybe 142)
           yes -> WARN, continue
           no  -> BOOT_GATE_REFUSAL STOP schema behind
    run_postgres_migrations_inner(All)
      checksum repair 071/078/118/121/125/131   << WRITES if allowed
      apply_sqlx = migrate_cli && pending nonempty
         else skip MIGRATOR.run
      reconcile hooks (execute_bootstrap_apply_sql no-op unless CLI)
      tokio::spawn m040 m139 m140 m141          << WRITES via raw_sql
      create_next_audit_log_partition (non-fatal)
  migration_engine::spawn_for_serving           SPEC-091 data copy (mode env)
  check_hnsw_index_manifest (warn)

main.rs:1140-1150
  BOOT_GATE_REFUSAL -> eprintln; exit 78
  other Err -> exit 1
  success -> bind later (~1897)
```

Boot-gate constants: `BOOT_GATE_EXIT_CODE = 78`, `IRREVERSIBLE_DROP_VERSIONS = [125,126,131]`, `LEGACY_CUTOVER_ASSERT_VERSION = 142` (`mod.rs:832-842`).

`pending_ok_to_serve`: every pending version is an irreversible drop, or 142 while legacy rows remain (`mod.rs:862-866`).

Checksum mismatch during **serving** is **not** sqlx `VersionMismatch` (no `MIGRATOR.run`). Drift surfaces on `edgequake migrate`. Unauthorized known-broken hashes fail serving with **exit 1**, not 78.

## How sqlx 0.8.6 applies (CLI only)

From `sqlx-postgres-0.8.6` / `sqlx-core-0.8.6`:

1. `pg_advisory_lock` derived from database name — **waits forever**.
2. Refuse `Dirty` version.
3. Validate applied checksums (`VersionMismatch` / `VersionMissing`).
4. Per file: transaction unless `-- no-transaction` on line 1; run SQL; `INSERT INTO _sqlx_migrations`.
5. Unlock.

EdgeQuake log line *"advisory lock held"* (`mod.rs:1274`) is true **only** inside `MIGRATOR.run`. Repair, census, reconcile, spawned backfills are outside.

## Reconcile / support SQL

`include_str!` of `migrations/support/NNN/*.sql` at `mod.rs:12-275`. `execute_bootstrap_apply_sql` is CLI-gated (`reconcile/mod.rs:15-28`).

**Bypass:** `reconcile_migration_139_background` (and 040/140/141) call `sqlx::raw_sql(SQL_*_APPLY).execute` directly (`m139.rs:35`), spawned whenever the version is in the ledger and a progress key is missing (`mod.rs:1487-1515`). support/140: `ALTER TABLE "EDGE"`, full `UPDATE`, unique index. support/141: DROP/ADD CHECK + VALIDATE.

`support/156-158` are **not** wired into the binary (ops copies; tests byte-compare).

## Health

| Route | Behavior |
|-------|----------|
| `/live` | Process up (container healthcheck `edgequake healthcheck`) |
| `/ready` | `readiness_blockers(&state.migration_bootstrap)` **boot snapshot** + storage ping + queue (`health.rs:563-572`) |
| `/health` | Live `schema_drift` with 750 ms bound (`health.rs:508`) |

Fixing indexes out-of-band (`apply_038.sh`) does **not** flip `/ready` until restart.

## Pools and timeouts

Admin pool max=2, acquire 30s. Session sets idle-in-xact only. Migrator has **no** `lock_timeout` / `statement_timeout` except inside some SQL files (156/157/158, support/086, support/092). `EDGEQUAKE_EQ_MAINTENANCE=1` raises support/092 lock_timeout 5s → 120s.

## Deployment

| Surface | Behavior |
|---------|----------|
| Distroless Dockerfile | `ENTRYPOINT edgequake`; copies migrations for `migrate!`; **no** auto-migrate |
| compose / quickstart / prebuilt | **No** migrate service |
| Helm Job | `helm.sh/hook: post-install,post-upgrade`, weight 5, `ttlSecondsAfterFinished: 600`, `backoffLimit: 3`, **no** `activeDeadlineSeconds` |
| Helm wait-for-postgres | Unconditional host `edgequake-postgres` (`migrate-job.yaml:23-32`) |
| API probes | readiness `/ready` (10s delay, 5 failures); liveness `/live` (15s); **no startupProbe** |
| Makefile `VISIBLE_MIGRATE_STEP` | `cargo run -- migrate` with allowlist 71,78,118,121,125,131 (`Makefile:1167-1187`) before backend start |

## Env (migration-adjacent)

| Variable | Effect |
|----------|--------|
| `DATABASE_URL` | Required |
| `EDGEQUAKE_MIGRATE_CLI` | Enables sqlx apply + support apply |
| `EDGEQUAKE_ALLOW_CHECKSUM_REPAIR` | Version allowlist for checksum UPDATE |
| `EDGEQUAKE_DEV_MODE` | Allows **all** checksum repairs (+ relaxes auth) |
| `EDGEQUAKE_MIGRATION_CONFIRM_DROP` | Same as `--confirm-drop` |
| `EDGEQUAKE_MIGRATION_MODE` (+ throttle/batch/lease) | SPEC-091 **data** engine, not schema |
| `EDGEQUAKE_MIGRATION_LARGE_GRAPH_THRESHOLD` | support/038 (default 500000) |
| `EDGEQUAKE_EQ_MAINTENANCE` | support/092 lock_timeout 120s |

## Tests and CI

| Asset | Gap |
|-------|-----|
| `contract_spec091_boot_gate.rs` | Scratch DBs; fake ledger; does not apply 156 files |
| `tests/common/test_db.rs` | Own `MIGRATOR.run`; own repair list; `eprintln!` on failure |
| storage `postgres_test_config.rs` | Also repairs **150** and deletes ledger `>= 150` — production allowlist has no 150 |
| `migration-checksum-guard` (`ci.yml:32-41`) | `check_migration_checksums.sh` — current bytes only |
| `migration-guard.yml` | `sqlx migrate run` on **pgvector:pg16, no AGE** |
| `postgres-integration.yml:137-139` | `psql -f "$file" \|\| true` — failures ignored, no ledger |

## SQLite (out of PostgreSQL train)

`edgequake-storage` SQLite applies 001–003 via `include_str!` + `raw_sql` every connect. No ledger. Relies on `IF NOT EXISTS`.

## Docs that lie

- `edgequake/docs/migrations.md` still claims bootstrap auto-apply on API start.
- `specs/138-kubernetes/12-lens-database.md` still says "SQLx on API boot".
- spec091 soak compose still sets `EDGEQUAKE_ALLOW_BOOT_MIGRATE=1`.
