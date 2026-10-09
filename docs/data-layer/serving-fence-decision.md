---
title: "Serving fence decision"
description: "Decision record GAP-091-21b: the serving fence is on by default, so queries only see chunks whose serving state is ready. Covers the rule, the reason, the evidence, and how operators turn it off."
---

# Serving fence decision (GAP-091-21b, SPEC-091 IP2)

**Status:** Accepted on 2026-07-31. The fence is **on** by default.

**Spec:** [21-ingestion-pipeline-data-model-improvement.md](../../specs/091-simplify-data-layer/21-ingestion-pipeline-data-model-improvement.md) (LAW-IP1, IP-AC-05).

The serving fence is a rule that hides a chunk from search until the whole ingest of that chunk has finished. It uses the table `chunk_serving_state`.

## Decision

- `EDGEQUAKE_SERVING_FENCE` is on when it is unset. Only `off`, `false`, `0`, or `no` turn it off.
- When the fence is on, vector search and keyword search return only chunks whose `chunk_serving_state.state` is `ready`.

## How a chunk becomes visible

The allowed `state` values are `declared`, `embedded`, `graphed`, `ready`, `quarantined`, and `deleting`. Only `ready` is visible to queries. The diagram shows the path to `ready`. Read it left to right.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart LR
    A["Chunk saved"] --> B["Embedding and graph writes finish"]
    B --> C["All deliveries for the document settle"]
    C --> D["State set to ready"]
    D --> E["Search can return the chunk"]
%% eq-classes
classDef eqLlm fill:#FEF3C7,stroke:#F59E0B,color:#451A03
class B eqLlm
```

The pipeline persister marks chunks `ready` after a successful save and merge. The projection worker also opens the fence for a document batch when the last graph and vector deliveries are acknowledged. It takes a per-document advisory lock so two acknowledgements that commit at the same time cannot both miss the open. A failed merge goes to compensation instead (rule LD-09 / LAW-IP1).

## Reason

Before this decision, the fence was off after the typed-table cutover. A half-ingested chunk could show up in answers. That is the opposite of failing closed. SPEC-091 IP2 added the writers for `outbox_events` and the step that marks chunks `ready` after a successful save and merge. With those in place, the safe default is on.

## Evidence in the repository

| Item | What it shows |
|---|---|
| `edgequake/crates/edgequake-storage/src/serving_fence.rs` | Unset means on. Only an explicit off value disables it. |
| `edgequake/migrations/109_spec091_serving_fence.sql` and `133_spec091_outbox_harden.sql` | The fence table and the outbox schema. |
| `edgequake/crates/edgequake-pipeline/src/persistence/ingestion_persister.rs` | Marks chunks `ready` and writes the outbox events `chunk_ready`, `merge_done`, and `compensate`. |

## What operators should do

1. On a fresh install the fence is on. Wait for ingest to finish before you expect a document to appear in answers.
2. To roll back during a soak, set `EDGEQUAKE_SERVING_FENCE=off`.
3. The document list shows a `query_ready` flag when the fence is on. See [20-ingestion-surface-assessment.md](../../specs/091-simplify-data-layer/20-ingestion-surface-assessment.md).

## When to revisit

If a measured recall drop appears on partial-ingest demos, record it under `specs/091-simplify-data-layer/measurements/` and consider a staged default for one release. Do not revert the default without evidence (LAW-I2).

See also: [pgvector.md](./pgvector.md#serving-fence) and [postgres.md](./postgres.md#chunks-and-chunk_serving_state).
