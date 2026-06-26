# 24 — Plan-19 Gap Closure (2026-06-26, rev.4)

> **Supersedes:** remaining items in plan-19 §6 and §13 (post P-G2 closure pass)

## Closed in this pass

| Gap | Fix | Evidence |
|-----|-----|----------|
| **P-G4-graph** | Batch `get_nodes_batch` + `upsert_nodes_batch`; batch edges | `merger/entity.rs`, `merger/relationship.rs` |
| **P-G2d** | `IngestionPersister` trait + `DefaultIngestionPersister::from_settings` | `ingestion_persister.rs`, contract tests |
| **P-G8-http** | HTTP Bypass + Mix (+ stats) | `e2e_spec021_query_modes_http.rs` |
| **P-G9** | Result cache + invalidation on worker + orchestrator persist | `query_result_cache.rs`, worker + orchestrator E2E |
| **P-G9-DIP** | `QueryResultCacheInvalidator`; processor `Arc<dyn …>` | `spec021_processor_cache_invalidator_contract.rs` |
| **Orchestrator cache** | `with_query_engine` + `invalidate_result_cache()` after persist | `ingestion.rs`, `spec021_orchestrator_cache_invalidation.rs` |
| **Worker graph E2E** | Seeded mock + provider override → `completed` + `SARAH_CHEN` node | `safety_limits` test hook, `e2e_spec021_ingestion_persister.rs` |
| **DRY merger** | Batch-only path (removed single shims) | `merger/entity.rs`, `merger/relationship.rs` |
| **DRY test state** | `build_test_state` + `build_ingestion_pipeline` (not `default_pipeline`) | `memory.rs` |

## Honest remaining (accepted)

| Item | Status | Notes |
|------|--------|-------|
| P-G1b legacy backfill | Admin-only | Never auto-run |
| Full 8-step persister | Deferred | KV/relational/lineage in processor |
| Postgres UNWIND worker E2E | Deferred | Adapter contracts elsewhere |
| GraphRAG communities | Out of scope | §8 |
| Mix HTTP weight ordering | Engine-only | `contract_query_modes.rs` |
| Default orchestrator engine | No result cache | Callers must `with_query_engine` for P-G9 |

## Verification

```bash
make test-spec021
```
