# 05 — Improvement Plan (SPEC-023 Phases I1–I10)

> **Spec**: 023-egdequake-audit  
> **Date**: 2026-06-27  
> **Status**: **I1–I5 + I3 implemented** · I4 partial · I6–I10 deferred

---

## Closure summary

| ID | Item | Status | Evidence |
|----|------|--------|----------|
| **I1** | Injection → IngestionPersister | ✅ **Done** | `injection.rs`, `tag_injection_sources`, `e2e_spec023_injection_persister.rs` |
| **I2** | Global mode honesty | ✅ **Done** | `modes.rs`, `query-modes.md`, `contract_global_mode_semantics.rs` |
| **I3** | Retrieval eval harness | ✅ **Done** | `rag_benchmark_recall.rs` |
| **I4** | Cross-encoder reranker option | ⚠️ **Partial** | `EDGEQUAKE_RERANKER` env + BM25 fallback; `contract_bootstrap_reranker_env.rs` |
| **I5** | RRF fusion for Mix | ✅ **Done** | `fusion.rs`, `EDGEQUAKE_MIX_FUSION=rrf`, `contract_rrf_fusion.rs` |
| I6 | Wire communities to global query | ⬜ Deferred | needs I3 baseline + product sign-off |
| I7 | AGE batch upsert parameterized | ⬜ Blocked | AGE MERGE + bound params |
| I8 | Slim vector metadata | ⬜ Deferred | |
| I9 | Batch LLM summarization | ⬜ Deferred | |
| I10 | Sparse BM25 retrieval arm | ⬜ Deferred | pairs with I5 RRF |

---

## I1 — Injection → IngestionPersister ✅

**Shipped**:

| Artifact | Change |
|----------|--------|
| `services/ingestion_persist.rs` | `tag_injection_sources`, `PersistIngestionParams::for_document` |
| `pipeline/.../ingestion_persist.rs` | `IngestionPersistContext::with_source_metadata` |
| `handlers/injection.rs` | Removed inline merger; uses `persist_ingestion_result` |
| `e2e_spec023_injection_persister.rs` | E2E + static contract |

**Acceptance** (verified):

- [x] Zero `KnowledgeGraphMerger::new` in `injection.rs`
- [x] Saga ordering (vectors before merge)
- [x] Cache invalidation via persister
- [x] E2E test green

---

## I2 — Global mode honesty ✅

**Shipped**: `modes.rs` rustdoc, `docs/deep-dives/query-modes.md` ASCII flow, `contract_global_mode_semantics.rs`

---

## I3 — Retrieval eval harness ✅

**Shipped**: `edgequake-query/tests/rag_benchmark_recall.rs` — recall@5 on mock fixture, mix naive-only arm.

Run:

```bash
cargo test -p edgequake-query --test rag_benchmark_recall
```

---

## I4 — Cross-encoder reranker ⚠️ partial

**Shipped**: `create_production_reranker()` reads `EDGEQUAKE_RERANKER=cross_encoder` and logs + falls back to BM25.

**Remaining**: HTTP cross-encoder provider implementation.

---

## I5 — RRF fusion for Mix ✅

**Shipped**:

| Artifact | Change |
|----------|--------|
| `edgequake-query/src/fusion.rs` | `reciprocal_rank_fusion`, `MixFusionMode` |
| `vector_queries.rs` | RRF branch when `EDGEQUAKE_MIX_FUSION=rrf` |
| `contract_rrf_fusion.rs` | Unit contracts |

Default remains **weighted min-max** (backward compatible).

---

## Deferred (I6–I10)

See prior plan sections — unchanged scope. I6 is the next product-facing GraphRAG increment.

---

## Test matrix (post-implementation)

| Test | Covers |
|------|--------|
| `e2e_spec023_injection_persister.rs` | I1 |
| `contract_global_mode_semantics.rs` | I2 |
| `rag_benchmark_recall.rs` | I3 |
| `contract_bootstrap_reranker_env.rs` | I4 hook |
| `contract_rrf_fusion.rs` | I5 |
| `contract_query_modes.rs` | Mix/Hybrid regression |
| `contract_ingestion_persistence.rs` | Persister SSOT |

---

## Ship verdict

**Ship all HTTP ingest + query paths.** Optional: enable `EDGEQUAKE_MIX_FUSION=rrf` in staging and compare recall@5.
