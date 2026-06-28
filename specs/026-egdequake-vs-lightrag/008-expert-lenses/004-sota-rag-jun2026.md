# 004 — SOTA RAG Expert Lens (June 2026)

**Cross-ref:** [004 Query](../004-query/001-query-comparison.md) · [007 Evaluations](../007-evaluations/001-evaluation-comparison.md)

**Finding:** C-07

---

## June 2026 SOTA Reference Stack

Based on production RAG patterns prevalent through mid-2026:

```text
  TIER 0 — Baseline RAG
  ─────────────────────
  chunk → embed → retrieve → generate

  TIER 1 — Hybrid RAG (2024-2025)
  ───────────────────────────────
  dense + sparse fusion, rerank, multi-index

  TIER 2 — Structured RAG (2025-2026)
  ───────────────────────────────────
  graph/metadata routing, intent classification,
  conversation-aware retrieval

  TIER 3 — Agentic RAG (2026)
  ───────────────────────────
  query decomposition, CRAG gates, iterative
  retrieval, self-correction, faithfulness verify
```

---

## Tier Placement

| Tier | LightRAG | EdgeQuake |
|------|:--------:|:---------:|
| T0 baseline | ✓ | ✓ |
| T1 hybrid | △ (dense only) | ✓ BM25+RRF+rerank |
| T2 structured | ✓ graph modes | ✓ graph + intent + history |
| T3 agentic | ✗ | ✗ |

**LightRAG: Tier 1.5** (graph structure elevates above naive).  
**EdgeQuake: Tier 2** (full hybrid stack + routing).  
**Neither: Tier 3.**

---

## Feature Checklist vs SOTA

| SOTA Pattern | LightRAG | EdgeQuake |
|--------------|:--------:|:---------:|
| Hybrid dense+sparse | ✗ | ✓ |
| Learned reranker | △ external | ✓ |
| Query routing | ✗ | ✓ intent |
| Multi-turn context | ✗ | ✓ |
| Query decomposition | ✗ | ✗ |
| CRAG / self-RAG | ✗ | ✗ |
| Iterative retrieval | ✗ | ✗ |
| Citation grounding | △ refs | △ refs |
| Faithfulness gate | ✗ | ✗ |
| Eval harness (RAGAS) | ✗ | △ skeleton |
| Cost-aware routing | ✗ | ✓ intent |

---

## Cost-Quality Frontier

```text
  Quality
    ▲
    │                              ┌─ SOTA agentic (cost $$$)
    │                         ┌────┘
    │                    ┌────┘ EdgeQuake Mix+RRF+rerank
    │               ┌────┘
    │          ┌────┘ LightRAG Mix
    │     ┌────┘
    │ ┌───┘ Naive RAG
    └──────────────────────────────> Cost / latency
```

EdgeQuake defaults **higher on the frontier** than LightRAG.  
Without intent routing engaged, it may be **over-engineered for simple factual Q&A**.

---

## SOTA Expert Verdict

| System | Grade | Position |
|--------|:-----:|----------|
| LightRAG | **C+** | Strong graph RAG, dated retrieval fusion |
| EdgeQuake | **B-** | Modern Tier-2 stack, no agentic |

**EdgeQuake is closer to June 2026 production RAG** than LightRAG reference — primarily due to BM25, RRF, rerank, and intent routing.

**Neither is research SOTA** (agentic, self-correcting systems).

---

## Minimum Path to B+ SOTA

1. RAGAS CI gate on golden set (EdgeQuake started)
2. Optional CRAG confidence check before LLM call
3. Query decomposition for multi-hop questions (without full agent loop)
4. Faithfulness post-check (NLI or LLM judge) on high-stakes answers

Do **not** jump to full agentic before Tier 2 eval gates pass.
