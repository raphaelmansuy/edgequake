# 04 — Cross-Reference Index (RC-023)

> **Spec**: 023-egdequake-audit  
> **Purpose**: Single registry linking findings → code → tests → fix phase

---

## Finding registry

| ID | Sev | Title | Code anchor | Test coverage | Fix phase |
|----|-----|-------|-------------|---------------|-----------|
| RC-023-1 | CRITICAL | Injection bypasses IngestionPersister | `handlers/injection.rs` (pre-I1) | `e2e_spec023_injection_persister.rs` | ✅ I1 closed |
| RC-023-2 | HIGH | Global mode mislabeled vs implementation | `modes.rs`, `vector_queries.rs:242-378` | `contract_global_mode_semantics.rs` | ✅ I2 closed |
| RC-023-3 | HIGH | No retrieval eval harness in CI | — | `rag_benchmark_recall.rs` | ✅ I3 closed |
| RC-023-4 | MEDIUM | BM25 rerank only, no cross-encoder | `bootstrap.rs` | `contract_bootstrap_reranker_env.rs` | ⚠️ I4 partial |
| RC-023-5 | MEDIUM | Mix fusion ≠ RRF | `vector_queries.rs`, `fusion.rs` | `contract_rrf_fusion.rs` | ✅ I5 closed |
| RC-023-6 | MEDIUM | Community detection not in query path | `community.rs`, `graph_community.rs` | `resource_safety_proof.rs` | [I6](./05-improvement-plan.md#i6) |
| RC-023-7 | LOW | AGE batch upsert inline Cypher | `nodes_ops.rs:107+` | `spec022_cypher_prepared_postgres.rs` | [I7](./05-improvement-plan.md#i7) |
| RC-023-8 | LOW | Chunk content duplicated in vector metadata | `ingestion_persist.rs:188-192` | — | [I8](./05-improvement-plan.md#i8) |
| RC-023-9 | LOW | LLM merge summarization O(E) cost | `ingestion_persist.rs:262-267` | — | [I9](./05-improvement-plan.md#i9) |
| RC-023-10 | LOW | No sparse BM25 retrieval arm (rerank only) | `reranking.rs` | — | [I10](./05-improvement-plan.md#i10) |

---

## Closed by SPEC-022 (do not reopen)

| ID | Title | Closed in |
|----|-------|-----------|
| RC-022-1 | Sync upload bypass persister | P-H1 |
| RC-022-2 | Batch upload global vectors | P-H2 |
| RC-022-4 | Orchestrator missing BM25 | P-H4 |
| RC-022-5 | pgvector 0.7.4 | P-H3 |
| RC-022-6 | AGE string interpolation hot path | P-H7 |
| RC-022-8 | GraphRAG communities | Deferred → I6 |

---

## Code → test traceability (green paths)

| Subsystem | Contract test | E2E test |
|-----------|---------------|----------|
| Ingestion persist | `contract_ingestion_persistence.rs` | `e2e_spec022_file_upload_persister.rs` |
| Worker ingest | — | `e2e_spec022_postgres_worker_ingest.rs` |
| Entity identity | `contract_entity_identity.rs` | — |
| Merger batch | `contract_merger_graph_batch.rs` | — |
| Query modes | `contract_query_modes.rs` | — |
| Mix HTTP | `mix_mode_cache_separates_weight_skews` | `e2e_spec022_mix_mode_http_ordering.rs` |
| BM25 bootstrap | `contract_bootstrap_reranker.rs` | — |
| Global no N+1 | `contract_global_no_nplus1.rs` | — |
| AGE parameterized | `spec022_cypher_prepared_postgres.rs` | — |
| pgvector migration | `migration_bootstrap` unit tests | `migration_readiness_proof.rs` |

---

## Crate ownership map

```
edgequake-pipeline     extraction, merger, DefaultIngestionPersister
edgequake-core         orchestrator insert, saga comments, SDK query
edgequake-api          HTTP handlers, ingestion_persist port, injection
edgequake-query        modes, vector_queries, mix_weights, bootstrap
edgequake-storage      Postgres AGE, pgvector, community detection
edgequake-llm          BM25 reranker, providers
```

---

## Document cross-links

| From | To | Relationship |
|------|-----|--------------|
| RC-023-1 | [01-ingestion](./01-ingestion-first-principles.md#injection-path-autopsy-rc-023-1) | Ingestion autopsy |
| RC-023-2 | [02-query](./02-query-first-principles.md#global-mode-honest-assessment) | Global mode truth |
| RC-023-4,5,10 | [03 lens 4](./03-eight-lens-audit.md#lens-4-sota-rag-expert-june-2026) | SOTA gap |
| RC-023-6 | [03 lens 3](./03-eight-lens-audit.md#lens-3-graphrag-expert) | GraphRAG gap |
| All fixes | [05-improvement-plan](./05-improvement-plan.md) | Execution plan |
