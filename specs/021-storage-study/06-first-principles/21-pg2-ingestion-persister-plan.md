# 21 — P-G2 IngestionPersister (First Principles, DRY, SOLID)

> **Spec**: 021-storage-study
> **Date**: 2026-06-26
> **Status**: ✅ DONE (2026-06-26)
> **Closes**: RC-7 / plan-19 P-G2 (structural DRY — one function body)

## First principles

| Principle | Decision |
|-----------|----------|
| **One write sequence** | Chunk vectors → KnowledgeGraphMerger (graph + entity vectors + relational sink) → compensate on failure |
| **DRY** | `persist_processing_result()` in `edgequake-pipeline` — both orchestrator and processor delegate |
| **SOLID** | SRP: pipeline computes; persister writes. DIP: callers pass `Arc<dyn GraphStorage/VectorStorage>` + config |
| **LightRAG** | Merger path is canonical (not manual `upsert_nodes_batch` in processor) |

## Implementation

- `edgequake-pipeline/src/persistence/ingestion_persister.rs`
- Orchestrator `ingestion.rs` delegates (~120 lines removed)
- Processor `text_insert.rs` replaces manual graph/entity-vector batch (~430 lines) with persister call
- `edgequake-pipeline/tests/contract_ingestion_persistence.rs`
- `edgequake-api/tests/e2e_spec021_ingestion_persister.rs`

## Acceptance ✅

- Contract: `contract_ingestion_persistence.rs` — double persist → one normalized node + one chunk vector
- E2E: `e2e_spec021_ingestion_persister.rs` — merge stats with entities > 0
- `sc2_sc5_ingestion` + `make test-spec021` green

## Out of scope

- KV chunk upsert (processor-only, pre-persist) — stays in processor
- Relational `documents` row updates — stays in processor post-persist
