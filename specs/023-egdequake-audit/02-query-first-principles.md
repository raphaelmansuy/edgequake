# 02 — Query Pipeline (First Principles)

> **Spec**: 023-egdequake-audit  
> **Code anchors**: `edgequake-query/src/engine_impl/`, `edgequake-api/handlers/query/`

---

## First principle

**Query answers: "Which evidence should the LLM see?"**

Retrieval is not generation. EdgeQuake separates:

1. **Prepare** — keyword extraction (LLM), dual embeddings (low/high level)
2. **Retrieve** — mode-specific context assembly
3. **Finalize** — rerank, truncate, balance token budget, optional LLM answer

Evidence: `query_entry/query_pipeline.rs:38-56`.

---

## Mode architecture

```
                    QueryRequest
                         │
                         ▼
              ┌──────────────────┐
              │ prepare_query    │
              │ • keywords (LLM) │
              │ • embed low/high │
              └────────┬─────────┘
                       │
     ┌─────────┬───────┼───────┬─────────┐
     ▼         ▼       ▼       ▼         ▼
  naive     local   global  hybrid      mix
     │         │       │       │         │
     │         │       │       │         │
     └─────────┴───────┴───────┴─────────┘
                       │
                       ▼
              ┌──────────────────┐
              │ rerank (BM25)    │
              │ balance_context  │
              │ LLM generate     │
              └──────────────────┘
```

---

## Mode semantics (code is law)

| Mode | What it actually does | LightRAG name | GraphRAG name |
|------|----------------------|---------------|---------------|
| **Naive** | Chunk ANN only | naive | vector RAG |
| **Local** | Entity ANN → linked chunks → chunk ANN re-rank | local | entity-centric |
| **Global** | Relationship-vector ANN → entities → batch degrees; fallback: popular nodes | global-ish | **NOT** community reports |
| **Hybrid** | Round-robin interleave local + global + naive chunks | hybrid | multi-index fusion (naive) |
| **Mix** | Weighted min-max score blend of three arms | mix (FEAT0105) | weighted hybrid |
| **Bypass** | No retrieval | bypass | — |

**RC-023-2**: Doc/comments say global = "community-based" (`modes.rs:64-66`) but implementation uses **relationship embeddings + degree fallback**, not Leiden/Louvain community summaries.

Evidence: `vector_queries.rs:256-378`.

---

## Local mode (strong)

```
keywords + low_level embedding
         │
         ▼
  entity vector ANN (type=entity filter)
         │
         ▼
  collect chunk IDs from entity metadata
         │
         ▼
  chunk vector query_filtered(by IDs)  ← cosine re-rank within candidates
         │
         ▼
  batch get_nodes + node_degrees_batch  ← no N+1
```

**Grade: A** — batched graph reads, SQL-layer tenant/workspace filter.

---

## Global mode (honest assessment)

```
high_level embedding
         │
         ▼
  relationship vector ANN (type=relationship)
         │
         ├── hits ──► extract src/tgt entities + rel descriptions
         │
         └── empty ──► get_popular_nodes_with_degree (degree fallback)
                       + get_edges_for_nodes_batch
         │
         ▼
  batch get_nodes + node_degrees_batch
         │
         ▼
  chunk collection from entity-linked chunk IDs
```

**What works**: Relationship-vector global search is a legitimate LightRAG pattern when relationship embeddings exist.

**What's missing vs GraphRAG**: Precomputed **community reports** (hierarchical summaries). EdgeQuake has `community.rs` (Louvain/modularity) but **zero query integration**.

**Grade: B** — functional but mislabeled; fallback to degree-popular nodes is a blunt instrument on sparse graphs.

---

## Hybrid vs Mix

| | Hybrid | Mix |
|---|--------|-----|
| Fusion | Round-robin interleave | Min-max normalize per arm × weights |
| Weights | implicit equal | `mix_weights.rs` SSOT + HTTP override |
| Cache key | mode only | includes weight skew (P-H6 fix) |
| SOTA alignment | simple | closer to learned weighting, not RRF |

**RC-023-5**: June 2026 production stacks often use **RRF** across sparse+dense+graph arms. EdgeQuake Mix uses score normalization — works on fixtures, can rank-unstable when score distributions differ wildly.

Evidence: `vector_queries.rs:638-664`, `mix_weights.rs:26-51`.

---

## Reranking layer

```
retrieved chunks
       │
       ▼
BM25Reranker (enhanced: stemming + Unicode norm)
       │
       ▼
filter by min_rerank_score
       │
       └── empty? ──► fallback: keep top_k original (OODA-231)
```

**RC-023-4**: SOTA 2026 expects **cross-encoder reranker** (e.g. `bge-reranker`, Cohere rerank) on top-50 candidates. BM25 is cheap and deterministic but misses semantic reordering that cross-encoders fix.

Evidence: `bootstrap.rs:17-24`, `reranking.rs:9-78`.

Production bootstrap **does** attach reranker — good. It's just not neural.

---

## Caching (correctness)

| Cache | Key includes | Invalidation |
|-------|--------------|--------------|
| Embedding cache | query text hash | TTL / process |
| Result cache | mode + weights + query + workspace | `invalidate_query_result_cache()` on ingest |

**Post SPEC-022**: Upload paths invalidate. **Injection does not** (RC-023-1 side effect).

---

## HTTP vs SDK parity

| Concern | Status |
|---------|--------|
| BM25 reranker | ✅ shared `build_production_query_engine` |
| Mix weights | ✅ HTTP forwards to engine |
| Workspace vector storage | ✅ strict resolution in handlers |
| Context-only mode | ✅ skips LLM, returns sources |

Evidence: `query_bootstrap.rs`, `contract_bootstrap_reranker.rs`, `e2e_spec022_mix_mode_http_ordering.rs`.

---

## O(n) query complexity

| Operation | Complexity | Status |
|-----------|------------|--------|
| Entity vector search | O(log N) ANN + filter | pgvector HNSW |
| `get_nodes_batch(E)` | O(1) RTT | ✅ |
| `node_degrees_batch(E)` | O(1) RTT | ✅ fixed P-G3 |
| Keyword validation | O(K) parallel graph searches | acceptable |
| BM25 rerank | O(k × term) | linear in top-k |
| Mix mode | O(chunks) hash merge | ✅ |

---

## Cross-refs

| Topic | See also |
|-------|----------|
| SOTA 2026 comparison | [03-eight-lens-audit.md](./03-eight-lens-audit.md#lens-4-sota-rag-expert-june-2026) |
| GraphRAG communities gap | [03-eight-lens-audit.md](./03-eight-lens-audit.md#lens-3-graphrag-expert) |
| Eval harness gap | [05-improvement-plan.md](./05-improvement-plan.md#i3-retrieval-eval-harness) |
