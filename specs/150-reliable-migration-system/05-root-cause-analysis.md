# 05 — Root-cause analysis (D-01..D-15)

Parent: [README](README.md) · Now: [04](04-current-architecture.md) · Incidents: [02](02-incident-catalogue.md) · Plan: [08](08-implementation-plan.md)

Each defect is a LAW-150 violation plus the incidents it caused or still allows.

## D-01 — Serving refuses before bind (crash-loop)

**Law:** 150-6. **Five whys:** API must not apply schema → exit 78 on pending → exit happens inside `AppState::new_postgres` → HTTP never binds → kubelet/compose restart forever.

**Code:** `main.rs:1140-1150`, bind after state construction.

**Incidents enabled:** every "schema behind" field upgrade, Helm post-hook (D-02).

**Fix direction:** WP-5 wait-mode: bind, `/live` 200, `/ready` 503 `schema_pending`, poll.

## D-02 — Helm migrate runs after new pods

**Law:** 150-2, 150-6. Helm: post-upgrade runs after resources; `--wait` waits for Ready **before** post-hooks ([hooks](https://helm.sh/docs/topics/charts_hooks/)).

**Code:** `deploy/kubernetes/helm/edgequake/templates/migrate-job.yaml:10-12`.

**Incidents:** no GitHub number; structural outage class on every chart upgrade.

**Fix:** WP-7 `pre-install,pre-upgrade` + wait-mode API + conditional postgres wait + `activeDeadlineSeconds`.

## D-03 — Compose has no migrate lifecycle

**Law:** 150-2. Compose `depends_on` without `service_completed_successfully` only waits for *start*, not migrate success ([startup order](https://docs.docker.com/compose/how-tos/startup-order/)).

**Fix:** WP-7 one-shot `migrate` service.

## D-04 — Serving still writes

**Law:** 150-2. "Verify-only" is true for `MIGRATOR.run` (`mod.rs:1260`) and for `execute_bootstrap_apply_sql` (`reconcile/mod.rs:19-24`). False for:

1. Checksum `UPDATE` during `run_postgres_migrations_inner` (always invoked from serving).
2. Spawned m040/m139/m140/m141 `raw_sql` (`mod.rs:1487-1515`, `m139.rs:35`).

**Incidents:** SPEC-083 X-03 class (DDL under traffic); SPEC-090 §8 unbounded reconcile.

**Fix:** WP-4.

## D-05 — Incomplete checksum fossil registry

**Law:** 150-1, 150-4. Eight post-ship edits; repair list omits 001 `9e44513e…` and 019 `7b544306…`. Repair is env-gated so even listed fossils fail in production until someone copies Makefile values.

**Incidents:** #195, 019 drift, #273/#275, SPEC-110, SPEC-111.

**Fix:** WP-2 auto-accept exact known hashes; env only for unknown.

## D-06 — SSOT is copy-pasted lists

**Law:** DRY / 150-4. `KNOWN_CHECKSUM_REPAIR_VERSIONS` in Rust, Makefile:1167, two test harnesses (one adds 150). `IRREVERSIBLE_DROP_VERSIONS` only in Rust. Contract test `contract_spec111_checksum_repair_wiring.rs` only twins Makefile↔Rust.

**Fix:** WP-1 `manifest.toml`.

## D-07 — Advisory lock too narrow and unbounded

**Law:** 150-5. sqlx `pg_advisory_lock` waits forever; covers apply only. Two `edgequake migrate` can interleave repair/reconcile.

**Fix:** WP-3 `pg_try_advisory_lock` + deadline wrapping the whole run.

## D-08 — No migrator lock_timeout / statement_timeout

**Law:** 150-5. Session baseline sets only idle-in-xact (`connection.rs:101-103`). Postgres: wait indefinitely for table locks ([§13.3.4](https://www.postgresql.org/docs/current/explicit-locking.html)). SPEC-083: 5s timeout in support/092 skipped ALTER, column missing, ingest storm.

**Fix:** WP-3 per-class timeouts + retry.

## D-09 — sqlx transaction vs CONCURRENTLY / inner COMMIT

**Law:** 150-3, 150-1. No file uses `-- no-transaction`. Inner `BEGIN`/`COMMIT` in 128/129/130/132/143/144 ends sqlx's txn early (sqlx issue class #1966). CONCURRENTLY illegal in a txn ([CREATE INDEX](https://www.postgresql.org/docs/current/sql-createindex.html)).

**Fix:** WP-6 lint; WP-3 honor `no_tx` from manifest; do not add inner COMMIT.

## D-10 — Data phase inside schema transactions

**Law:** 150-3. M156/M158 batch inside one `DO` + one sqlx txn + `statement_timeout=0`. M117–122 unbounded INSERT. SPEC-091 engine is separate but still skip-and-advance (#396).

**Incidents:** #363, SPEC-139, #396, soak abort 125.

**Fix:** WP-3/WP-4 move graph rewrites to batched jobs; do not mark schema success until batches complete **or** split marker vs job.

## D-11 — `/ready` is a boot snapshot

**Law:** 150-6. `readiness_blockers(&state.migration_bootstrap)` (`health.rs:567`). Out-of-band 038 apply stays 503.

**Fix:** WP-5 live re-read.

## D-12 — CI does not resemble production

**Law:** 150-7. `psql || true` (`postgres-integration.yml:139`). migration-guard without AGE (#273 would still pass). Seeds lack multi-workspace duplicate ids (M118). Empty graphs (M078 `->>>`). Test harness swallows migrate errors.

**Fix:** WP-8.

## D-13 — `support/` and marker dual-write

**Law:** 150-1. Versioned file can be a no-op marker; real DDL in support, re-run, not checksum-locked. support/083, 086, 092 documented as every-boot.

**Fix:** WP-1 classify markers vs jobs; WP-6 lock support bytes; WP-4 never run support on serve.

## D-14 — Image / CLI / API version skew

**Law:** 150-3 N-1 window undefined. #396: CLI 0.26.5 vs API 0.26.1. Boot treats `applied_max > embedded_max` as hard refuse (`mod.rs:1055-1059`) — rolling upgrade of mixed binaries is illegal today.

**Fix:** WP-5 compat window in manifest (old binary may serve while expand-only schema is ahead).

## D-15 — God module + dual migrators

`mod.rs` 2803 lines. Tests embed a second `sqlx::migrate!`. SQLite has a third path. `docker/init.sql` is a fourth schema.

**Fix:** WP-9 split modules; optional `edgequake-migrate` bin that **links the same crate**, not a fork.

## Defect × work package

```text
  D-01 D-11 D-14  --> WP-5
  D-02 D-03       --> WP-7
  D-04 D-13       --> WP-4
  D-05 D-06       --> WP-1 WP-2
  D-07 D-08 D-09 D-10 --> WP-3 WP-6
  D-12            --> WP-8
  D-15            --> WP-9
  speed           --> WP-10 after measurement
```
