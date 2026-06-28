# 001 — First Principles Architecture

**Cross-ref:** [README](./README.md) · [002 Ingestion](./002-ingestion-pipeline-audit.md) · [003 Query](./003-query-retrieval-audit.md) · [012 Plan](./012-improvement-plan.md)

---

## 1. First Principle: What Problem Does EdgeQuake Solve?

**Problem:** Turn unstructured documents into **queryable knowledge** with three retrieval lenses:

1. **Direct text** (chunks)
2. **Entity neighborhood** (local / low-level)
3. **Thematic relationships** (global / high-level)

**Solution shape (from code):** Ingest → chunk → LLM extract → merge graph → embed three vector types → query with mode-specific views over **one index**.

This is LightRAG's insight, not GraphRAG's. Code says so explicitly in `edgequake-query/src/modes.rs`.

---

## 2. System Topology (post SPEC-024)

```text
                         INGESTION (converged)
    ┌──────────────────────────────────────────────────────────────┐
    │  HTTP: text | file | batch | injection                       │
    │         │                                                    │
    │         ├── KV: metadata + content + hash map                │
    │         └── enqueue_task() ──> Postgres tasks + channel      │
    │                                                              │
    │  Library: EdgeQuake::insert() [sync, no queue]               │
    │         └── adaptive chunk + optional Gleaning               │
    └──────────────────────────┬───────────────────────────────────┘
                               │
                               v
              ┌────────────────────────────────────┐
              │  Worker: DocumentTaskProcessor     │
              │  process_with_resilience_cancellable│
              │  + pipeline checkpoints            │
              └────────────────┬───────────────────┘
                               │
                               v
              ┌────────────────────────────────────┐
              │  DefaultIngestionPersister (SSOT)  │
              │  1. KV chunk records (text SSOT)   │
              │  2. pgvector chunk/entity/rel      │
              │  3. AGE graph merge (+ saga)       │
              │  4. debounced community refresh    │
              │  5. workspace cache invalidation   │
              └────────────────────────────────────┘


                         QUERY (single engine)
    ┌──────────────────────────────────────────────────────────────┐
    │  POST /api/v1/query → run_query_pipeline                     │
    │                                                              │
    │  prepare: keywords ∥ embed → QueryEmbeddings (3 vectors)    │
    │  retrieve: naive | local | global | hybrid | mix | bypass    │
    │  finalize: rerank → truncate (30K) → LLM                     │
    │                                                              │
    │  Default mode: Mix + RRF (not Hybrid round-robin)          │
    └──────────────────────────────────────────────────────────────┘
```

**Invariant (unchanged):** Query modes are **views**, not separate indexes. Quality ceiling = index quality + merge policy + LLM.

---

## 3. Single Sources of Truth (verified)

| Concern | SSOT | Evidence |
|---------|------|----------|
| Persist sequence | `edgequake-pipeline/src/persistence/ingestion_persister.rs` | KV → vectors → merge → debounced community |
| Chunk text | `edgequake-pipeline/src/chunk_storage.rs` | Vectors hold `content_ref`, not inline text |
| Task enqueue | `edgequake-api/src/state/mod.rs::enqueue_task` | All HTTP ingest paths |
| Duplicate re-ingest | `storage_helpers.rs::resolve_workspace_duplicate_for_reingestion` | file/text/batch parity |
| Workspace vectors | `edgequake_core::workspace_vector_resolve` | Library + API delegate |
| Query pipeline | `query_entry/query_pipeline.rs::run_query_pipeline` | prepare → retrieve → finalize |
| Hybrid merge | `hybrid_merge.rs::merge_hybrid_contexts` | Round-robin default |
| Mix fusion | `fusion.rs` + `EDGEQUAKE_MIX_FUSION` | RRF default |
| Community debounce | `community_index_service.rs` | 300s default window |
| Queue pressure | `task_queue_pressure.rs` | `/health` degraded on critical |

---

## 4. Trust Boundaries

```text
  Client ──HTTP──> edgequake-api
                        │
         ┌──────────────┼──────────────┐
         │              │              │
    TenantContext   Rate limits    Admission
         │              │              │
         v              v              v
    strict workspace   task queue    KV pre-write
    resolver (prod)    backpressure  (before worker)
         │                              │
         v                              v
    per-workspace pgvector + AGE    Worker isolation
         │                              │
         └──────────> QueryEngine (read-mostly)
```

**Remaining boundary risk (N-12):** KV metadata written at HTTP admission **before** worker success. Saga compensates merge failure inside persister, not admission orphan KV if worker never runs.

**Mitigation in code:** Orphan document recovery at startup (`main.rs`), task retry, checkpoint resume.

---

## 5. Saga vs Transaction (honest)

```text
  Step 1: KV chunk upsert          ─┐
  Step 2: pgvector upsert           │  Not 2PC
  Step 3: AGE merge                 │
  Step 4: on merge fail → compensate│  compensate_merge_failure()
  Step 5: schedule community refresh│
  Step 6: invalidate query cache    ─┘
```

**First-principle truth:** Multi-store consistency is **eventual + compensating**. Accept it or pay for distributed transactions (you won't in Postgres+AGE).

---

## 6. What SPEC-024 Fixed (don't re-litigate)

| SPEC-024 claim | Code proof |
|----------------|------------|
| Async file/batch upload | `file_upload.rs` → 202 + `TaskType::Insert` |
| Injection queued | `TaskType::KnowledgeInjection` |
| Workspace cache bust | `invalidate_workspace` in persister path |
| Hybrid round-robin | `hybrid_merge.rs` |
| Mix default | `QueryEngineConfig.default_mode = Mix` |
| BM25 all arms | `chunk_retrieval.rs` + `sparse_retrieval.rs` |
| graph_depth | `graph_hops.rs::edges_within_depth` |
| Modes split | `engine_impl/modes/*` (largest ~205 LOC) |

---

## 7. First-Principles Scorecard (2026-06-27)

| Dimension | 1–5 | Brutal note |
|-----------|:---:|-------------|
| Index correctness | **5** | Provenance hydration, `vector_type` filters, workspace SQL pushdown |
| Ingestion consistency | **4** | One HTTP runtime; library path still differs (N-02) |
| Query mode honesty | **5** | Global ≠ GraphRAG documented; Hybrid vs Mix distinguished |
| Operational scale | **4** | Debounced Louvain, queue pressure; triple-arm cost (N-04) |
| Code consolidation | **4** | Modes split; `text_insert.rs` still fat (N-08) |
| SOTA retrieval | **3** | Hybrid+RRF+rerank yes; agentic/conversational no |
| API truthfulness | **3** | `conversation_history` dead (N-01) |

---

## 8. Non-Negotiable Next Fixes

If you ship only three things after SPEC-024:

1. **Wire or remove `conversation_history`** — API contract lie is worse than missing feature.
2. **Port adaptive chunk + Gleaning to worker path** — library shouldn't be smarter than production HTTP.
3. **Batch graph edge reads** — `graph_hops` N+1 is the query hot-path cliff at scale.

See [012-improvement-plan.md](./012-improvement-plan.md).
