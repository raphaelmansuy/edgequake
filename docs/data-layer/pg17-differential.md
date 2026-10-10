---
title: "PG17 differential measurement"
description: "Decision record for SPEC-091 IW4: PostgreSQL 17 planner changes did not justify any PG17-specific SQL, so EdgeQuake keeps one SQL path for PG16, PG17, and PG18."
---

# PG17 differential measurement (SPEC-091 IW4)

**Status:** Closed (LAW-I2). The result was measured against the IW1 typed-CRUD scorecard. **No PG17-specific SQL was adopted.** This is a historical decision record. The measured numbers are in the SPEC-091 measurements, not here.

## Question

GAP-091-31 asked: do PostgreSQL 17 planner improvements, such as skip scan and better join ordering, justify SQL or index shapes that differ from the shared PG16, PG17, and PG18 path?

## Method

- Same corpus and test harness as IW1 (`perf_harness`): typed relational create, read, update, and delete on `documents`, `chunks`, and `chunk_embeddings`.
- PG17 (`edgequake-postgres:pg17`) against a PG16 baseline. Both had the same migrations. The extension pins were pgvector 0.8.5 and AGE 1.7.0 on PG17.
- The adoption bar was a p95 improvement of at least 10% on list, delete, or search paths.

## Result

No operation met the bar. PG17 varied within noise on the indexed paths. The shared SQL stays the default on all supported majors. This includes the UNION workspace delete in `document_read_model.rs` and the listing indexes from migration 128.

## Decision

- Keep one SQL path. Gate features by runtime capability probes (`edgequake/crates/edgequake-storage/src/adapters/postgres/capabilities.rs`), not by hard-coded version branches.
- Ship no PG17-only migrations in this release train.
- Revisit only when a measured regression, or a win of at least 10% on a named Ref ID in [version-matrix.md](./version-matrix.md), appears.

The flow below records how the decision was reached.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart TD
    q["Do PG17 planner changes justify PG17-only SQL?"] --> m["Measure list, delete, and search p95 with the same harness"]
    m --> q1{"Is there a win of at least 10% on a named Ref ID?"}
    q1 -- "yes" --> adopt["Add a probe-gated path"]
    q1 -- "no" --> keep["Keep the shared SQL path"]
    keep --> rev["Revisit on a measured regression or win"]
```

## Related

- Runtime capability report: `/health`, field `schema.postgres_capabilities`.
- Nightly full matrix: `.github/workflows/postgres-matrix-nightly.yml`.
- Pull request smoke test: `.github/workflows/spec091-data-layer.yml` (job `spec091-pg-matrix-smoke`).
- PG18 decisions: [pg18-adoption.md](./pg18-adoption.md).
