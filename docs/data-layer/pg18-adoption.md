---
title: "PG18 capability-gated adoption notes"
description: "SPEC-091 IW4 decisions on PostgreSQL 18 features: which ones EdgeQuake uses through runtime probes (uuidv7, iterative ANN scans, AGE casts) and which ones are deferred (virtual columns, RETURNING OLD/NEW, async I/O, skip scan)."
---

# PG18 capability-gated adoption notes (SPEC-091 IW4)

EdgeQuake supports PostgreSQL 16, 17, and 18 with one SQL path. The code checks at run time which features the database has (`PostgresCapabilityProbe`, reported at `/health` under `schema.postgres_capabilities`). It uses a PG18-only feature only if that feature cannot break PG16.

## Adopted through a capability probe

| Feature | Probe | Behavior |
|---|---|---|
| uuidv7 document IDs | `uuidv7_available` (`capabilities.rs`) | On PG18, new document IDs come from `uuidv7()`. Older versions use random v4 UUIDs. |
| Iterative ANN scans | `iterative_scan_available` (pgvector 0.8.0 or later) | Filtered vector queries set `hnsw.iterative_scan` and `hnsw.max_scan_tuples`. See [pgvector.md](./pgvector.md#how-a-vector-search-runs). |
| AGE jsonb to agtype casts | `age_jsonb_agtype_cast_available` (AGE 1.8.0-rc0 or later) | The probe only reports on `/health`. Application SQL stays portable until a win is measured (SPEC-091 RM3). |

## Deferred

### Virtual generated column for workspace metadata (GAP-091-24)

PG18 can define a virtual generated column. It could index `metadata->>'workspace_id'` without storing a second copy. A migration that depends on it would break PG16 databases, so it is deferred.

Interim fix on all versions: the workspace bulk delete in `document_read_model.rs` uses a UNION of indexed predicates, and migration 128 adds listing indexes. See [serving-fence-decision.md](./serving-fence-decision.md).

Later option: a migration guarded by `server_version_num >= 180000`, only when a measured win justifies it.

### `RETURNING OLD` and `RETURNING NEW`

PG18 lets triggers and statements return the old and new row. That would simplify outbox-style change capture, but it needs PG18-only trigger code. The compensation and outbox paths use portable `RETURNING` plus explicit reads.

### Asynchronous I/O (`io_method`)

PG18 async I/O can cut heap-fetch time for large sequential scans and some ANN filter paths. EdgeQuake does not set `io_method` in application code. Operators may try it and measure first:

```sql
-- Use 'worker' where io_uring is not available
ALTER SYSTEM SET io_method = 'io_uring';
SELECT pg_reload_conf();
```

Before you enable it across a fleet, compare p95 for three paths on the same hardware with `io_method` set to `worker`, `io_uring`, and the default:

1. Filtered typed vector search with the serving-fence join.
2. The document list on `(workspace_id, created_at)`.
3. AGE neighbor expansion.

Keep the artifacts under `specs/091-simplify-data-layer/measurements/`. Revert if the change is neutral or worse (LAW-I2).

### Skip scan on composite btrees

PG18 may use a skip scan on a composite index where PG16 and PG17 would scan the whole table. When you add a composite index (migration 128 is an example), check the plan with `EXPLAIN (ANALYZE, BUFFERS)` on PG18. No separate SQL branch is needed today.

## Sources of truth

- Runtime probes: `edgequake/crates/edgequake-storage/src/adapters/postgres/capabilities.rs`
- Version pins: `edgequake/docker/extension-pins.sh`
- Operator matrix: [version-matrix.md](./version-matrix.md)
- Related decision: [pg17-differential.md](./pg17-differential.md)
