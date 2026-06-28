# 003 — Query & Retrieval Audit

**Cross-ref:** [004 LightRAG](./004-lightrag-expert-lens.md) · [005 GraphRAG](./005-graphrag-expert-lens.md) · [006 SOTA](./006-sota-rag-expert-lens.md) · [009 O(n)](./009-complexity-on-lens.md)

**Findings:** R-05, R-06, R-07, R-10, R-11, R-12, N-01, N-04, N-05, N-06, N-11, N-13

---

## 1. Pipeline Architecture

**SSOT:** `engine_impl/query_entry/query_pipeline.rs::run_query_pipeline`

```text
                    run_query_pipeline
                           │
         ┌─────────────────┼─────────────────┐
         v                 v                 v
    pipeline_prepare   pipeline_retrieve  pipeline_finalize
         │                 │                 │
    keywords ∥ embed    mode dispatch     rerank
    QueryEmbeddings     (6 modes)         truncate (30K)
    mode resolution                       LLM answer
```

### Prepare

- Parallel: LLM keyword extraction + optional `embed_one`
- `QueryEmbeddings::compute_with_query_vec` → 3 vectors (query / high_level / low_level)
- Mode: explicit → adaptive intent → **`Mix` default** (`QueryEngineConfig`)

### Retrieve (modes/)

| Mode | Mechanism | BM25 | Graph |
|------|-----------|:----:|:-----:|
| Naive | Chunk ANN | ✓ | — |
| Local | Entity ANN → BFS → provenance chunks | ✓ | `graph_depth` |
| Global | Rel ANN → community expand → chunks | ✓ | co-`community_id` |
| Hybrid | 3 arms ∥ → round-robin merge | ✓ | all |
| Mix | 3 arms ∥ → RRF default | ✓ | all |
| Bypass | No retrieval | — | — |

### Finalize

- `filter_context_by_document_ids`
- Cross-encoder rerank (`EDGEQUAKE_RERANKER=cross_encoder`)
- `balance_context` — 10K entity + 10K rel + remainder chunks
- LLM generation or `context_only` / `prompt_only` branches

---

## 2. SPEC-024 Wins (verified)

### R-05 — Mix + RRF default ✅

```158:158:edgequake/crates/edgequake-query/src/engine_impl/mod.rs
            default_mode: QueryMode::Mix,
```

API mirrors: `query_execute.rs` defaults to Mix when mode omitted.

### R-10 — Hybrid round-robin ✅

`hybrid_merge.rs` — local → global → naive per rank slot, dedup, `max_chunks` cap. Optional `EDGEQUAKE_HYBRID_FUSION=rrf`.

### R-06 — BM25 on all arms ✅

`chunk_retrieval.rs` + `sparse_retrieval.rs` — Postgres FTS when native; in-memory BM25 fallback.

### R-07 — Config wired ✅

- `graph_depth` → `graph_hops::edges_within_depth`
- `max_results` → `PreparedQuery.max_chunks`

### R-11 — Cross-encoder reranker ✅

`bootstrap.rs` → `create_production_reranker`; `QueryStats.rerank_time_ms` propagated to API (no fabrication).

### R-12 — Modular modes ✅

`engine_impl/modes/*` — dead `vector_queries.rs` monolith removed.

---

## 3. Open Gaps (brutal)

### N-01 — `conversation_history` is dead (P1) ✗

Defined in `types.rs`, accepted by API, **zero references** in `engine_impl/`.

```text
  Client sends history ──> QueryRequest ──X──> (ignored)
                                    │
                                    └──> LLM prompt (query only)
```

**This is a protocol lie.** Worse than unsupported: clients believe multi-turn RAG works.

### N-04 — Default Mix = 3× retrieval (P1) ⚠

`mix.rs` / `hybrid.rs` use `tokio::join!` on local + global + naive.

Each arm can trigger:
- Vector ANN queries (entity, rel, chunk)
- BM25 candidate pool (`max_chunks × 5` default)
- Graph BFS + community scan (global)
- KV hydration batch

**Quality trade accepted; cost trade often ignored.** At 100 QPS this is 300 vector queries + 300 BM25 pools per second unless mode routing throttles.

**Mitigation exists but not default:** `use_adaptive_mode` can pick Local-only for factual queries — still LLM keyword call every time.

### N-05 — Global ≠ GraphRAG (P2) ✗ by design

Documented in `modes.rs`. Global = relationship vector ANN + `community_global` entity co-membership. **No community reports.**

Do not sell this as GraphRAG. See [005-graphrag-expert-lens.md](./005-graphrag-expert-lens.md).

### N-06 — Single-shot retrieval only (P2) ✗

No query decomposition, no CRAG confidence gate, no second-pass retrieval. Prepare → retrieve → generate once.

June 2026 SOTA expects **selective** agentic escalation, not universal loops — but EdgeQuake has **zero** loop infrastructure.

### N-11 — Serde default confusion (P3) ✗

`QueryMode` enum `#[default]` = Hybrid (serde). Runtime default = Mix.

API consumers reading OpenAPI/serde docs get wrong default.

### N-13 — `community_global` scan cost (P2) ✗

`community_global.rs` calls `get_popular_nodes_with_degree(max_entities × 2)` to find co-community members.

At large graphs this is **O(popular scan)**, not index lookup by community.

---

## 4. Shared Chunk Path (strength)

```text
  entity/rel source_chunk_ids
           │
           v
  append_score_ranked_chunks (chunk_retrieval.rs)
           │
           ├── vector query with ID allowlist + embedding re-rank
           ├── optional BM25 fusion (sparse_retrieval)
           └── chunk_hydration (KV batch)
```

This is **correct LightRAG provenance retrieval**. EdgeQuake exceeds stock LightRAG on sparse fusion here.

---

## 5. Query Cost Model (first principles)

```text
  Per Mix query (worst case):

  LLM calls:  1 keyword extract + 1 answer (+ reranker API if cross-encoder)
  Embeddings: 1 query + 2 keyword texts (batched) = up to 4 embed calls
  Vector:     3 arms × (entity + rel + chunk queries) ≈ 9 ANN ops
  Graph:      BFS depth×frontier × get_node_edges (N+1)
  BM25:       up to 3 × (max_chunks × multiplier) candidates
  KV:         1 hydration batch
```

Caches mitigate repeats: keyword cache 24h, embedding cache 1h, result cache 5min (context_only only).

---

## 6. Brutal Verdict

**Retrieval algorithm: A** — Faithful LightRAG with BM25, RRF, reranker, multi-hop, hydration.  
**Production economics: B** — Default Mix is expensive; adaptive mode underused.  
**API honesty: C+** — Dead conversation field; serde/runtime default mismatch.

**Top fixes:** N-01, N-04 (mode routing default to cheaper path for simple queries), N-06 (Phase 3 agentic track).
