# SPEC-034 E2E Test Suite

> **Status**: Defined  
> **Last updated**: 2026-06-30  
> **Coverage**: IMP-01 through IMP-08

---

## Overview

This directory contains end-to-end tests and evidence for SPEC-034: Graph Storage
Performance Improvements. Each test validates a specific improvement track and
records before/after measurements.

## Test Files

| File                                 | IMP    | What it validates                                       |
| ------------------------------------ | ------ | ------------------------------------------------------- |
| `001_migration_smoke.sh`             | All    | Migrations 067–073 run without error                    |
| `002_kv_gin_absent.sql`              | IMP-03 | KV GIN index removed, queries still work                |
| `003_fts_dedup_absent.sql`           | IMP-05 | Duplicate FTS index removed                             |
| `004_age_index_count.sql`            | IMP-02 | AGE index count reduced to ≤6 per label                 |
| `005_edge_text_indexes.sql`          | IMP-07 | Edge text-cast expression indexes exist                 |
| `006_vector_metadata_gin_absent.sql` | IMP-08 | Vector metadata GIN removed                             |
| `007_hnsw_ef32.sql`                  | IMP-04 | HNSW rebuilt with ef_construction=32                    |
| `008_native_writes_feature_flag.sh`  | IMP-01 | EDGEQUAKE_NATIVE_GRAPH_WRITES=1 compiles and dispatches |
| `009_async_community_index.sh`       | IMP-06 | Community index no longer blocks persist path           |

## Running the Tests

```bash
# Prerequisites
make postgres-start
make backend-bg
sleep 5

# Run all e2e tests
cd specs/034-graph-storage-improvement/e2e
bash run_all.sh

# Or run individually:
bash 001_migration_smoke.sh
psql $DATABASE_URL -f 002_kv_gin_absent.sql
psql $DATABASE_URL -f 003_fts_dedup_absent.sql
# ... etc
```

## Evidence Files

After running, evidence is written to:
- `evidence/001_migration_smoke.out` — migration run output
- `evidence/002_kv_gin_absent.out` — SQL query results
- `evidence/index_sizes_before.txt` — index sizes before migrations
- `evidence/index_sizes_after.txt` — index sizes after migrations

## Acceptance Criteria Summary

| Improvement | Criterion                                              | Pass         |
| ----------- | ------------------------------------------------------ | ------------ |
| IMP-01      | Node batch upsert 200 nodes < 500ms (with native flag) | Requires DB  |
| IMP-02      | Node label index count ≤ 6                             | DB migration |
| IMP-03      | KV GIN index absent                                    | DB migration |
| IMP-04      | HNSW ef_construction=32                                | DB migration |
| IMP-05      | No duplicate FTS index                                 | DB migration |
| IMP-06      | Community index refresh non-blocking                   | Unit test    |
| IMP-07      | Edge text-cast indexes present                         | DB migration |
| IMP-08      | Vector metadata GIN absent                             | DB migration |
