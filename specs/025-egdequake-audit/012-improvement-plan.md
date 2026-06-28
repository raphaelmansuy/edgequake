# 012 — First Principles Improvement Plan (SPEC-025 Implementation)

**Cross-ref:** [README finding matrix](./README.md#cross-reference-matrix) · [001 Architecture](./001-first-principles-architecture.md)

**Last updated:** 2026-06-27 (Phase 6 Sprint 2–3 implementation)  
**Principle:** One SSOT per concern · API must not lie · HTTP path = library intelligence

---

## First Principles (non-negotiable)

```text
  1. INGEST ONCE, PERSIST ONCE     → IngestionPersister + worker queue (R-02 ✅)
  2. CHUNK SSOT IN KV              → content_ref (R-08 ✅)
  3. SAME BRAIN ALL PATHS          → adaptive chunk + gleaning everywhere (N-02 ✅)
  4. API TRUTH                     → every request field used or removed (N-01 ✅)
  5. ADMISSION DRY                 → one enqueue helper (N-07 ✅)
  6. QUERY DEFAULT = RUNTIME       → Mix + RRF (N-11 ✅)
  7. MEASURE BEFORE SOTA CLAIMS    → RAGAS CI skeleton (N-10 ✅ 8.1)
```

---

## Implementation Status

| Phase | Item | Finding | Status | Evidence |
|-------|------|---------|:------:|----------|
| **5.1** | Wire `conversation_history` | N-01 | ✅ Done | `conversation_context.rs`, `query_pipeline.rs` |
| **5.2** | Adaptive chunk SSOT + worker path | N-02 | ✅ Done | `adaptive_chunking.rs`, `ingestion_pipeline.rs` |
| **5.3** | Gleaning on worker (metadata-driven) | N-02 | ✅ Done | Task metadata + `build_ingestion_pipeline` |
| **5.4** | `document_admission.rs` SSOT | N-07 | ✅ Done | file/text/batch delegate |
| **5.5** | `QueryMode` serde default = Mix | N-11 | ✅ Done | `modes.rs` |
| **6.1** | Slim task payload (KV ref only) | N-03 | ✅ Done | `text_insert_content.rs`, empty task `text` |
| **6.2** | Batch graph incident edges | N-06 | ✅ Done | `get_incident_edges_batch`, `graph_hops` |
| **6.3** | `community_id` indexed lookup | N-13 | ✅ Done | `NodeListFilter.community_ids`; `community_global` push-down |
| **6.4** | Cheap intent routing (skip triple-arm) | N-04 | ✅ Done | `intent.rs`; `contract_spec025_intent_routing.rs` |
| **6.5** | Injection list pagination | N-09 | ✅ Done | `injection_list.rs`; `limit`/`offset`/`has_more` |
| **6.6** | Split `text_insert.rs` | N-08 | ✅ Done | `processor/text_insert/` (all files ≤354 LOC) |
| **8.1** | RAGAS eval CI | N-10 | ✅ Done | `eval/golden_set.rs`, 50-case fixture, `contract_spec025_ragas_skeleton.rs` |
| **7.x** | GraphRAG honest track | R-04 | ⏳ Open | — |

---

## Phase 6 — Sprint 1 Completed (2026-06-27)

### 6.1 Slim task payload

Admission enqueues `text=""`; worker resolves via `resolve_text_insert_content()`.

### 6.2 Batch graph hops

`edges_within_depth()` uses `get_incident_edges_batch(frontier)` per BFS level.

### 6.4 Cheap intent routing

Exploratory→Naive, Comparative→Local; Mix reserved for Procedural adaptive path.

---

## Phase 6 — Sprint 2–3 Completed (2026-06-27)

### 6.3 Community lookup (N-13)

```text
  expand_global_context_with_communities()
           │
           └── list_nodes_filtered(community_ids = seed communities)
                    (replaces get_popular_nodes_with_degree scan)
```

Postgres push-down: `(properties->>'community_id')::bigint IN (...)`.

### 6.5 Injection pagination (N-09)

```text
  GET /injections?limit=50&offset=0
           │
           └── services/injection_list.rs (SSOT)
                    ├── prefix key scan
                    ├── sort by created_at
                    └── slice page (bounded response)
```

### 6.6 Text insert SRP split (N-08)

```text
  processor/text_insert/
    mod.rs        → orchestrator
    prepare.rs    → pipeline + preprocess
    extraction.rs → LLM + checkpoint
    persist.rs    → P-G2 persist + status gate
    finalize.rs   → lineage + metrics
    cancel.rs     → cancellation gate
```

### 8.1 RAGAS skeleton (N-10)

```text
  eval/golden_set.rs     → 50 Q&A fixture (embedded JSON)
  eval/metrics.rs        → keyword_recall + context_entity_recall
  contract_spec025_ragas_skeleton.rs
```

---

## Remaining

| # | Action | First principle | Acceptance |
|---|--------|-----------------|------------|
| 7.x | GraphRAG honest track | Category honesty | No GraphRAG claims without reports |
| 8.2 | Full RAGAS CI gate | Measure | Real LLM eval in nightly CI |

---

## Success Metrics (updated)

| Metric | Pre-025 | Post Phase 6 | Target |
|--------|:-------:|:------------:|:------:|
| Task payload 10MB doc | ~20MB | **<1KB** | ✅ |
| Graph BFS RTTs / depth | O(frontier) | **O(1)** | ✅ |
| Community expansion | O(popular scan) | **O(filtered page)** | ✅ |
| `text_insert.rs` max LOC | ~990 | **≤354** | ✅ |
| Golden Q&A regression set | 0 | **50** | ✅ |
| Injection list response | unbounded | **paginated** | ✅ |

---

## Lens Grade Projection

| Lens | Pre-025 | Post Phase 6 |
|------|:-------:|:------------:|
| Ingestion | A- | **A+** |
| Query / API truth | A / C+ | **A+** |
| Rust DRY/SOLID | A- | **A+** |
| O(n) | B | **A-** |
| AI engineering | B+ | **A-** |
| Overall | A- | **A+** |

---

## Priority Stack

```text
  Sprint 1: 6.1 + 6.2 + 6.4  ✅
  Sprint 2: 6.3 + 8.1         ✅
  Sprint 3: 6.6 + 6.5         ✅
  Next:     7.x GraphRAG honesty + 8.2 full RAGAS CI
```

**Code is law.** Each closed item requires a contract or E2E test.
