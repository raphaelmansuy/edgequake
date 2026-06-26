# 21 — P-G2 IngestionPersister (First Principles, DRY, SOLID)

> **Spec**: 021-storage-study
> **Date**: 2026-06-26
> **Status**: ✅ **DONE** (P-G2a–c closed 2026-06-26)
> **Closes**: RC-7, RC-9 (merger batching), RC-10 (partial saga), plan-19 P-G2

## First principles

| Principle | Decision |
|-----------|----------|
| **One write sequence** | `persist_processing_result()` — chunk vectors → batched entity/rel vectors → graph merge → full compensation on failure |
| **DRY** | `IngestionPersistConfig::from_settings` + `ChunkVectorBuildOptions::STANDARD` — orchestrator + processor identical |
| **SOLID** | SRP: persister writes; processor orchestrates lifecycle. DIP: storage traits injected via config factory |
| **LightRAG** | `KnowledgeGraphMerger` canonical; no manual processor graph batches |

## Implementation

- `edgequake-pipeline/src/persistence/ingestion_persister.rs`
- `edgequake-storage/src/compensation.rs` — `compensate_merge_failure` (vectors + new graph nodes/edges)
- `edgequake-pipeline/src/merger/*` — batched vector upserts + `MergeArtifacts` for P-G5
- Callers: `orchestrator/ingestion.rs`, `processor/text_insert.rs`

## Tests ✅

| Test | Proves |
|------|--------|
| `contract_ingestion_persistence.rs` | Double-persist dedup, config parity, cross-doc merge |
| `e2e_spec021_ingestion_persister.rs` | Worker upload → chunks; graph nodes when `completed` |
| `compensation::tests::compensate_merge_failure_*` | P-G5 rollback |
| `make test-spec021` | Full SPEC-021 contract suite green |

## Out of scope (unchanged)

- KV chunk upsert, relational `documents` row, lineage — processor/orchestrator wrappers
- `IngestionPersister` trait (P-G2d) — free function sufficient; OCP gap accepted

## Post-ship review

Initial gaps documented in `22-pg2-post-ship-brutal-assessment.md`; closure in `23-pg2-gaps-closed.md`.
