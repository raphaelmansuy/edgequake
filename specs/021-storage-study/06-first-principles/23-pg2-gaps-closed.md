# 23 — P-G2 Gap Closure (2026-06-26)

> **Spec**: 021-storage-study
> **Supersedes**: open items in `22-pg2-post-ship-brutal-assessment.md` §6–§7

## Closed items

| Gap ID | Fix | Evidence |
|--------|-----|----------|
| **P-G2b-config** | `IngestionPersistSettings` + `IngestionPersistConfig::from_settings`; `ChunkVectorBuildOptions::STANDARD` for all callers | `ingestion_persister.rs`; `contract_persist_config_parity_across_callers` |
| **P-G5 saga** | `compensate_merge_failure` rolls back chunk vectors, new entity/rel vectors, new graph nodes/edges | `compensation.rs`; `MergeArtifacts` in merger |
| **P-G4-merger** | Batch entity + relationship vector upserts in `KnowledgeGraphMerger::merge` | `merger/entity.rs`, `merger/relationship.rs` |
| **DRY tests** | `edgequake-pipeline/tests/common/mod.rs` shared fixtures | contract tests |
| **P-G2c E2E** | Worker-backed upload + document chunks; graph assert on `completed` | `e2e_spec021_ingestion_persister.rs` |
| **Error policy** | Processor persist failure → `failed` (not `partial_failure`) | `text_insert.rs` |

## Honest remaining gaps

| Item | Status | Notes |
|------|--------|-------|
| P-G2d trait persister | Deferred | Free function meets DRY; trait adds indirection without correctness delta |
| Postgres UNWIND E2E | Deferred | Memory contracts + worker E2E sufficient for CI; postgres feature tests elsewhere |
| Mock LLM `partial_failure` | Accepted | Worker E2E asserts chunks always; graph assert gated on `completed` |
| Legacy graph corruption | P-G1b admin | Not auto-run |

## Verification

```bash
make test-spec021
cargo test -p edgequake-pipeline --test contract_ingestion_persistence
cargo test -p edgequake-api --test e2e_spec021_ingestion_persister
cargo test -p edgequake-storage --lib compensation
```
