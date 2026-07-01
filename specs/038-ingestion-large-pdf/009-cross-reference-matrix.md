# SPEC-038 — Cross-Reference Matrix

**Purpose:** Maps every claim across all lenses to live source evidence and test artifacts.  
**Method:** Code is law — file + line verification (2026-07-01)

---

## Reproducer Evidence

| Claim | Source | Value | Verified |
| ----- | ------ | ----- | -------- |
| 603 pages | `pdfinfo` on reproducer | 603 | ✅ |
| 11 043 120 bytes | `pdfinfo` | 10.5 MiB | ✅ |
| Born-digital text 1 443 139 bytes | `pdftotext` | ~2389 chars/page | ✅ |
| arXiv GenPDF creator | `pdfinfo` Producer | tex2pdf | ✅ |

---

## Symptom Evidence (Code)

| Claim | File | Line(s) | Verified |
| ----- | ---- | ------- | -------- |
| Vision is default backend | `edgequake-pdf/src/backend/mod.rs` | 17–20 | ✅ |
| `enable_vision` default true | `handlers/pdf_upload/types.rs` | 14–15 | ✅ |
| Backend resolution chain | `handlers/pdf_upload/types.rs` | 67–72 | ✅ |
| Worker timeout 7200 s | `edgequake-tasks/src/worker.rs` | 182 | ✅ |
| Worker comment 3.3 h vs 2 h cap | `edgequake-tasks/src/worker.rs` | 154–156, 182 | ✅ |
| Vision outer timeout formula | `safety_limits.rs` | 641–644 | ✅ |
| `secs_per_page` cloud = 8 | `safety_limits.rs` | 632–634 | ✅ |
| 603-page concurrency local = 1 | `processor/pdf_processing.rs` | 93–94 | ✅ |
| 603-page dpi = 120 | `processor/pdf_processing.rs` | 121–122 | ✅ |
| Vision timeout + fallback EdgeParse | `processor/pdf_processing.rs` | 730–765 | ✅ |
| Resume requires stored markdown | `processor/pdf_processing.rs` | 368–375 | ✅ |
| EdgeParse O(P) cpu path | `edgequake-pdf/src/backend/edgeparse.rs` | 51–61 | ✅ |
| Page markers for Pdf chunking | `edgequake-pdf/src/backend/edgeparse.rs` | 15–19 | ✅ |
| Pdf chunk strategy auto | `ingestion_pipeline.rs` | 41–50 | ✅ |
| Adaptive chunk 600 tokens >100KB | `adaptive_chunking.rs` | 12–14 | ✅ |
| max_concurrent_extractions = 16 | `pipeline/config.rs` | 161 | ✅ |
| chunk timeout 180 s | `pipeline/config.rs` | 58, 163 | ✅ |
| Orchestrator 10 MB limit | `orchestrator/ingestion.rs` | 138 | ✅ |
| Upload 50 MB limit | `resource/budget.rs` | 21 | ✅ |
| Circuit breaker on 3 timeouts | `edgequake-tasks/src/types/task.rs` | 206+ | ✅ |
| `extract_page_count` at upload | `handlers/pdf_upload/upload.rs` | 590 | ✅ |
| Upload UI progress hard-coded 40% | `use-file-upload.ts` | — | ✅ **Fixed** — `transferProgressPercent` via XHR |
| Upload phase i18n "Step 2" | `locales/en.json` | `documents.upload.sending` / `saving` | ✅ |
| Sync BYTEA insert on admit | `pdf_storage_impl.rs` | 50–76 | ✅ (unchanged; single copy via move) |
| `pdf_data.clone()` on admit | `handlers/pdf_upload/upload.rs` | 606–615 | ✅ **Removed** — `file_data` moved into `create_pdf` |
| No client upload timeout | `multipart-upload-client.ts` | `uploadTimeoutMs` | ✅ **Fixed** — scaled XHR timeout |
| Early metadata `converting` stage | `processor/pdf_processing.rs` | 246–264 | ✅ |
| Mission ADR heavy PDF | `mission/04-heavy-pdf.md` | RC-1–RC-4 | ✅ |
| SPEC-011 EC-001 embed inputs | `specs/011-pipeline-reliabilty/docs/EDGE_CASES.md` | EC-001 | ✅ |
| SPEC-016 merge O(n) DB | `specs/016-datalayer-audit/005-ingestion/001-pipeline-flow.md` | Stage 2 | ✅ |

---

## Requirement Traceability

| Requirement | Source Lens | Implementation Target |
| ----------- | ----------- | --------------------- |
| REQ-038-01 | `002-first-principles.md` P2 | `pdf_routing_policy.rs`, `upload.rs` |
| REQ-038-02 | `005-fullstack-developer-lens.md` | `large_document_profile.rs` |
| REQ-038-03 | `006-on-expert-lens.md` | `task.rs`, `worker.rs` |
| REQ-038-04 | `004-ux-ui-designer-lens.md` | Upload admission card |
| REQ-038-05 | `004-ux-ui-designer-lens.md` | Document row + WS progress |
| REQ-038-06 | `001-five-whys.md` Symptom B | `pdf_processing.rs` resume |
| REQ-038-07 | `008-implementation-plan.md` Phase 7 | `spec038_large_pdf.rs` |
| REQ-038-08 | `007-decision-matrix.md` D6 | `ingestion.rs` |
| REQ-038-09 | `003-product-owner-lens.md` AC10–12 | `pipeline_progress_callback.rs` |
| REQ-038-10 | `007-decision-matrix.md` Attack on D1 | `pdf_routing_policy.rs` overrides |
| REQ-038-11 | `002-first-principles.md` P9–P10 | `multipart-upload-client.ts`, `use-file-upload.ts`, `upload.rs` |

---

## Decision Cross-Reference

| Decision | Justified In | Adversarially Tested In |
| -------- | ------------ | ----------------------- |
| Text probe → EdgeParse | `006-on-expert-lens.md` | `007-decision-matrix.md` Attack on A |
| LargeDocumentProfile SSOT | `002-first-principles.md` P8 | `005-fullstack-developer-lens.md` |
| Scaled worker timeout | `001-five-whys.md` Symptom B | `006-on-expert-lens.md` time budget |
| Admission card P≥100 | `004-ux-ui-designer-lens.md` | `007-decision-matrix.md` Attack on A |
| Gold reproducer fixture | `008-implementation-plan.md` Phase 7 | `006-on-expert-lens.md` benchmarks |

---

## Edge Case Cross-Reference

| Edge Case | Specified In | Mitigation In |
| --------- | ------------ | ------------- |
| Born-digital misrouted to Vision | `001-five-whys.md` WHY 2 | REQ-038-01 probe |
| Worker timeout mid-extract | `006-on-expert-lens.md` | REQ-038-03 scaled timeout |
| Vision timeout before fallback | `001-five-whys.md` WHY 4 | Route before attempt |
| Resume without markdown | `001-five-whys.md` WHY 4 | Incremental markdown flush (Phase 4+) |
| 10 MB orchestrator reject | `001-five-whys.md` Symptom C | REQ-038-08 |
| Embedding 512 input limit | `006-on-expert-lens.md` EMBED | SPEC-011 EC-001 |
| Graph merge slow | `006-on-expert-lens.md` MERGE | SPEC-016 (follow-up) |
| Circuit breaker | `001-five-whys.md` Symptom B | REQ-038-09 failure_class |
| OOM multi-vision P-G13 | `006-on-expert-lens.md` Space | `compute_safe_pdf_resource_profile` |
| Encrypted PDF | `008-implementation-plan.md` EC-038-01 | `pdf_text_probe` fail-fast |

---

## Test Cross-Reference

| Test Type | Plan Location | Covers REQ |
| --------- | ------------- | ---------- |
| `probe_detects_born_digital` unit | `008-implementation-plan.md` 1.5 | REQ-038-01 |
| `timeout_scales_with_pages` unit | `008-implementation-plan.md` 1.5 | REQ-038-03 |
| `spec038_large_pdf` integration | `008-implementation-plan.md` 7.1 | REQ-038-07 |
| `spec038-large-pdf-admission` e2e | `008-implementation-plan.md` 6.5 | REQ-038-04 |
| `spec038-upload-progress` e2e | `008-implementation-plan.md` 9.6 | REQ-038-11 |
| `resource_safety_proof` update | `008-implementation-plan.md` 5.3 | REQ-038-08 |
| Reproducer baseline JSON | `008-implementation-plan.md` 0.2 | Evidence lock |

---

## Lens → Document Map

| Lens | Document | Primary Outputs |
| ---- | -------- | --------------- |
| Root Cause | `001-five-whys.md` | RC-038-01..06, causal chains |
| First Principles | `002-first-principles.md` | P1–P8, non-goals |
| Product Owner | `003-product-owner-lens.md` | User stories, KPIs |
| UX/UI | `004-ux-ui-designer-lens.md` | Admission card, progress, failures |
| Full Stack | `005-fullstack-developer-lens.md` | Modules, API, files |
| O(n) Expert | `006-on-expert-lens.md` | Complexity table, time budgets |
| Decision | `007-decision-matrix.md` | DECISION-038-01..06 |
| Implementation | `008-implementation-plan.md` | Phases 0–9, DoD |

---

## Related Specs & Missions

| Reference | Link | Relationship |
| --------- | ---- | ------------ |
| mission/04-heavy-pdf | `../../mission/04-heavy-pdf.md` | Vision timeout ADRs (partial) |
| SPEC-011 | `../011-pipeline-reliabilty/` | Embedding edge cases |
| SPEC-016 | `../016-datalayer-audit/` | Merge bottleneck |
| SPEC-032 | `../032-graph/` | Pdf chunk strategy |
| SPEC-006 | `edgequake-core/src/resource/` | Upload budget SSOT |

---

## ASCII: Evidence Chain

```text
  reproducer.pdf
       │
       ├─ pdfinfo ──────────────► P=603, B=11MB ──────────────┐
       ├─ pdftotext ────────────► T=1.44MB text layer ───────┤
       └─ code audit ───────────► Vision default, T_worker=7200┤
                                                               │
                                                               ▼
                                                    SPEC-038 DECISIONS
                                                               │
                     ┌─────────────────────────────────────────┼──────────────────────────┐
                     ▼                     ▼                   ▼                          ▼
              EdgeParse route      Scaled timeout      Admission UX            spec038_large_pdf.rs
```
