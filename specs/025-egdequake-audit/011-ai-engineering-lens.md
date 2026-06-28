# 011 — AI Engineering Lens

**Cross-ref:** [003 Query](./003-query-retrieval-audit.md) · [004 LightRAG](./004-lightrag-expert-lens.md) · [006 SOTA](./006-sota-rag-expert-lens.md)

**Findings:** R-05, R-06, R-11, N-01, N-02, N-04, N-06, N-10

---

## AI Pipeline Map

```text
  INGEST (LLM-heavy)                 QUERY (LLM-heavy)
  ─────────────────                  ─────────────────

  chunk text                         extract keywords (LLM)
       │                                   │
       v                                   v
  extract entities/rel (LLM×chunks)   embed query + kw texts
       │                                   │
       v                                   v
  optional Gleaning (library)         retrieve (no LLM)
       │                                   │
       v                                   v
  embed entities/rels/chunks          rerank (cross-encoder)
       │                                   │
       v                                   v
  persist                             generate answer (LLM)
```

**Cost drivers:** Ingest = O(chunks) LLM calls. Query = 2+ LLM calls + 3× retrieval (Mix).

---

## Extraction Quality

| Feature | HTTP worker | Library SDK | Impact |
|---------|:-----------:|:-----------:|--------|
| Resilient per-chunk extract | ✓ | ✓ | Partial failure tolerance |
| Gleaning multi-pass | ✗ | ✓ | **N-02 — recall gap** |
| Adaptive chunk size | ✗ | ✓ | **N-02 — boundary errors** |
| Tuple + JSON parsers | ✓ | ✓ | Robust extraction |
| Table preprocessing | ✓ | ? | Excel restructuring |

**Brutal truth:** Production API path may under-extract vs SDK on hard documents. Measure before claiming parity.

---

## Retrieval Intelligence

### Keyword extraction (strength)

- Dual-level high/low keywords
- Graph validation drops hallucinated entity terms
- `QueryIntent` drives adaptive mode

### Embedding strategy (strength)

- Three query vectors mapped to three index types — correct LightRAG design
- `CachingEmbeddingProvider` — 10K / 1h TTL

### Fusion + rerank (strength)

- RRF default (Mix) — SOTA-aligned
- Cross-encoder reranker — fixes ANN ordering errors
- BM25 catches exact identifiers vectors miss

### Missing AI patterns

| Pattern | Status | Finding |
|---------|:------:|---------|
| Conversation context | ✗ | N-01 |
| Query rewrite loop | ✗ | N-06 |
| Answer faithfulness check | ✗ | N-10 |
| Contextual compression | ✗ | Truncate only |
| Citation verification | ✗ | Sources post-hoc |

---

## Prompt / Context Engineering

`balance_context` — 30K token budget split entity/rel/chunk. Matches LightRAG rationale in comments.

**Risk:** Mix mode fills chunk budget with fused arms — good recall, possible **entity dilution** if chunk arm dominates RRF ranks.

**Mitigation in code:** Separate entity/rel union in Mix from chunk fusion — entities not tripled, only chunks are.

---

## Evaluation Discipline (N-10)

```text
  What exists:
    ✓ Contract tests (behavioral invariants)
    ✓ E2E ingest/query paths
    ✓ Mock LLM default in CI

  What is missing:
    ✗ Golden dataset Q&A regression
    ✗ RAGAS faithfulness / context precision gates
    ✗ A/B mode comparison harness
    ✗ Extraction recall metrics per doc type
```

**AI engineering without eval is prompt gambling.** EdgeQuake has strong **unit** tests, weak **quality** tests.

---

## Provider Architecture (strength)

- Mock default in CI
- OpenAI / Ollama / hybrid embedding mode (SPEC-033)
- Workspace-scoped pipeline resolver (strict prod)
- Separate query vs pipeline LLM providers (documented operational hazard)

---

## AI Engineering Verdict

**Grade: B+**

| Sub-area | Grade |
|----------|:-----:|
| Index + retrieval stack | A |
| Extraction parity | B- (N-02) |
| Generation context | B+ |
| Multi-turn | F (N-01) |
| Eval / feedback loops | D (N-10) |

**Highest ROI AI work:**

1. Wire conversation history into keyword + retrieval bias (N-01)
2. Port Gleaning to worker pipeline (N-02)
3. Add RAGAS CI on 50-question golden set (N-10)
4. Intent-based cheap routing before Mix (N-04)

**Do not:** Add more query modes. Add **smarter routing** and **measurement**.
