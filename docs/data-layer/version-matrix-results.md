---
title: "Version matrix live results"
description: "One captured run of the data-layer test suites on a local PostgreSQL 18.4 database on 2026-07-25, with PG17 and PG16 left to the CI matrix. Historical; may not reflect current code."
---

# Version matrix live results

This page records one run of the data-layer test suites. The PG18 column comes from a local run. The PG17 and PG16 columns were left for CI.

> **Historical record.** These results were captured on **2026-07-25**, on a local PostgreSQL 18.4 database (pgvector 0.8.5, AGE 1.8.0). The code and the migrations have changed since then, so they may not reflect the current code. Re-run the suites for current results.

| Suite | PG18 | PG17 | PG16 |
|---|---|---|---|
| `data_layer_ops_matrix` (235 Ref IDs) | **pass** | pending CI | pending CI |
| `data_layer_scaling` | **pass** | pending CI | pending CI |
| `data_layer_limits` | **pass** | pending CI | pending CI |
| `data_layer_registry` | **pass** (no database needed) | **pass** | **pass** |
| `lint_dataop_xref` | **pass** | **pass** | **pass** |

## How the cells are filled

The PG18 cells come from the local run above. The PG17 and PG16 cells marked "pending CI" are filled by the scheduled run of `.github/workflows/data-layer-matrix.yml`, or by starting that workflow with the `battle` input set to `true`.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart LR
    local["Local run on PostgreSQL 18.4"] --> pg18["PG18 cells"]
    sched["Scheduled run of data-layer-matrix.yml"] --> pg17["PG17 and PG16 cells"]
    dispatch["Manual run with battle set to true"] --> pg17
    pg17 --> results["Updated results table"]
    pg18 --> results
%% eq-classes
classDef eqStore fill:#D1FAE5,stroke:#10B981,color:#064E3B
class local,results eqStore
```

For the version pins and per-operation status, see [version-matrix.md](./version-matrix.md). To run the suites yourself, see the commands in [README.md](./README.md#ref-ids-and-tests).
