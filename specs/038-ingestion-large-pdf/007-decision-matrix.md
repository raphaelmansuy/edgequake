# SPEC-038 — Decision Matrix

**Purpose:** Evaluate options for large PDF ingestion reliability  
**Method:** Weighted scoring + adversarial attacks + complexity analysis  
**Decision date:** 2026-07-01

---

## Decision 1 — PDF Conversion Backend Selection

| Option | Description | Score /10 | Verdict |
| ------ | ----------- | --------- | ------- |
| **A — Text probe → auto EdgeParse** | Sample text layer; route born-digital to O(P) path | **9** | ✅ **CHOSEN** |
| B — Keep Vision default | Status quo | 2 | ❌ Fails reproducer |
| C — Always EdgeParse | No Vision path | 4 | ❌ Breaks scanned PDFs |
| D — Vision with higher timeout only | Raise caps, no routing | 5 | ❌ Wastes API cost |
| E — Client-side split only | User splits PDF manually | 3 | ❌ Poor UX; lineage loss |

### Why A wins

- Reproducer: 1.44 MB text / 603 pages → EdgeParse ~60–120 s vs Vision ~4500+ s  
- O(n) expert: changes asymptotic class from O(P×T_llm) to O(P)  
- DRY: routing policy is one function, not scattered env hacks

### Attack on A — "Text probe can misclassify"

**Defense:** User override + workspace `pdf_parser_backend` + env `EDGEQUAKE_PDF_PARSER_BACKEND`. Log `routing_audit` with probe metrics. False negative → user picks Vision; false positive → quality heuristic (printable ratio) triggers Vision fallback post-EdgeParse if garbled.

---

## Decision 2 — Timeout Strategy

| Option | Description | Score | Verdict |
| ------ | ----------- | ----- | ------- |
| **A — Per-task timeout from LargeDocumentProfile** | `f(pages, backend, provider)` | **9** | ✅ **CHOSEN** |
| B — Global 24 h timeout | `TASK_PROCESSING_TIMEOUT_SECS=86400` | 4 | ❌ Stuck tasks linger |
| C — No worker timeout | Remove cap | 2 | ❌ Phantom processing |
| D — Vision-only timeout fix (mission/04) | Already partial | 6 | ⚠️ Necessary not sufficient |

### Attack on A — "Formula wrong for some GPUs"

**Defense:** Env overrides remain: `EDGEQUAKE_PDF_SECS_PER_PAGE`, `TASK_PROCESSING_TIMEOUT_SECS`. Profile formula is **default**, not hard cap.

---

## Decision 3 — SSOT for Profile / Estimates

| Option | Description | Score | Verdict |
| ------ | ----------- | ----- | ------- |
| **A — `LargeDocumentProfile` module** | Single struct drives routing, timeout, UX | **10** | ✅ **CHOSEN** |
| B — Duplicate formulas in upload + worker | Copy-paste | 3 | ❌ DRY violation |
| C — Frontend-only estimates | Client guesses | 4 | ❌ Wrong without probe |

---

## Decision 4 — Pre-Upload UX

| Option | Description | Score | Verdict |
| ------ | ----------- | ----- | ------- |
| **A — Admission card for P≥100** | Page count + ETA + parser choice | **8** | ✅ **CHOSEN** |
| B — No UI change (backend only) | Silent fix | 6 | ⚠️ Phase 1 acceptable alone |
| C — Block uploads P>500 | Hard reject | 2 | ❌ Product regression |

### Attack on A — "Extra friction"

**Defense:** Card only for `P≥100` or `B≥10MB`. Small PDFs unchanged.

---

## Decision 5 — Gleaning on Large Docs

| Option | Description | Score | Verdict |
| ------ | ----------- | ----- | ------- |
| **A — Profile disables gleaning when P≥500** | Cuts ~2× extract LLM calls | **8** | ✅ **CHOSEN** |
| B — Always gleaning | Quality max | 5 | ❌ +16 min on reproducer |
| C — User toggle only | No auto | 6 | ⚠️ Combine with A |

---

## Decision 6 — Content Size Limit Alignment

| Option | Description | Score | Verdict |
| ------ | ----------- | ----- | ------- |
| **A — Orchestrator uses `MAX_UPLOAD_BYTES` SSOT** | 50 MB consistent | **9** | ✅ **CHOSEN** |
| B — Keep 10 MB orchestrator | Status quo | 4 | ❌ Future OCR docs fail |
| C — Remove limit entirely | No cap | 2 | ❌ OOM risk |

---

## Decision 7 — Gold Test Fixture

| Option | Description | Score | Verdict |
| ------ | ----------- | ----- | ------- |
| **A — Commit reproducer PDF to test fixtures** | Real test is law | **9** | ✅ **CHOSEN** (git-lfs) |
| B — Synthetic 603-page PDF in CI | Generated | 6 | ⚠️ Doesn't catch arXiv quirks |
| C — No PDF in repo; manual only | Skip | 1 | ❌ Regression guaranteed |

---

## Weighted Summary

| Criterion | Weight | Chosen stack score |
| --------- | ------ | ------------------ |
| Fixes reproducer | 25% | 10 |
| DRY / maintainability | 20% | 9 |
| Asymptotic efficiency | 20% | 10 |
| UX / informed consent | 15% | 8 |
| Testability | 10% | 9 |
| Backward compat (scanned PDFs) | 10% | 9 |
| **Weighted total** | | **9.3** |

---

## Final Decision Record

```
DECISION-038-01: Text-layer probe routes born-digital PDFs to EdgeParse by default.
                  User/workspace/env override preserved.

DECISION-038-02: LargeDocumentProfile SSOT computes task timeout, estimates,
                  and gleaning defaults.

DECISION-038-03: Per-task processing_timeout_override on PDF tasks.

DECISION-038-04: Admission card UX for page_count ≥ 100 OR size ≥ 10 MB.

DECISION-038-05: Gold integration test on guide_2606.24937v1-opt.pdf (603 pages).

DECISION-038-06: Align orchestrator MAX_DOCUMENT_SIZE with MAX_UPLOAD_BYTES SSOT.
```

---

## Rejected Alternatives Log

| Alternative | Why rejected |
| ----------- | ------------ |
| Skip entity extraction for P>500 | Destroys RAG value |
| Sync upload for large PDFs | HTTP timeout; already async task queue |
| PDF → single chunk | Breaks retrieval granularity |
| Cloud-only Vision with bigger quota | Cost + still O(P×T_llm) |
