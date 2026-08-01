# 041 — Topic-admit chunk id/content fidelity (First Principles)

**Status:** Closed — audit complete · law **CE_GAP**  
**Cross-ref:** [040](./040-topic-trunc-protect-exploratory.md) · [039](./039-topic-ce-protect-exploratory.md) · [038](./038-topic-entity-admit-exploratory.md) · [037](./037-summarize-chunk-link-audit.md)  
**Warm WS:** `2a7bcb2f-b156-4c49-9229-67f5bcde22a4`  
**Binding Q:** `Medical-0002d2de`  
**Artifact:** [`ingest-audit/topic-fidelity-20260720T112509Z`](../e2e/artifacts/ingest-audit/topic-fidelity-20260720T112509Z/)  
**Tool:** [`tools/bench001/scripts/audit_topic_chunk_fidelity.py`](../../../tools/bench001/scripts/audit_topic_chunk_fidelity.py)

---

## 1. First principles

```text
entity (exact-name) → source_chunk_ids → storage body → question bigrams?
                              ↓
                         Mix C (post CE/trunc)
```

| Law | Necessary condition |
|-----|---------------------|
| **RESOLVE** | Each `source_chunk_id` fetches a non-empty body |
| **CONTENT** | ≥1 body contains a question content bigram |
| **IN_MIX** | ≥1 such bigram appears in admitted Mix C |
| **CE_GAP** | CONTENT ∧ ¬IN_MIX — survivors lost between link and C |

038–040 assumed CONTENT and tried protect knobs. This audit measures CONTENT/RESOLVE before more protect.

Industry parallel: graph provenance must verify linked text, not only node degree ([GraphRAG drift / imperfect KG](https://arxiv.org/pdf/2603.14828)); packing metrics assume answer-bearing docs exist in the shortlist ([answer-in-context](https://arxiv.org/html/2607.00725v1)).

---

## 2. Observables (`T112509Z`)

| Check | Result |
|-------|--------|
| Entity | `BONE_CANCER` EQ 5 chunks · LR 6 |
| RESOLVE | **5/5** (KV key = chunk id; vectors hold `content_ref` only) |
| CONTENT | **3/5** hit `bone cancers` (also TNM on 149–150) |
| EQ Mix C | **0** question bigrams (41k chars / 6 parts — cervical/anal/AML) |
| LR Mix C | **hits `bone cancers`** |

**Verdict law: `CE_GAP`.**

Not RESOLVE (ids are real). Not CONTENT (bodies are on-topic). Protect ladder failed because topic CONTENT ids never occupy post-CE Mix (040: no trunc-prefer log on this Q).

---

## 3. Implications

1. **Stop** stacking `TOPIC_*_PROTECT` Acc packages without a CE_GAP confound that **materializes** CONTENT chunk bodies into Mix (direct KV fetch by id), not only reorders an empty survivor set.  
2. Keep **a1fp** Acc peer.  
3. Forbidden still: densify-all, dual-list as Acc headline, FAQ induce.

---

## 4. Next (one confound)

**042:** Done — [materialize](./042-topic-chunk-materialize.md) Sum ER↑ / Acc REJECT. **043:** CONTENT-gated materialize (bigram filter on KV body).

---

## 5. Reproduce

```bash
export BENCH001_EQ_WORKSPACE_ID=2a7bcb2f-b156-4c49-9229-67f5bcde22a4
python3 tools/bench001/scripts/audit_topic_chunk_fidelity.py \
  --predictions-eq specs/001-benchmark/e2e/artifacts/history/smoke-20260720T111944Z/predictions_eq.json \
  --predictions-lr specs/001-benchmark/e2e/artifacts/history/smoke-20260720T111944Z/predictions_lr.json
  # SPEC-097: predictions_*.json are local-only (gitignored); regenerate via make bench001-*
```
