# 13 — References

Parent: [README](README.md)

Fetched 2026-09-26. Quotes are abbreviated; follow the URL for the full text.

## sqlx

- Migrator 0.8.6: <https://docs.rs/sqlx/0.8.6/sqlx/migrate/struct.Migrator.html>  
  `run`: *"Run any pending migrations … and, validate previously applied migrations against the current migration source to detect accidental changes."*  
  `set_locking`: *"Disabling locking can lead to errors or data loss if multiple clients attempt to apply migrations simultaneously."*
- `-- no-transaction`: sqlx 0.8.0 changelog [#3181](https://github.com/launchbadge/sqlx/pull/3181) — *"migrations that should not run in a transaction can be flagged by adding `-- no-transaction` to the beginning."* Implemented in `sqlx-core-0.8.6/src/migrate/source.rs` (`sql.starts_with("-- no-transaction")`).
- sqlx 0.9.0 (2026-05-06): <https://github.com/launchbadge/sqlx/blob/main/CHANGELOG.md> — `sqlx.toml` can *"Set characters to ignore when hashing migrations (e.g. ignore whitespace)"*; breaking `Migrate` trait / MSRV 1.94. **Not adopted** in SPEC-150 engine ([06](06-target-architecture.md)).
- EdgeQuake pin: `edgequake/Cargo.lock` `name = "sqlx"` `version = "0.8.6"`.

## PostgreSQL

- Explicit locking: <https://www.postgresql.org/docs/current/explicit-locking.html>  
  ACCESS EXCLUSIVE *"guarantees that the holder is the only transaction accessing the table in any way."*  
  CREATE INDEX (no CONCURRENTLY) acquires **SHARE**.  
  Deadlocks: *"a transaction seeking either a table-level or row-level lock will wait indefinitely."*  
  Advisory locks: session-level *"do not honor transaction semantics"*; held until release or session end.
- CREATE INDEX CONCURRENTLY: <https://www.postgresql.org/docs/current/sql-createindex.html> — builds without locking out writes; two scans; **"cannot be executed within a transaction block"** (manual; confirmed by PG error `25001` in operations).
- ALTER TABLE `NOT VALID` / `VALIDATE CONSTRAINT`: <https://www.postgresql.org/docs/current/sql-altertable.html>
- SET vs SET LOCAL: <https://www.postgresql.org/docs/current/sql-set.html> — LOCAL lasts only until transaction end.
- Client timeouts (`lock_timeout`, `statement_timeout`, `idle_in_transaction_session_timeout`): <https://www.postgresql.org/docs/current/runtime-config-client.html>

## Orchestration

- Helm chart hooks: <https://helm.sh/docs/topics/charts_hooks/>  
  `pre-install`: after render, **before** resources created.  
  `pre-upgrade`: after render, **before** resources updated.  
  `post-install`: after resources loaded.  
  *"`--wait` … will not run the post-install hook until they are ready."*
- Kubernetes probes: <https://kubernetes.io/docs/tasks/configure-pod-container/configure-liveness-readiness-startup-probes/>  
  Liveness failure **kills** the container. Readiness failure removes endpoints. Startup probe covers slow init.
- Kubernetes Jobs: <https://kubernetes.io/docs/concepts/workloads/controllers/job/> — `activeDeadlineSeconds`, `backoffLimit`.
- Compose startup order: <https://docs.docker.com/compose/how-tos/startup-order/>  
  `service_completed_successfully`: *"dependency is expected to run to successful completion before starting a dependent service."*

## Industry migration practice

- GitLab Migration Style Guide: <https://docs.gitlab.com/development/migration_style_guide/>  
  *"Migrations are not allowed to require GitLab installations to be taken offline ever."*  
  Types: regular schema (before new code, **≤ 3 minutes** guideline), post-deploy (≤ 10 minutes), batched background (data, not schema).  
  Background: *"any single query must stay below 1 second execution time with cold caches."*

## Extensions

- Apache AGE overview: <https://age.apache.org/age-manual/master/intro/overview.html> — graph extension; Cypher + SQL. EdgeQuake graphs live in `ag_catalog`; Node/EDGE tables are where M038/M078/M156 hurt.
- pgvector: <https://github.com/pgvector/pgvector> — HNSW / ivfflat; dimension limits drove #275 / M071 halfvec. HNSW create is a SHARE-class index build.

## In-repo (binding)

| Path | Role |
|------|------|
| [`edgequake/crates/edgequake-api/src/state/migration_bootstrap/mod.rs`](../../edgequake/crates/edgequake-api/src/state/migration_bootstrap/mod.rs) | Gate, sqlx embed, spawn |
| [`checksum_repair.rs`](../../edgequake/crates/edgequake-api/src/state/migration_bootstrap/checksum_repair.rs) | Allowlist 71,78,118,121,125,131 |
| [`edgequake/src/main.rs`](../../edgequake/src/main.rs) | `dispatch_migrate`, exit 78 |
| [`postgres.rs`](../../edgequake/crates/edgequake-api/src/state/postgres.rs) | `bootstrap_for_serving`, `spawn_for_serving` |
| [`health.rs`](../../edgequake/crates/edgequake-api/src/handlers/health.rs) | `/ready` snapshot |
| [`connection.rs`](../../edgequake/crates/edgequake-storage/src/adapters/postgres/connection.rs) | search_path, idle-in-xact |
| [`migrate-job.yaml`](../../deploy/kubernetes/helm/edgequake/templates/migrate-job.yaml) | post-install hook |
| [`Makefile`](../../Makefile) L1167–1187 | migrate allowlist |
| [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml) | checksum guard |
| [`.github/workflows/postgres-integration.yml`](../../.github/workflows/postgres-integration.yml) | `psql \|\| true` |
| [`edgequake/migrations/checksums.lock`](../../edgequake/migrations/checksums.lock) | current SHA-384 |
| [`docs/operations/spec091-upgrade-from-v0.22.0.md`](../../docs/operations/spec091-upgrade-from-v0.22.0.md) | expand/drop ladder |
| Prior specs | [041](../041-fix-migration/000-index.md), [93](../93-migration-assessment/README.md), [110](../110-migration-issue/README.md), [111](../111-issues/10-migration-immutability.md), [137](../137-issue-migration-25-to-26/README.md), [139](../139-issue-migration/README.md) |

## GitHub issues (canonical)

#195 checksum 001 · #273 M078 `->>>` · #275 HNSW dims · #280 PG18 volume · #288 auth backfill · #362 residue timeout · #363 silent drop · #364 drop gate · #374/#377/#383 unique/stale · #396 guard RED (open) · #405 FTS 42P01 (open)
