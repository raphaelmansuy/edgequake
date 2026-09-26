# 10 — Performance budget

Parent: [README](README.md) · Runner: [06](06-target-architecture.md) · WP-10: [08](08-implementation-plan.md)

Measure first (WP-3 telemetry). Numbers below are **targets**, not today's facts except SPEC-93.

## Known measurement

SPEC-93 (2026-08-02): GHCR `0.22.0` → migrations through **141**, 600 docs, `realism` profile ([matrix-summary](../93-migration-assessment/reports/matrix-summary.md)):

| PG | Wall |
|----|------|
| 16.14 | **85 s** |
| 17.10 | **397 s** |
| 18.4 | **322 s** |

That is schema+synthetic data, not 178k-node graphs (SPEC-083) and not M156/M158 lineage rewrite.

M158 header in-tree: ~20s per 120k rows (single txn). Live SPEC-149 note: 11.5s orphan repair on that corpus — still one transaction.

## Targets (empty DB, PG16, AGE+vector, CI hardware class)

| Work | Budget | If exceeded |
|------|--------|-------------|
| Fresh 001..HEAD Phase E only (no HNSW rebuild of big tables) | **≤ 3 min** (GitLab "regular") | WP-10 squash for **empty** ledger |
| Fresh including index builds on empty tables | **≤ 10 min** | defer non-critical indexes to Phase D CIC |
| Epoch v0.22 → HEAD empty-ish | **≤ 15 min** | split D jobs; do not block API (wait-mode) |
| Phase D per batch statement | **≤ 1 s** cold (GitLab background guideline as aspiration) | smaller batches |
| Phase C DROP after GREEN | **≤ 5 min** exclusive window | maintenance window |

HNSW: pgvector builds are CPU/IO heavy and take SHARE (non-CONCURRENT). Policy: **one HNSW build per database at a time** (already SPEC-091/112). Never inside a transaction that also rewrites types if it can be split.

## Timeout classes (WP-3 session defaults)

| Class | `lock_timeout` | `statement_timeout` | Used for |
|-------|----------------|---------------------|----------|
| `ddl_short` | 5s | 30s | ADD nullable, CREATE TABLE |
| `ddl_share` | 15s | 10min | CREATE INDEX in txn (empty/small) |
| `ddl_cic` | 5s (each wait) | 0 **only** on CIC connection | `CREATE INDEX CONCURRENTLY` no_tx |
| `dml_batch` | 5s | 30s | keyset UPDATE/INSERT |
| `contract` | 30s | 10min | VALIDATE / DROP when gated |
| `graph_rewrite` | 30s | 60s per batch | M156/M158 **after** split; forbid 0 on whole-run |

`statement_timeout = 0` in numbered files is a lint failure (WP-6) except documented CIC sessions.

Retry: on `55P03` (lock_timeout) exponential backoff, max 8, then fail. Do not silently skip ALTER (SPEC-083).

## Batching rules (Phase D)

```text
  keyset or ctid loop
  LIMIT 1000..10000 (adaptive: WP if already in migration_engine)
  COMMIT per batch          -- requires no_tx or engine not sqlx-file-txn
  on 21000/23505: do not advance cursor; split/dedupe; count failed_count
```

M156/M158 today: loop inside one `DO` in one sqlx txn = **not** batched for lock/WAL purposes. Rewrite is WP-3/D, not a new numbered edit of shipped 156 if 156 already released — if still unreleased (HEAD vs v0.26.10 **A** files), **fix the file before the next tag**.

## CONCURRENTLY / no_tx

Postgres: CIC cannot run in a transaction. sqlx: `-- no-transaction` first line (`sqlx-core-0.8.6` `source.rs:127`).

Use CIC for: GIN on large `Node` (038), listing indexes on large `documents` (128 class), unique indexes on `EDGE`.

Do not CIC in the same file as a transactional `INSERT` ledger expectation without splitting marker vs job.

## Squash decision (WP-10)

Compute `T_fresh_e` = p50 wall of F-empty migrate CI (PG16).

```text
  if T_fresh_e > 180s for two consecutive weekly nightlies:
      propose 000_baseline.sql = schema-only dump after 001..HEAD
      apply only when _sqlx_migrations missing/empty
      still ship 001..HEAD for existing ledgers
  else:
      no squash
```

Never squash to "fix" fossils — that is WP-2.

## Instrumentation

`edgequake.migration_run_step(run_id, version, phase, duration_ms, lock_waits, sqlstate)`. `migrate status` prints a table. CI uploads CSV next to SPEC-93 reports.
