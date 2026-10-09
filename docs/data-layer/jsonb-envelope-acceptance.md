---
title: "JSONB envelope acceptance"
description: "Decision record GAP-091-05: four typed tables keep a JSONB payload column on purpose. Lists which tables, why, what is typed elsewhere, and what operators should know."
---

# JSONB envelope acceptance (GAP-091-05)

**Status:** Accepted by design. This is not a missing migration.

**Spec:** SPEC-091 IW3, [19-improvement-plan.md](../../specs/091-simplify-data-layer/19-improvement-plan.md).

JSONB is a PostgreSQL column type that stores flexible JSON. A few typed tables keep one JSONB payload column instead of splitting it into many typed columns. The application defines the shape of that payload (an "envelope"). This was a deliberate choice, not unfinished work.

## Tables that keep a JSONB payload

| Table | Column | What it holds | Why JSONB stays |
|---|---|---|---|
| `pipeline_checkpoints` | checkpoint payload | Resume tokens and extraction snapshots | Pipeline stages change often, and few queries read inside the payload. |
| `document_artifacts` | artifact body | Lineage and multimodal manifests and chunks | The shape varies. Reads are always by document ID. |
| `llm_cache` | `value` | Cached LLM, keyword, and multimodal answers | Keyed by hash. The provider payload is opaque. |
| `compensation_quarantine` | `payload` | Dead-letter records from failed merges | Same shape as the old key-value dead-letter queue, so operators keep one format. |

## Data that is already typed

- Document metadata lives in `documents.metadata` (JSONB) with a relational compare-and-set (`document_shell.rs`). It is the source for list and detail views and is outside this decision.
- Chunk text lives in `chunks.content` (text).
- Chunk vectors live in `chunk_embeddings.embedding` (`halfvec`).

## What operators should know

- The console and advisor residue checks leave out checkpoints, caches, and quarantine on purpose when they check that migration 125 drained the old key-value data. Those families are transient.
- The drain worker (`compensation_drain.rs`) reads the `payload.kind` of quarantine rows. You cannot retract a quarantined item with plain SQL. Use the applier.

## Where it is verified

- The typed stores write these tables directly: `relational_sidecar_store.rs` (in `edgequake-api`), `llm_cache.rs`, and `PgQuarantineSink`.
- Tests: `contract_spec091_llm_cache_scope.rs` and the compensation tests in `compensation.rs`.

See also: [llm-cache-scope.md](./llm-cache-scope.md) and the legacy key-value section in [postgres.md](./postgres.md#legacy-key-value-store).
