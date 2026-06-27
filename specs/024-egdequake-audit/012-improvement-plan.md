# 012 — Consolidated Improvement Plan

**Cross-ref:** All lenses · [README finding IDs](./README.md#cross-reference-matrix)  
**Last updated:** 2026-06-27 (SPEC-024 implementation pass 14 — library workspace vector registry W7)

---

## Implementation Status (2026-06-27)

| Phase | Item | Status | Evidence |
|-------|------|:------:|----------|
| **1.1** | Async `file_upload` + `batch_upload` | ✅ Done | `202 ACCEPTED`, `TaskType::Insert`, `e2e_spec024_async_file_upload.rs` |
| **1.2** | Injection → task queue | ✅ Done | `TaskType::KnowledgeInjection`, `injection_processing.rs`, `e2e_spec023_injection_persister.rs` (worker-backed) |
| **1.3** | Debounced community index | ✅ Done | `community_index_service.rs`; `EDGEQUAKE_COMMUNITY_REFRESH_DEBOUNCE_SECS` (default 300s) |
| **1.4** | Workspace-scoped cache invalidation | ✅ Done | `QueryResultCache::invalidate_workspace`, `contract_workspace_cache_invalidation.rs` |
| **1.5** | Enforce strict workspace in prod | ✅ Done | `main.rs` + tests use `with_workspace_support_strict`; injection worker uses strict resolver |
| **1.6** | Injection prefix index | ✅ Done | `keys_with_prefix` in `list_injections` / `delete_injection` |
| **2.0** | Hybrid LightRAG merge | ✅ Done | `hybrid_merge.rs` — round-robin KG-first, dedup, `max_chunks`; optional RRF via `EDGEQUAKE_HYBRID_FUSION=rrf` |
| **2.1** | Default query mode → Mix; default fusion → RRF | ✅ Done | `QueryEngineConfig.default_mode = Mix`; API default Mix; `EDGEQUAKE_MIX_FUSION` defaults RRF |
| **2.2** | Document Hybrid semantics | ✅ Done | Documented in `modes.rs` + module docs |
| **2.3** | BM25/FTS on local/global | ✅ Done | `chunk_retrieval.rs` + `contract_local_global_sparse.rs` |
| **2.4** | Cross-encoder reranker | ✅ Done | `edgequake-llm` factory (`create_production_reranker`); `contract_bootstrap_reranker_env.rs`; PR [#79](https://github.com/raphaelmansuy/edgequake-llm/pull/79) |
| **2.5** | Dedupe chunk storage | ✅ Done | `chunk_storage.rs` (SSOT); vector metadata `content_ref`; KV hydration; Postgres FTS KV join |
| **2.6** | Wire `max_results` | ✅ Done | `PreparedQuery.max_chunks` + API passes `max_results` to engine |
| **2.7** | `graph_depth` multi-hop traversal | ✅ Done | `graph_hops.rs` BFS via `get_node_edges`; `contract_graph_depth.rs` |
| **2.8** | Split `vector_queries.rs` | ✅ Done | `engine_impl/modes/*` (naive/local/global/hybrid/mix + shared `chunk_retrieval.rs`); largest file 205 LOC |
| **4.1** | Wire `rerank_time_ms` (engine → API) | ✅ Done | `QueryStats.rerank_time_ms`; `query_pipeline.rs`; `contract_rerank_stats.rs` |
| **4.2** | Extract `CommunityIndexService` | ✅ Done | `community_index_service.rs` (debounce scheduler SRP) |
| **4.3** | Health operational snapshot | ✅ Done | `/health` → `operational.task_queue` + `operational.query_engine`; `e2e_spec024_operational_excellence.rs` |
| **4.4** | Queue metrics E2E contract | ✅ Done | `GET /api/v1/pipeline/queue-metrics` in `e2e_spec024_operational_excellence.rs` |
| **4.5** | JSON structured logs | ✅ Done | `EDGEQUAKE_LOG_FORMAT` → `ObservabilityConfig::from_env()`; `/health` → `operational.observability` |
| **4.6** | Dual-write KV ↔ documents reconciliation | ✅ Done | `document_read_model.rs` merge + drift; `/health` → `operational.read_model`; `contract_spec024_document_read_model.rs` |
| **4.7** | Chat completion `rerank_time_ms` parity | ✅ Done | `chat/completion.rs` forwards `result.stats.rerank_time_ms`; `contract_rerank_stats.rs` |
| **4.8** | Fusion config in health snapshot | ✅ Done | `/health` → `query_engine.hybrid_fusion` + `mix_fusion`; `e2e_spec024_operational_excellence.rs` |
| **4.9** | Queue backpressure monitoring | ✅ Done | `task_queue_pressure.rs`; `/health` + queue-metrics `pressure`; degraded on critical |
| **4.10** | Split `migration_bootstrap` (SRP) | ✅ Done | `migration_bootstrap/{mod,reconcile,helpers}.rs`; `contract_spec024_task_queue_pressure.rs` |
| **4.11** | Prometheus task-queue gauges | ✅ Done | `edgequake-observability::record_task_queue_stats`; seeded at metrics bootstrap |
| **4.12** | E2E critical backlog degrades `/health` | ✅ Done | `spec024_health_degrades_on_critical_queue_backlog` in `e2e_spec024_operational_excellence.rs` |
| **4.13** | Per-migration reconcile modules | ✅ Done | `migration_bootstrap/reconcile/{m038,m040,m042,m043,m044,m045}.rs` (largest 196 LOC) |
| **1.7** | Duplicate re-ingest SSOT | ✅ Done | `resolve_workspace_duplicate_for_reingestion` in `storage_helpers.rs` |
| **1.8** | Batch upload re-ingest parity | ✅ Done | `batch_upload.rs`; `e2e_spec024_batch_upload_reingest.rs` |
| **2.9** | Health ingestion snapshot | ✅ Done | `/health` → `operational.ingestion` (worker_queue + IngestionPersister) |
| **2.10** | Health storage + scale signals | ✅ Done | `/health` → `operational.storage` + `community_refresh_scheduled_workspaces` |
| **2.11** | KV chunk persist in IngestionPersister | ✅ Done | `ingestion_persister.rs`; worker/orchestrator/injection wired; `spec024_orchestrator_kv_persist.rs` |
| **1.9** | Library workspace-scoped cache bust | ✅ Done | `orchestrator/ingestion.rs` → `invalidate_query_result_cache_for_workspace` |
| **1.10** | Library workspace vector registry (W7) | ✅ Done | `workspace_vector_resolve.rs`; `EdgeQuake::with_workspace_vector_support`; API delegates to SSOT |

**Tests added/updated:** `spec024_orchestrator_workspace_vector_registry.rs`, `contract_spec024_ingestion_uniformity.rs` (pass 14), (+ pass 13 tests)

---

## Executive Summary

EdgeQuake has **finished Phase 1**, **Phase 2 retrieval & storage hardening**, and **Phase 4 operational excellence**. Hybrid follows LightRAG round-robin semantics; Mix defaults to RRF; chunk text lives once in KV; `/health` exposes task-queue depth, query-engine config, observability runtime, and read-model reconciliation strategy for operators.

```text
  DONE (pass 14)                 NEXT                         LATER
  ──────────────                 ────                         ─────
  Library vector registry W7       publish edgequake-llm 0.6.26 Phase 3 GraphRAG
  KV chunks in IngestionPersister  consolidate postgres fixtures OTEL full stack
  Hybrid LightRAG + full E2E
```

---

## Phase 1 — Stop the Bleeding (P0) ✅

**Goal:** Eliminate correctness and scale cliffs without algorithm changes. **Complete.**

| # | Action | Finds | Files | Acceptance |
|---|--------|-------|-------|------------|
| 1.1 | Route `file_upload` + `batch_upload` through `TaskRuntime` (202 + task) | F-01 | `file_upload.rs`, `batch_upload.rs` | ✅ E2E: `e2e_spec024_async_file_upload.rs` |
| 1.2 | Replace injection `tokio::spawn` with queued task type | F-01 | `injection.rs`, `edgequake-tasks` | ✅ `TaskType::KnowledgeInjection`; worker E2E |
| 1.3 | Debounce `refresh_community_index` (e.g. 5min coalesce per workspace) | F-03 | `community_index_service.rs` | ✅ `EDGEQUAKE_COMMUNITY_REFRESH_DEBOUNCE_SECS` (default 300s) |
| 1.4 | Workspace-scoped query cache invalidation | F-09 | `query_result_cache.rs`, `ingestion_persist.rs` | ✅ `contract_workspace_cache_invalidation.rs` |
| 1.5 | Enforce `strict_workspace_mode=true` in production bootstrap | F-01 | `main.rs`, `processor/mod.rs` | ✅ Production + worker tests use strict mode |
| 1.6 | Injection list/delete: prefix index or Postgres table | F-10 | `injection.rs` | ✅ `keys_with_prefix` |
| 1.7 | Duplicate re-ingest SSOT (all upload paths) | F-01 | `storage_helpers.rs` | ✅ `resolve_workspace_duplicate_for_reingestion` |
| 1.8 | Batch upload re-ingest parity | F-01 | `batch_upload.rs` | ✅ `e2e_spec024_batch_upload_reingest.rs` |
| 1.9 | Library workspace-scoped cache bust | F-09 | `orchestrator/ingestion.rs` | ✅ `invalidate_query_result_cache_for_workspace` |
| 1.10 | Library insert → workspace vector registry | W7 / F-01 | `workspace_vector_resolve.rs`, `orchestrator/workspace_vector.rs` | ✅ `spec024_orchestrator_workspace_vector_registry.rs` |

---

## Phase 2 — Retrieval & Storage Hardening (P1) ✅

**Complete.** See Hybrid LightRAG semantics and chunk storage SSOT sections below.

| # | Action | Status |
|---|--------|:------:|
| 2.0–2.8 | Hybrid LightRAG, Mix/RRF default, BM25 local/global, reranker, chunk dedupe, graph_depth, modes split | ✅ |
| 2.9 | Health ingestion snapshot | ✅ |
| 2.10 | Health storage + community scale signal | ✅ |
| 2.11 | KV chunk persist inside IngestionPersister (all paths) | ✅ |

### Hybrid LightRAG semantics (implemented)

```text
  LOCAL arm ──┐
  GLOBAL arm ─┼──> round-robin per rank slot (local → global → naive)
  NAIVE arm ──┘         │
                        ├── dedup by chunk ID
                        ├── truncate to max_chunks
                        └── optional RRF: EDGEQUAKE_HYBRID_FUSION=rrf
```

**Mix mode** is the default query mode. **Hybrid** remains available for LightRAG round-robin merge.

---

## Phase 3 — GraphRAG / SOTA Optional Track (P2–P3, 8+ weeks)

Not started — community summaries, agentic retrieval, incremental Louvain.

---

## Phase 4 — Operational Excellence (P2) ✅

**Goal:** Operator visibility, truthful metrics, SRP for background services (008 system engineering, 018 observability).

| # | Action | Finds | Files | Acceptance |
|---|--------|-------|-------|------------|
| 4.1 | Wire `rerank_time_ms` engine → API (never fabricate) | 008 observability | `types.rs`, `query_pipeline.rs`, `query_execute.rs` | ✅ `contract_rerank_stats.rs` |
| 4.2 | Extract `CommunityIndexService` (debounce scheduler) | 010 DRY / SRP | `community_index_service.rs` | ✅ Scheduler out of `community_persist.rs` |
| 4.3 | `/health` operational snapshot | 008 backpressure | `health.rs`, `health_types.rs` | ✅ `operational.task_queue` + `query_engine` |
| 4.4 | Queue metrics E2E | FEAT0570 | `pipeline.rs` | ✅ `e2e_spec024_operational_excellence.rs` |
| 4.5 | JSON structured logs | 018 Phase 1 | `edgequake-observability`, `main.rs` | ✅ `EDGEQUAKE_LOG_FORMAT`; `/health` → `operational.observability` |
| 4.6 | Dual-write KV ↔ documents reconciliation | 008 G4 | `document_read_model.rs` | ✅ merge + drift SSOT; `/health` → `operational.read_model` |
| 4.7 | Chat `rerank_time_ms` parity with query API | 008 observability | `chat/completion.rs` | ✅ no fabrication; `contract_rerank_stats.rs` |
| 4.8 | Expose fusion env in health | 004 LightRAG ops | `health.rs`, `hybrid_merge.rs`, `fusion.rs` | ✅ `hybrid_fusion` + `mix_fusion` labels |
| 4.9 | Queue backpressure monitoring | 008 backpressure | `task_queue_pressure.rs` | ✅ `pressure` + thresholds; `/health` degraded on critical |
| 4.10 | Split `migration_bootstrap` | 010 SRP | `migration_bootstrap/*` | ✅ orchestration vs reconcile vs helpers |
| 4.11 | Prometheus task-queue gauges | 008 backpressure | `metrics.rs`, `task_queue_pressure.rs` | ✅ `edgequake_task_queue_{pending,processing,failed}` |
| 4.12 | E2E critical backlog → degraded `/health` | 008 ops | `e2e_spec024_operational_excellence.rs` | ✅ pressure=critical + status=degraded |
| 4.13 | Per-migration reconcile modules | 010 SRP | `reconcile/m038..m045.rs` | ✅ one module per migration family |

### Operational health shape (4.3–4.13)

```json
{
  "operational": {
    "task_queue": {
      "pending": 0,
      "processing": 0,
      "failed": 0,
      "pressure": "normal",
      "pending_warn_threshold": 100,
      "pending_critical_threshold": 500
    },
    "query_engine": {
      "default_mode": "mix",
      "reranker_configured": true,
      "community_refresh_debounce_secs": 300,
      "hybrid_fusion": "round_robin",
      "mix_fusion": "rrf",
      "community_refresh_scheduled_workspaces": 0
    },
    "observability": {
      "log_format": "json",
      "otel_enabled": false
    },
    "read_model": {
      "merge_strategy": "max(postgresql, kv)",
      "relational_backfill_enabled": true,
      "entity_count_graph_reconcile": true
    },
    "ingestion": {
      "execution_model": "worker_queue",
      "persist_ssot": "IngestionPersister",
      "duplicate_reingest_enabled": true
    },
    "storage": {
      "chunk_text_ssot": "kv",
      "vector_metadata_ref": "content_ref",
      "chunk_kv_in_persister": true
    },
    "migration": {
      "latest_version": 45,
      "source_ids_indexes_ready": true,
      "pgvector_iterative_scan_capable": true,
      "ready_for_traffic": true
    }
  }
}
```

**Env vars (4.9):** `EDGEQUAKE_QUEUE_PENDING_WARN` (default 100), `EDGEQUAKE_QUEUE_PENDING_CRITICAL` (default max(500, 5×warn)).

**Prometheus (4.11):** `edgequake_task_queue_pending`, `edgequake_task_queue_processing`, `edgequake_task_queue_failed` — updated on every `/health` probe and `GET /api/v1/pipeline/queue-metrics` via `publish_queue_observability`.

### DRY / SOLID notes (pass 14)

| Concern | SSOT |
|---------|------|
| Workspace vector resolve (library + API) | `edgequake_core::workspace_vector_resolve` |
| Library registry wiring | `EdgeQuake::with_workspace_vector_support` |
| Default workspace UUID alias | `default_workspace_uuid()` (core; API middleware delegates) |

### DRY / SOLID notes (pass 13)

| Concern | SSOT |
|---------|------|
| Chunk KV + vector persist sequence | `IngestionPersister::persist` (KV → vectors → merge) |
| Duplicate re-ingest (upload paths) | `resolve_workspace_duplicate_for_reingestion` |
| Workspace vector resolve (ingestion) | `workspace_vector_resolve::resolve_workspace_vector_storage` |
| Queue backpressure | `task_queue_pressure::assess_queue_pressure` |
| Queue observability publish | `task_queue_pressure::publish_queue_observability` |
| Prometheus queue gauges | `edgequake_observability::record_task_queue_stats` |
| Migration orchestration | `migration_bootstrap/mod.rs` (~565 LOC) |
| Migration reconcile hooks | `migration_bootstrap/reconcile/m038..m045.rs` |
| Migration shared helpers | `migration_bootstrap/helpers.rs` |
| Community debounce scheduler | `edgequake_storage::community_index_service` |
| Community label persist | `edgequake_storage::community_persist` |
| Rerank timing (query + chat) | `QueryStats.rerank_time_ms` |
| Log format runtime | `ObservabilityConfig::from_env()` → `/health` operational.observability |
| Hybrid merge (LightRAG) | `hybrid_merge::round_robin_merge_chunks` |
| Hybrid fusion env | `hybrid_fusion_mode_from_env()` → health `hybrid_fusion` |
| Mix fusion env | `mix_fusion_mode_from_env()` → health `mix_fusion` |
| Chunk KV + hydration | `chunk_storage` + `chunk_hydration` |
| KV ↔ PG document drift | `document_read_model::{merge_*, detect_document_drift}` |
| Query mode modules | `engine_impl/modes/*` (dead `strategies/` removed) |
| Ops visibility | `/health` + `/api/v1/pipeline/queue-metrics` + Prometheus gauges |

---

## Success Metrics (code-verifiable)

| Metric | Baseline | Target | Current (2026-06-27) |
|--------|----------|--------|----------------------|
| Ingest execution paths | 4 | 1 (+ library) | **1 worker queue (+ library)** |
| Upload duplicate handling | 3 divergent | 1 SSOT helper | **✅** |
| Louvain runs per 100 ingests | ~100 | ≤2 (debounced) | **~1–2** per debounce window |
| Query cache cross-workspace bust | 100% | 0% on single-ws ingest | **0%** |
| Hybrid merge | round-robin no cap | LightRAG + max_chunks | **✅** |
| `rerank_time_ms` API (query + chat) | fabricated / missing | engine truth | **✅** |
| Health task-queue visibility | none | pending/processing/failed | **✅** |
| Queue backpressure labels | none | normal/elevated/critical | **✅** |
| Prometheus task-queue gauges | none | pending/processing/failed | **✅** |
| Migration bootstrap modularity | 1×1014 LOC monolith | SRP split | **✅ per-migration reconcile** |
| Health observability snapshot | none | log_format + otel_enabled | **✅** |
| KV↔PG read-model reconciliation | best-effort | merge + operator visibility | **✅** |
| Chunk metadata inline content | 100% | 0% (new ingests) | **✅ content_ref** |
| Chunk KV written outside persister | worker-only | all paths via persister | **✅** |
| Library cache bust scope | global | workspace when configured | **✅** |
| Library vector persist target | global default | workspace registry | **✅** |

---

## What NOT to Do

1. **Do not** market Global mode as GraphRAG without Phase 3.
2. **Do not** add new query modes before splitting `vector_queries.rs`. ✅ Split complete.
3. **Do not** add more ingestion entry points — redirect to queue.
4. **Do not** run full Louvain synchronously on persist hook at scale. ✅ Fixed via debounce.
5. **Do not** fabricate `rerank_time_ms` at the API layer. ✅ Fixed (4.1, 4.7).

---

## Lens Grades (post SPEC-024 pass 14)

| Lens | Before | Pass 9 | Pass 10 | Pass 11 | Pass 12 | Pass 13 | Pass 14 |
|------|:------:|:------:|:-------:|:-------:|:-------:|:-------:|:-------:|
| Ingestion uniformity | D | **B+** | **B+** | **B+** | **A-** | **A** | **A** |
| Query Hybrid fidelity | C | **A** | **A** | **A** | **A** | **A** | **A** |
| Scale (community/cache) | C- | **B+** | **B+** | **B+** | **A-** | **A** | **A** |
| Operational visibility | D | **A** | **A+** | **A+** | **A+** | **A+** | **A+** |
| Storage efficiency | D | **B+** | **B+** | **B+** | **A-** | **A** | **A** |
| Code modularity (query + migrations) | C | **A-** | **A** | **A+** | **A+** | **A+** | **A+** |
| System engineering (008) | B-/D | **A-** | **A** | **A** | **A** | **A** | **A** |
| Rust DRY/SOLID (010) | B- | **A-** | **A** | **A+** | **A+** | **A+** | **A+** |
| Overall | B- | **A** | **A** | **A** | **A** | **A** | **A** |

**Lens doc refresh:** [002-ingestion-pipeline-audit.md](./002-ingestion-pipeline-audit.md)

**Next recommended sprint:** Phase 3 GraphRAG track OR publish `edgequake-llm` 0.6.26. Remaining P2: consolidate postgres test fixtures (spec013 + spec022).
