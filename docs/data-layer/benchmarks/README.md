---
title: "Data-layer benchmark templates"
description: "Per-operation plan templates for nine hot data-layer operations, with the expected cost and the plan shape to check. Holds no measured timings; the test harness can append real EXPLAIN output."
---

# Data-layer benchmark templates

Each page here describes one hot operation: its cost class, the access path it should use, and how latency should scale with table size. The pages are templates. They hold no measured timings.

| Page | Operation |
|---|---|
| [001](./001.md) | `DATA-PGVEC-VECTORS-ANN-QUERY-001`, vector search |
| [002](./002.md) | `DATA-PGVEC-VECTORS-ANN-QUERY-FILTERED-002`, vector search with filters |
| [004](./004.md) | `DATA-PGVEC-VECTORS-UPSERT-BATCH-004`, vector upsert |
| [031](./031.md) | `DATA-AGE-GRAPH-GET-NODES-BATCH-031`, batch node read |
| [046](./046.md) | `DATA-AGE-GRAPH-UPSERT-NODES-BATCH-046`, batch node write |
| [075](./075.md) | `DATA-PG-KV-GET-BY-ID-075`, legacy key-value read |
| [076](./076.md) | `DATA-PG-KV-GET-BY-IDS-076`, legacy key-value batch read |
| [079](./079.md) | `DATA-PG-KV-UPSERT-079`, legacy key-value write |
| [140](./140.md) | `DATA-PG-TASKS-CLAIM-NEXT-140`, task claim |

The full set of 235 templates is under `specs/088-data-layer/benchmarks/`. The key-value operations are legacy: the `eq_*_kv` tables were dropped by migration 125.

## Capture a real plan

The data-layer test harness can append a plan to a template when you set `EDGEQUAKE_DATA_LAYER_CAPTURE_EXPLAIN=1`. It adds a "Captured EXPLAIN" section once and never overwrites it. Run it against a scratch database, because the matrix tests create their own scratch tables.

Known gap: the harness builds the path to this folder as `../../docs/data-layer/benchmarks/` from the crate directory. That resolves to `edgequake/docs/data-layer/`, which does not exist, so today the plan is appended only to the copy under `specs/088-data-layer/benchmarks/`. Copy the section over by hand if you want it here.

```bash
export DATABASE_URL=postgres://edgequake:edgequake_secret@localhost:5432/edgequake
EDGEQUAKE_DATA_LAYER_CAPTURE_EXPLAIN=1 \
  cargo test -p edgequake-storage --features postgres --test data_layer_ops_matrix
```

Where the measured results live: [improvements.md](../improvements.md) for the SPEC-088 timings, and [version-matrix-results.md](../version-matrix-results.md) for suite results per PostgreSQL version.
