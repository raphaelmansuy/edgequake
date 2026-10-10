---
title: "Data-layer benchmark templates"
description: "Per-operation plan templates for nine hot data-layer operations, with the expected cost and the plan shape to check. Holds no measured timings; the test harness can append real EXPLAIN output."
---

# Data-layer benchmark templates

These pages describe nine hot data-layer operations. Each page gives the cost class, the access path the operation should use, and how latency should grow with table size. The pages are templates, so they hold no measured timings.

## Operations

| Page | Operation ID | What it does | Engine |
|------|--------------|--------------|--------|
| [001](./001.md) | `DATA-PGVEC-VECTORS-ANN-QUERY-001` | Vector search (ANN) | PGVEC |
| [002](./002.md) | `DATA-PGVEC-VECTORS-ANN-QUERY-FILTERED-002` | Vector search with filters | PGVEC |
| [004](./004.md) | `DATA-PGVEC-VECTORS-UPSERT-BATCH-004` | Batch vector upsert | PGVEC |
| [031](./031.md) | `DATA-AGE-GRAPH-GET-NODES-BATCH-031` | Batch node read | AGE |
| [046](./046.md) | `DATA-AGE-GRAPH-UPSERT-NODES-BATCH-046` | Batch node write | AGE |
| [075](./075.md) | `DATA-PG-KV-GET-BY-ID-075` | Legacy key-value read | PG |
| [076](./076.md) | `DATA-PG-KV-GET-BY-IDS-076` | Legacy key-value batch read | PG |
| [079](./079.md) | `DATA-PG-KV-UPSERT-079` | Legacy key-value write | PG |
| [140](./140.md) | `DATA-PG-TASKS-CLAIM-NEXT-140` | Task claim | PG |

The full set of 235 templates lives under `specs/088-data-layer/benchmarks/`. The key-value operations are legacy: migration 125 (`125_spec091_kv_drop.sql`) dropped the `eq_*_kv` tables.

## Where the results are

- [improvements.md](../improvements.md): measured SPEC-088 timings.
- [version-matrix-results.md](../version-matrix-results.md): suite results per PostgreSQL version.

## Capture a real plan

The data-layer test harness can append a real plan to a template. Set `EDGEQUAKE_DATA_LAYER_CAPTURE_EXPLAIN=1`. The harness adds a "Captured EXPLAIN" section once and never overwrites it.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart LR
  A["Set DATABASE_URL and EDGEQUAKE_DATA_LAYER_CAPTURE_EXPLAIN=1"] --> B["cargo test data_layer_ops_matrix"]
  B --> C["Harness runs EXPLAIN ANALYZE"]
  C --> D["docs copy: path does not exist, skipped"]
  C --> E["specs copy: section appended"]
%% eq-classes
classDef eqStore fill:#D1FAE5,stroke:#10B981,color:#064E3B
class A eqStore
```

The plan lands only in the `specs/088-data-layer/benchmarks/` copy, as the diagram shows. See the known gap below.

Run the matrix test from the Rust workspace, with a PostgreSQL server that the role in `DATABASE_URL` can use to create databases:

```bash
cd edgequake
export DATABASE_URL=postgres://edgequake:edgequake_secret@localhost:5432/edgequake
EDGEQUAKE_DATA_LAYER_CAPTURE_EXPLAIN=1 \
  cargo test -p edgequake-storage --features postgres --test data_layer_ops_matrix
```

The test creates and uses its own scratch database, so the shared dev database is not touched.

## Known gap

The harness builds the docs path as `../../docs/data-layer/benchmarks/` from the crate directory. That resolves to `edgequake/docs/data-layer/`, which does not exist. The docs live at the repository root, so the plan is written only to the `specs/088-data-layer/benchmarks/` copy. Copy the "Captured EXPLAIN" section over by hand if you want it here.
