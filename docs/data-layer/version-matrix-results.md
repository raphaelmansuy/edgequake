---
title: "Version matrix live results"
description: "One captured run of the data-layer test suites on a local PostgreSQL 18.4 database on 2026-07-25, with PG17 and PG16 left to the CI matrix. Historical; may not reflect current code."
---

# Version matrix live results

> **Historical record.** These results were captured on **2026-07-25**, on a local PostgreSQL 18.4 database (pgvector 0.8.5, AGE 1.8.0). The code and the migrations have changed since then, so they may not reflect the current code. Re-run the suites for current results.

| Suite | PG18 | PG17 | PG16 |
|---|---|---|---|
| `data_layer_ops_matrix` (235 Ref IDs) | **pass** | pending CI | pending CI |
| `data_layer_scaling` | **pass** | pending CI | pending CI |
| `data_layer_limits` | **pass** | pending CI | pending CI |
| `data_layer_registry` | **pass** (no database needed) | **pass** | **pass** |
| `lint_dataop_xref` | **pass** | **pass** | **pass** |

The PG17 and PG16 columns marked "pending CI" are filled by the scheduled run of `.github/workflows/data-layer-matrix.yml`, or by starting it with `battle=true`.

For the version pins and per-operation status, see [version-matrix.md](./version-matrix.md). To run the suites yourself, see the commands in [README.md](./README.md#ref-ids-and-tests).
