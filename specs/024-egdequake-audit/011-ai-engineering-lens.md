# 011 — AI Engineering Lens

**Cross-ref:** [004 LightRAG](./004-lightrag-expert-lens.md) · [006 SOTA](./006-sota-rag-expert-lens.md) · [002 Ingestion](./002-ingestion-pipeline-audit.md)

---

## AI Pipeline Overview

```text
  INGEST                              QUERY
  ──────                              ─────

  Document text                       User question
       │                                   │
       v                                   v
  Chunker (token window)              Keyword LLM extract
       │                                   │
       v                                   v
  Entity/Rel LLM extract              3-level embeddings
  (tuple/JSON parsers)                     │
       │                              validate_keywords
       v                              (graph label filter)
  Optional gleaning                        │
  (multi-pass)                             v
       │                              Mode dispatch
       v                                   │
  Merge + normalize                        v
  (UPPERCASE entities)              Retrieve context
       │                                   │
       v                                   v
  Embed chunks/entities/rels          BM25 rerank (lexical)
       │                                   │
       v                                   v
  Index to pgvector + AGE             Truncate to token budget
                                           │
                                           v
                                      LLM generate answer
```

---

## LLM Call Budget Analysis

### Ingest (dominant cost)

Per document:

```text
  LLM_calls ≈ Σ_chunks (1 + gleaning_passes) × retry_factor
            + summarizer calls (if enabled in merger config)
```

**150KB document:** ~250 chunks → **250–750+ LLM calls** with gleaning.

This is **expected for LightRAG-quality graphs** — not a bug, but an **operational cost fact**.

Mitigations in code:
- `max_concurrent_extractions` semaphore
- Per-chunk timeout + retry cap
- Checkpoint (skip re-extract on worker retry)
- Adaptive chunk size in orchestrator (not all paths)

**AI engineering gap:** No ingest cost estimator exposed to API/admission. Users upload 500-page PDFs blindly.

### Query (secondary cost)

Per query:

```text
  LLM_calls = 1 (keyword extract, if enabled)
            + 1 (answer generation)
            + 0 (bypass mode)
```

Embeddings: 1 query embed + batched keyword embeds (3-level).

**Reasonable** for production RAG.

---

## Extraction Quality Mechanisms

| Mechanism | File | Assessment |
|-----------|------|------------|
| Tuple + JSON dual parser | pipeline extractors | Robust to LLM format drift ✓ |
| Entity normalization | `EntityId`, normalizer | Consistent graph keys ✓ |
| Gleaning multi-pass | orchestrator init | Quality ↑, cost ↑↑ |
| Per-chunk resilience | `extraction.rs` | Good (worker path) |
| Keyword validation | query `validate_keywords` | Reduces hallucinated entity search ✓ |
| Adaptive query mode | `keywords/intent.rs` | Can surprise users if mode omitted |

---

## Context Assembly (retrieval → prompt)

`balance_context` + truncation (`truncation.rs`):
- 30k token budget default
- 10k cap per section (entities, relationships, chunks)

**AI engineering view:** Fixed budgets are **coarse**. No dynamic allocation by query intent (e.g., procedural → more chunks, relational → more edges).

---

## Reranking as AI Component (F-11)

Production reranker: **BM25 lexical only**.

Implications:
- Semantic near-misses rank poorly after ANN
- Cross-encoder would recover many false ANN positives
- Same BM25 used for sparse retrieval and final rerank — **double lexical bias**

`EDGEQUAKE_RERANKER=cross_encoder` is a stub — **AI roadmap item left unimplemented**.

---

## Evaluation & Feedback Loops

Present in code:
- `rag_benchmark_recall.rs` — lightweight recall test
- Contract tests for BM25, RRF, global semantics
- Mock LLM default in tests

Missing in code:
- RAGAS / faithfulness metrics
- Human preference logging
- A/B mode comparison harness
- Gold Q&A sets in CI against real Postgres

**AI engineering grade cap:** Without eval harness, retrieval changes are **flying blind**.

---

## Provider Architecture

- Factory pattern via env (`OPENAI_API_KEY`, `EDGEQUAKE_LLM_PROVIDER`)
- Hybrid mode: separate embedding provider (`EDGEQUAKE_EMBEDDING_PROVIDER`)
- Workspace-scoped embedding in query path ✓

**Risk:** Mock provider in CI ≠ production LLM behavior. Contract tests validate structure, not extraction quality.

---

## Injection vs Document Sources

Injection content tagged `source_type: "injection"` — excluded from certain citation paths.

**AI concern:** Injection uses fail-fast `process` — one malformed chunk fails entire injection. Documents use resilience. **Inconsistent knowledge quality** across source types.

---

## AI Engineering Verdict

**Grade: B- (architecture) / C+ (eval & rerank)**

Strengths:
- Serious extraction pipeline (gleaning, tuple parse, normalization)
- Thoughtful query keyword validation
- Multi-mode retrieval reflects question taxonomy

Weaknesses:
- No cross-encoder rerank (SOTA gap)
- No production eval loop
- Ingest cost unbounded and path-dependent
- Lexical rerank dominates semantic retrieval

---

## AI-Specific Recommendations

| Priority | Action |
|:--------:|--------|
| P1 | Ship cross-encoder reranker (local `bge-reranker` or API) |
| P1 | Ingest token/LLM call budget in admission response |
| P2 | Intent-aware context budgets |
| P2 | Freeze 50-query gold set + recall@k in CI |
| P3 | Optional HyDE query expansion (env gated) |

See [012-improvement-plan.md](./012-improvement-plan.md).
