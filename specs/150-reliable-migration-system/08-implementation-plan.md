# 08 — Implementation plan (WP-1..WP-10)

Parent: [README](README.md) · Target: [06](06-target-architecture.md) · Defects: [05](05-root-cause-analysis.md) · Proof: [09](09-test-proof-protocol.md)

Documents-only pack: these WPs are the **sure** build sequence. Do not squash (WP-10) before WP-2 and WP-8 exist.

```text
  WP-1 manifest SSOT
    -> WP-2 fossil checksums (unblocks 001/019)
    -> WP-3 runner lock/timeout/telemetry
    -> WP-4 serving never writes
    -> WP-5 wait-mode + live /ready
    -> WP-6 authoring lint + append-only lock
    -> WP-7 Helm/compose/ECS wiring
    -> WP-8 epoch CI (realistic + AGE)
    -> WP-9 split god module / optional extra bin
    -> WP-10 measure; squash only if budget miss
```

GitLab duration reminder: regular schema *"<= 3 minutes"* unless critical; long data is a background job ([style guide](https://docs.gitlab.com/development/migration_style_guide/)). Map: Phase E ≈ regular, Phase D ≈ batched background, Phase C ≈ post-deploy / gated.

---

## WP-1 — Manifest SSOT

**Closes:** D-06. **Files:**
- add `edgequake/migrations/manifest.toml` (new; not in tree until this WP)
- parse in `edgequake-api` (or small `edgequake-migrate-manifest` module)
- replace `KNOWN_CHECKSUM_REPAIR_VERSIONS` ([`checksum_repair.rs:24`](../../edgequake/crates/edgequake-api/src/state/migration_bootstrap/checksum_repair.rs))
- replace `IRREVERSIBLE_DROP_VERSIONS` ([`mod.rs:838`](../../edgequake/crates/edgequake-api/src/state/migration_bootstrap/mod.rs))
- replace `Makefile:1167`
- replace lists in [`tests/common/test_db.rs`](../../edgequake/crates/edgequake-api/tests/common/test_db.rs) and [`postgres_test_config.rs`](../../edgequake/crates/edgequake-storage/tests/support/postgres_test_config.rs)
- extend `contract_spec111_checksum_repair_wiring.rs` to assert Makefile **or** delete Makefile copy

**AC:** one parse error fails `cargo test`; grepping the old integer arrays in those files returns nothing except comments pointing at the manifest.

**Rollback:** revert the commit; lists restored.

---

## WP-2 — Known-variant checksum registry

**Closes:** D-05. **Depends:** WP-1.

Embed full SHA-384 (not 24-char prefixes) for:

| version            | hashes                                                            |
| --------------------| -------------------------------------------------------------------|
| 1                  | `bb40c61f…` current, `9e44513e…` v0.11.0                          |
| 19                 | `1f538faa…` current, `7b544306…` v0.10.6–v0.10.12                 |
| 71                 | `fa6cce9c…` pre-0.14, `fea7b113…` current                         |
| 78                 | `d22cc6d8…` v0.13.2, `a0431772…` current                          |
| 118, 121, 125, 131 | old v0.23.0 vs v0.24.2 (see [03](03-release-schema-evolution.md)) |

Behavior: if stored checksum ∈ manifest set, `UPDATE _sqlx_migrations SET checksum = current` **without env**. If stored ∉ set, fail with `refuse_silent_repair_message` plus listed known hashes. `EDGEQUAKE_ALLOW_CHECKSUM_REPAIR` remains for **unknown** emergency only; `EDGEQUAKE_DEV_MODE` must **not** auto-allow unknown hashes (auth bleed).

**AC:** e2e: fake ledger row 1 with `9e44513e` full hash; `edgequake migrate` succeeds; unknown hash fails without env.

**Rollback:** keep env-gated path behind a flag for one release.

---

## WP-3 — Runner hardening

**Closes:** D-07, D-08, part of D-09/D-10.

- Session `SET lock_timeout`, `SET statement_timeout` on the migrate connection (defaults in [10](10-performance-budget.md)).
- `pg_try_advisory_lock` loop with `EDGEQUAKE_MIGRATE_LOCK_DEADLINE` (default 60s) wrapping **repair + sqlx + reconcile**.
- Dirty version: print version, `error` column if present, pointer to [11](11-ops-runbook.md).
- Table `edgequake.migration_run` / `_step` (duration_ms, phase, sqlstate).
- Honor `-- no-transaction` / manifest `no_transaction` for CIC and batched D.
- Retry `55P03` lock_not_available with backoff.

**AC:** two concurrent migrate: one holds lock, second exits 75 within deadline. Kill-9 during M156-style job: rerun resumes (D jobs). Per-step durations in `migrate status`.

**Rollback:** feature-flag old sqlx-only lock.

---

## WP-4 — Serving never writes

**Closes:** D-04, D-13.

- Checksum repair: CLI-only (`migrate_cli_mode()`), **or** read-only compare on serve.
- Delete `tokio::spawn` of m040/m139/m140/m141 from `bootstrap_for_serving` (`mod.rs:1487-1515`). Run those scripts from CLI Phase D.
- `migration_engine::spawn_for_serving` (`postgres.rs:269`): default **off** on API; on for migrate worker / explicit `EDGEQUAKE_MIGRATION_MODE=automatic` **on migrate process**.
- `execute_bootstrap_apply_sql` stays CLI-only (already).

**AC:** `contract` test: serving boot with pending support/140 work does not `pg_stat_activity` DDL. grep `raw_sql` under `reconcile/` not reached from `bootstrap_for_serving` without CLI env.

**Rollback:** emergency env `EDGEQUAKE_SERVE_RECONCILE=1` one release, default off.

---

## WP-5 — Compat gate and wait mode

**Closes:** D-01, D-11, D-14.

- `EDGEQUAKE_SCHEMA_GATE=wait|fail`.
- Bind HTTP **before** schema refusal in wait mode (restructure `main.rs` so a minimal Axum with `/live` `/ready` can start; full `AppState` may still require schema — if full state needs tables, wait-mode uses a **lite** router until gate passes, then upgrades, **or** constructs state against expand-compatible schema only).
- Honest constraint: if `AppState` queries missing tables, wait-mode **must not** construct full state. Implementation: two-phase listen (lite then full) is the sure path.
- `compat_serve_min/max` from manifest.
- `/ready` re-queries ledger + 038 indexes (drop boot snapshot as sole source).
- Keep exit 78 for `fail` and for NEWER than `compat_serve_max`.

**AC:** `contract_spec091_boot_gate.rs` grows wait-mode cases: pending expandable → process lives, `/live` 200, `/ready` 503, after CLI migrate `/ready` 200 **without** restart.

**Rollback:** default `fail` = today's exit 78.

---

## WP-6 — Authoring lint + lock hygiene

**Closes:** D-09, part D-12/D-13.

Rust test (not only bash) over `edgequake/migrations/*.sql`:

| Lint | Fail if |
|------|---------|
| inner txn | top-level `BEGIN;` / `COMMIT;` (except comments) |
| CIC | `CONCURRENTLY` without `-- no-transaction` first line |
| unbounded DML in expand | `UPDATE`/`INSERT INTO … SELECT` without `LIMIT`/`WHERE ctid` in phase=expand |
| CHECK | `ADD CONSTRAINT` without `NOT VALID` on existing tables (allow on CREATE TABLE) |
| timeout 0 | `statement_timeout = 0` in numbered expand files |

`scripts/update_migration_checksums.sh`: refuse to change an existing lock line (append-only). Checksum-lock `support/**`.

**AC:** a PR that edits M001 body fails CI even if lock "updated". Inner BEGIN in a new file fails lint.

---

## WP-7 — Deployment wiring

**Closes:** D-02, D-03.

- Helm: `pre-install,pre-upgrade`; `postgres.enabled` gates wait container; `activeDeadlineSeconds`; API `SCHEMA_GATE=wait`; `startupProbe` `/live`.
- Compose: migrate service + `service_completed_successfully` ([docs](https://docs.docker.com/compose/how-tos/startup-order/)).
- ECS task snippet in [11](11-ops-runbook.md).
- Fix `edgequake/docs/migrations.md` auto-apply lie; SPEC-138 lens.

**AC:** `helm template` shows pre-upgrade hook. compose config contains migrate service. Docs grep "auto-apply" on API start = 0.

---

## WP-8 — Epoch upgrade matrix in CI

**Closes:** D-12. Protocol: [09](09-test-proof-protocol.md).

- Remove `|| true` on psql apply (`postgres-integration.yml:139`).
- `migration-guard` image **with AGE**.
- Nightly (or pre-release): restore dump from each **key** epoch fixture → HEAD migrate → assert ledger max + canary queries.
- Seeds: multi-workspace same `doc_id`; non-empty `"Node"`; embedding dim > 2000 for 071 path.

**AC:** M078 typo class would fail (AGE + Node table). M118 21000 would fail (two workspaces).

---

## WP-9 — Split `migration_bootstrap`

**Closes:** D-15. After WP-3/4/5 so splits are not churn.

Modules: `gate.rs`, `runner.rs`, `repair.rs`, `reconcile/`, `report.rs`. Optional `[[bin]] edgequake-migrate` that calls `runner::main`. No duplicated `migrate!`.

**AC:** `mod.rs` < 400 lines; clippy `-D warnings`.

---

## WP-10 — Measured fast path

**Depends:** WP-8 timings in `migration_run_step`.

If fresh 001..HEAD **empty** DB > budget in [10](10-performance-budget.md): add `000_baseline.sql` used **only** when ledger empty (detect: no `_sqlx_migrations` or zero rows). Existing DBs never squash. Decision recorded in measurements/.

**AC:** numbers in CI artifact; squash PR blocked without those numbers.

---

## Suggested release cuts

| Cut | WPs | Operator-visible |
|-----|-----|------------------|
| 150.a | 1–2 | Ancient fleets migrate |
| 150.b | 3–5 | No crash-loop; no serve DDL |
| 150.c | 6–8 | Lint + epoch CI |
| 150.d | 9–10 | Structure + optional squash |
