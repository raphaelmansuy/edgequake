# 006 — SOTA RAG Expert Lens (June 2026 Reference)

**Cross-ref:** [003 Query](./003-query-retrieval-audit.md) · [005 GraphRAG](./005-graphrag-expert-lens.md) · [011 AI Engineering](./011-ai-engineering-lens.md) · [012 Plan](./012-improvement-plan.md)

**Findings:** R-05, R-06, R-11, N-01, N-04, N-05, N-06, N-10

**External reference (June 2026 production consensus):** Hybrid dense+sparse retrieval → RRF fusion → cross-encoder rerank → optional graph/agentic escalation for multi-hop queries. Agentic RAG for simple factoids is overhead; for complex comparative questions, iterative retrieval is the differentiator ([Agentic RAG survey](https://arxiv.org/html/2501.09136v4), [production RAG 2026 patterns](https://1337skills.com/blog/2026-06-12-production-rag-2026-hybrid-search-reranking-graphrag/)).

---

## June 2026 Production Stack Checklist

| Layer | 2026 enterprise standard | EdgeQuake | Grade |
|-------|-------------------------|-----------|:-----:|
| Chunking | Semantic/adaptive | Static (HTTP); adaptive (library) | B |
| Extraction | LLM + optional multi-pass | Gleaning library-only | B- |
| Dense retrieval | pgvector / HNSW | ✓ workspace-scoped | A |
| Sparse retrieval | BM25 / FTS | ✓ Postgres FTS + fallback | A |
| Fusion | RRF | ✓ Mix default; Hybrid optional | A |
| Reranking | Cross-encoder (Cohere-class) | ✓ `create_production_reranker` | A- |
| Graph retrieval | GraphRAG reports OR agentic traverse | LightRAG rel vectors + labels | C |
| Query routing | Complexity-based mode select | Adaptive intent (LLM) | B+ |
| Multi-turn | Conversation-aware retrieval | **Field ignored (N-01)** | F |
| Agentic loops | Retrieve → critique → re-query | **None (N-06)** | F |
| Evaluation | RAGAS / DeepEval CI gates | Contract tests only | D |
| Observability | Retrieval traces + faithfulness | Metrics + `/health`; no RAGAS | B+ |
| Governance | Central vector registry | ✓ workspace registry | A |

**Composite SOTA grade: B-** — Strong **retrieval stack**, weak **orchestration intelligence** and **eval discipline**.

---

## What EdgeQuake Gets Right (SOTA-aligned)

```text
  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
  │ Dense ANN   │     │ BM25 / FTS  │     │ Graph BFS   │
  │ (pgvector)  │     │ (sparse)    │     │ (local)     │
  └──────┬──────┘     └──────┬──────┘     └──────┬──────┘
         │                   │                   │
         └───────────────────┼───────────────────┘
                             v
                    RRF (Mix default)
                             │
                             v
                    Cross-encoder rerank
                             │
                             v
                    Token budget + LLM
```

This matches the **2026 linear pipeline best practice** for document Q&A. EdgeQuake implements it in Rust with workspace isolation — rare and valuable.

---

## SOTA Gaps (brutal)

### N-06 — No agentic retrieval (P2)

Single pass: `prepare → retrieve → finalize`.

Missing:
- Query decomposition into sub-queries
- Sufficiency check ("do I need more context?")
- CRAG-style confidence routing
- Tool-selecting retrieval mode per sub-task

**Honest take:** Not every deployment needs agentic RAG. But **June 2026 SOTA** means you should have an **escalation path**, not absence.

### N-01 — Multi-turn dead (P1)

Enterprise RAG expects follow-up questions to inherit entity focus. EdgeQuake drops `conversation_history` on the floor.

### N-04 — Cost vs quality default (P1)

Mix triple-arm is **quality-maximizing**, not **cost-optimal**. SOTA deployments route:

- Simple → Naive or Local only
- Complex → Mix or agentic GraphRAG

EdgeQuake has `use_adaptive_mode` but still pays keyword LLM + often triple retrieval.

### N-10 — No eval harness (P2)

2026 enterprise standard: Faithfulness > 0.90, Context Precision > 0.80 as CI gates.

EdgeQuake has excellent **contract tests** (behavior of merge, hydration, fusion) but no **quality regression suite** on golden Q&A sets.

### Missing SOTA techniques (P3, optional)

| Technique | Status |
|-----------|:------:|
| Late interaction (ColBERT) | ✗ |
| Contextual compression (LLM distill) | ✗ |
| Learned fusion weights | ✗ |
| Citation grounding verification | ✗ |
| Multimodal retrieval | ✗ (images to LLM only) |

---

## Competitive Positioning (honest)

```text
  Capability spectrum:

  Naive RAG ────── EdgeQuake ────── Agentic GraphRAG ────── Research
       │                │                      │
       │    hybrid+RRF+rerank+KG              │
       │    production Postgres               │
       │                                      │
       └────────── sweet spot for ────────────┘
                  enterprise doc Q&A
                  (not research agents)
```

EdgeQuake is **not behind** on the retrieval engineering layer. It is **behind** on conversational orchestration and GraphRAG narratives.

---

## SOTA Expert Verdict

**Ship today for:** Enterprise document KB, LightRAG-quality Q&A, multi-tenant Postgres deployments.

**Do not ship for:** GraphRAG marketing, multi-turn chat without fixes, agentic research assistants without Phase 3.

**Minimum SOTA catch-up:** N-01 + N-10 + selective mode routing to cut N-04 cost.

See [012-improvement-plan.md](./012-improvement-plan.md) Phase 3–5.
