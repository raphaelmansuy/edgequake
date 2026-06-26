# 24 — Plan-19 Gap Closure (2026-06-26, rev.3)

> **Supersedes:** remaining items in plan-19 §6 and §13 (post P-G2 closure pass)

## Closed in this pass

| Gap                     | Fix                                                                                                                                    | Evidence                                                                             |
| -------------------------| ----------------------------------------------------------------------------------------------------------------------------------------| --------------------------------------------------------------------------------------|
| **P-G4-graph**          | Merger uses `get_nodes_batch` + `upsert_nodes_batch` (entities) and `get_edges_for_nodes_batch` + `upsert_edges_batch` (relationships) | `merger/entity.rs`, `merger/relationship.rs`, `merger/mod.rs`                        |
| **P-G2d**               | `IngestionPersister` trait + `DefaultIngestionPersister`; orchestrator + processor depend on trait (DIP)                               | `ingestion_persister.rs`; `contract_persister_trait_matches_free_function`           |
| **P-G8-http**           | HTTP contract for Bypass + Mix (+ stats on mix)                                                                                        | `e2e_spec021_query_modes_http.rs`                                                    |
| **P-G9-result**         | `QueryResultCache` + invalidation on worker persist                                                                                    | `query_bootstrap.rs`, `with_query_engine` / `with_query_cache_invalidator`, `e2e_spec021_query_cache_invalidation.rs` |
| **P-G9-worker E2E**   | Upload → persist → query cache miss via production worker wiring                                                                       | `e2e_spec021_worker_cache_invalidation.rs`, `WorkerAppGuard.query_engine`            |
| **P-G9-DIP**            | `QueryResultCacheInvalidator` trait; processor holds `Arc<dyn …>`, not `Arc<QueryEngine>`                                              | `cache/mod.rs`, `spec021_processor_cache_invalidator_contract.rs`, `contract_invalidator_trait_clears_result_cache` |
| **P-G4-graph contract** | 12-entity batch merge test                                                                                                             | `contract_merger_graph_batch.rs`                                                     |
| **Cross-doc merge**     | Two documents, same entity → single node with both chunk ids                                                                           | `contract_cross_document_entity_merge` in `contract_ingestion_persistence.rs`        |
| **DRY merger**          | Batch-only merge path; removed unused single-entity/relationship shims                                                                 | `merger/entity.rs`, `merger/relationship.rs`                                        |
| **DRY persister**       | `DefaultIngestionPersister::from_settings` in orchestrator, processor, trait parity test                                               | `ingestion_persister.rs`                                                             |

## Honest remaining (accepted)

| Item | Status | Notes |
|------|--------|-------|
| P-G1b legacy backfill | Admin-only | Destructive; never auto-run |
| Full 8-step persister | Deferred | KV/relational/lineage still in processor by design |
| Postgres UNWIND worker E2E | Deferred | Memory contracts + postgres adapter tests elsewhere |
| GraphRAG communities | Out of scope | §8 |
| Core `insert()` cache invalidation | Deferred | No query engine in core; API worker path only |
| Mix HTTP weight ordering | Engine-only | `contract_query_modes.rs`; HTTP checks mode + stats |

## Verification

```bash
make test-spec021
cargo test -p edgequake-api --test e2e_spec021_worker_cache_invalidation
cargo test -p edgequake-pipeline --test contract_ingestion_persistence
cargo test -p edgequake-query --test contract_query_result_cache
```
