# 006 — SOTA RAG Expert Lens (June 2026 reference)

**Cross-ref:** [003 Query](./003-query-retrieval-audit.md) · [004 LightRAG](./004-lightrag-expert-lens.md) · [011 AI Engineering](./011-ai-engineering-lens.md)

---

## SOTA RAG Stack (2024–2026, code-relevant)

What production RAG systems commonly implement by mid-2026:

| Layer | SOTA practice | EdgeQuake code |
|-------|---------------|----------------|
| Chunking | Semantic / adaptive | Adaptive in orchestrator only; API file upload uses workspace default |
| Embedding | Multi-model, matryoshka | Single workspace embedding provider |
| Dense retrieval | HNSW / IVF + filters | pgvector HNSW/IVF + `MetadataFilter` ✓ |
| Sparse retrieval | BM25 / SPLADE / FTS | Postgres FTS naive-only; BM25 rerank elsewhere |
| Fusion | RRF, convex combo, learned | RRF opt-in (`EDGEQUAKE_MIX_FUSION=rrf`); Hybrid default is round-robin |
| Rerank | Cross-encoder (bge, Cohere, Jina) | BM25 only; cross_encoder env stub |
| Query transform | HyDE, decomposition, step-back | Not present |
| Graph RAG | GraphRAG / LightRAG / PathRAG | LightRAG-style; not GraphRAG |
| Agentic | Tool-use retrieval loops | Not present |
| Cache | Semantic + exact match | Embedding cache ✓; result cache context_only only |
| Eval | BEIR, MTEB, RAGAS | `rag_benchmark_recall.rs` — mock/lightweight |

---

## Scorecard vs SOTA

```text
  SOTA dimension          EdgeQuake    Gap severity
  ─────────────────       ─────────    ────────────
  Hybrid dense+sparse     ██░░░ 40%    P1 (naive only)
  Score-based fusion      ███░░ 60%    P1 (Mix not default)
  Neural rerank           █░░░░ 20%    P1 (stub)
  Multi-hop graph         ██░░░ 40%    P2 (1-hop only)
  Community global        ██░░░ 40%    P2 (labels not summaries)
  Ingestion reliability   ███░░ 60%    P0 (path split)
  Postgres ANN tuning     ████░ 80%    — (strong)
  Workspace isolation     ████░ 80%    P0 if misconfigured
  Observability           ██░░░ 40%    P2 (not re-audited)
```

---

## Fusion: Where EdgeQuake Falls Short

**Naive mode** (`sparse_retrieval.rs`):

```text
  vector ANN ──┐
               ├── RRF (if env) ──> fused ranking
  FTS/BM25  ───┘
               or
               └── weighted fallback: SPARSE WINS ENTIRELY (not blend)
```

**Mix mode** (`fusion.rs`, `mix_weights.rs`):
- Weighted min-max normalization across local/global/naive chunk scores
- RRF when `EDGEQUAKE_MIX_FUSION=rrf`

**Hybrid mode (DEFAULT):**
```text
  local_chunks[0], global_chunks[0], naive_chunks[0],
  local_chunks[1], global_chunks[1], naive_chunks[1], ...
```
This is **not SOTA fusion**. It ignores scores entirely.

**SOTA expectation:** RRF or learned fusion as default; Hybrid deprecated.

---

## Reranking Gap (F-11)

`bootstrap.rs` — cross-encoder path logs warning and uses BM25.

SOTA systems apply cross-encoder **after** candidate generation (20–100 docs). EdgeQuake applies BM25 twice (sparse rank + finalize rerank) — **lexical-only ceiling**.

Impact: entity-heavy queries where semantic match matters more than term overlap.

---

## Retrieval-Only Benchmarks

`edgequake-query/tests/rag_benchmark_recall.rs` exists — signals intent to measure recall. Without gold datasets wired to CI against Postgres, **SOTA claims are unverified**.

---

## What EdgeQuake Does Better Than Average SOTA Implementations

1. **Provenance chunk hydration** — fixes entity-vector pollution (many naive RAG ports miss this)
2. **SQL-pushdown metadata filters** — tenant/workspace/document scoping in ANN query
3. **Transaction-scoped HNSW tuning** — `SET LOCAL` per query
4. **Unified persist saga** — cross-store consistency with compensation
5. **Honest mode documentation** in code comments

These are **engineering maturity** wins, not algorithmic SOTA wins.

---

## SOTA Expert Verdict

**Grade: C+ overall RAG quality potential, B+ infrastructure**

EdgeQuake is a **well-operationalized LightRAG** with **selective SOTA features** (FTS, optional RRF) bolted on. It is **not** competitive with best-in-class retrieval stacks on fusion + rerank without:

1. Default Mix + RRF
2. Cross-encoder reranker (local model or API)
3. Sparse retrieval in local/global arms
4. Unified ingestion path (quality in = quality out)

---

## Minimum SOTA Upgrade Path (code-only scope)

| Priority | Change | Files |
|:--------:|--------|-------|
| P1 | Default `QueryMode::Mix` + RRF | `engine_impl/mod.rs`, API default |
| P1 | Integrate `edgequake-llm` cross-encoder or API rerank | `bootstrap.rs`, `reranking.rs` |
| P1 | Extend `fuse_vector_and_bm25_chunks` pattern to local/global | `vector_queries.rs`, new module |
| P2 | Wire `max_results` → `max_chunks` or remove | API + `types.rs` |
| P2 | BEIR-style contract test with frozen corpus | new test fixture |
| P3 | HyDE optional pre-retrieval embed | new `query_transform.rs` |

Full plan: [012-improvement-plan.md](./012-improvement-plan.md)
