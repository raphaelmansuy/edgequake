# 21 — P-G2 IngestionPersister (First Principles, DRY, SOLID)

> **Spec**: 021-storage-study
> **Date**: 2026-06-26
> **Status**: ✅ **P-G2a SHIPPED** — structural SSOT (`404ce915`); gaps in **22-pg2-post-ship-brutal-assessment.md**
> **Closes**: RC-7 structural divergence / plan-19 P-G2 (partial — not full §2.2 trait design)

## First principles

| Principle | Decision | Honest gap |
|-----------|----------|------------|
| **One write sequence** | Chunk vectors → KnowledgeGraphMerger → compensate chunk vectors on failure | Entity-vector / partial-graph rollback still missing (P-G5) |
| **DRY** | `persist_processing_result()` in `edgequake-pipeline` — both orchestrator and processor delegate | Config + metadata built differently per caller (see file 22 §2) |
| **SOLID** | SRP: pipeline computes; persister writes. DIP: `Arc<dyn GraphStorage/VectorStorage>` | **No trait** — OCP not satisfied; plan-19 §2.2 not delivered |
| **LightRAG** | Merger path is canonical (manual processor batches removed) | Orchestrator omits chunk lineage metadata processor includes |

## Implementation

- `edgequake-pipeline/src/persistence/ingestion_persister.rs`
- Orchestrator `ingestion.rs` delegates (~120 lines removed)
- Processor `text_insert.rs` replaces manual graph/entity-vector batch (~430 lines) with persister call
- `edgequake-pipeline/tests/contract_ingestion_persistence.rs`
- `edgequake-api/tests/e2e_spec021_ingestion_persister.rs` (**misnamed** — memory-only, not HTTP E2E)

## Acceptance ✅ / ❌

| Check | Status |
|-------|--------|
| Contract: double persist → one normalized node + one chunk vector | ✅ |
| Both callers invoke `persist_processing_result` | ✅ |
| `sc2_sc5_ingestion` + `make test-spec021` green | ✅ |
| Byte-identical storage across callers (plan-19 original) | ❌ metadata + `MergerConfig` differ |
| Trait-backed `IngestionPersister` (plan-19 §2.2) | ❌ deferred → P-G2d |
| Production Postgres / API E2E | ❌ → P-G2c |

## Out of scope (explicit)

- KV chunk upsert (processor-only, pre-persist)
- Relational `documents` row updates (processor post-persist)
- Full 8-step persister (KV, relational stats, lineage) — plan-19 §P-G2 original steps 2, 7, 8

## Post-ship review

See **`22-pg2-post-ship-brutal-assessment.md`** for GraphRAG / LightRAG / AI / Systems verdict,
flakiness audit, and follow-up priorities (P-G2b-config, P-G2c-e2e, P-G5, P-G4-merger).
