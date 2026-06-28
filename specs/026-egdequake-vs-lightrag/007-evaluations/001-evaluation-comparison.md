# 001 — Evaluation Comparison

**Cross-ref:** [004 Query](../004-query/001-query-comparison.md) · [009 Plan](../009-improvement-plan/001-improvement-plan.md)

**Findings:** C-07, C-08, C-09

---

## 1. Current State (brutal)

```text
  LightRAG repo                         EdgeQuake repo
  ─────────────                         ──────────────

  ✗ No RAGAS harness                   △ RAGAS skeleton (50 cases)
  ✗ No golden Q&A CI                   ✓ spec025_golden_qa.json
  ✗ No retrieval recall metrics        △ context_entity_recall
  ✓ Reproduce scripts (Step_0-3)       ✓ e2e + contract tests
  ✓ Parser golden fixtures             △ PDF fixtures only
  △ Manual eval in examples/           △ e2e_query_engine.rs
```

**C-09:** EdgeQuake is **ahead** on eval infrastructure — but skeleton ≠ production gates.

---

## 2. LightRAG Evaluation Surface

### Reproduce Scripts

`LightRAG/reproduce/`:
- `Step_0.py` — extract contexts
- `Step_1.py` / `Step_1_openai_compatible.py` — insert
- `Step_3.py` — query extraction

**Purpose:** Paper reproduction, not CI regression.

### Test-Driven Quality

LightRAG quality is enforced via:
- **Parser golden files** (`tests/parser/docx/golden/`, markdown golden)
- **Pipeline contract tests** (cancellation, hash, chunk preservation)
- **Storage impl tests** (upsert, migration, retry)
- **LLM cache identity tests**

**Gap:** No automated **answer quality** or **retrieval recall** gates.

---

## 3. EdgeQuake Evaluation Surface

### Golden Q&A Set (SPEC-025 8.1)

```text
  spec025_golden_qa.json (≥50 cases)
       │
       ├── expected_answer_keywords
       ├── expected_context_entities
       └── optional mode hint

  eval/golden_set.rs    → loader + stats
  eval/metrics.rs       → keyword_recall, context_entity_recall
  contract_spec025_ragas_skeleton.rs → CI smoke
```

**What it measures:**
- Keyword presence in answers (proxy for correctness)
- Entity presence in retrieved context (proxy for recall)

**What it does NOT measure:**
- RAGAS faithfulness / answer relevance (LLM-judged)
- Latency SLOs under load
- Cross-mode ranking quality
- Head-to-head vs LightRAG on same corpus

---

## 4. Recommended Eval Framework (both need)

```text
  Tier 1 — CI smoke (minutes)          Tier 2 — nightly (hours)
  ───────────────────────────          ────────────────────────

  ✓ Golden keyword recall              △ RAGAS on 50 cases
  ✓ Context entity recall              ✗ NDCG@k per mode
  ✓ Mode routing sanity                ✗ LightRAG parity diff
  ✓ Ingest+query e2e                   ✗ Cost per query budget

  Tier 3 — release gate (weekly)
  ──────────────────────────────

  ✗ Human eval sample (n=30)
  ✗ A/B Mix vs Hybrid vs LightRAG Mix
  ✗ Parser format matrix (LR only today)
```

---

## 5. Head-to-Head Protocol (not yet run)

To honestly claim "matches LightRAG quality":

```text
  1. Fixed corpus: 10 docs (mix PDF + MD + text)
  2. Same LLM + embedding provider
  3. Ingest both systems
  4. 50 queries from golden set
  5. Measure:
       - context_entity_recall
       - answer_keyword_recall
       - latency p50/p95
       - cost (LLM tokens + embed calls)
  6. Diff graphs: entity count, edge count, chunk count
```

**Status:** Not implemented. SPEC-026 documents the gap.

---

## 6. Evaluation Grades

| Dimension | LightRAG | EdgeQuake |
|-----------|:--------:|:---------:|
| Parser quality gates | **A** | **C** |
| Algorithm contract tests | **B+** | **A-** |
| Answer quality gates | **F** | **D+** |
| Retrieval metrics | **F** | **C+** |
| Reproducibility docs | **B** | **B** |
| CI coverage breadth | **A** | **B+** |

**Neither system can claim SOTA-quality validation.** EdgeQuake has the **foundation** to get there first.
