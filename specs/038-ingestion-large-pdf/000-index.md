# SPEC-038 — Large PDF Ingestion: Reliable Processing for 500+ Page Documents

**Spec:** `038-ingestion-large-pdf`  
**Date:** 2026-07-01  
**Method:** Code is law — all claims cross-referenced against live source files and reproducible measurements.  
**Status:** `IMPLEMENTED` (2026-07-01) — See [008-implementation-plan.md](./008-implementation-plan.md)  
**Trigger:** Failed ingestion of `guide_2606.24937v1-opt.pdf` (603 pages, 11 MB, born-digital arXiv PDF)

---

## TL;DR — Executive Decision

> **A 603-page born-digital PDF fails because the default Vision backend turns an O(pages) CPU parse into O(pages × LLM_calls), blowing past the fixed 7200 s worker timeout. Fix: text-layer probe → auto EdgeParse for text-native PDFs, adaptive task timeout `f(pages, backend, provider)`, honest pre-flight ETA in UI, and gold integration tests on the reproducer file. DRY: one `LargeDocumentProfile` SSOT drives timeout, concurrency, chunking, and UX warnings.**

---

## Reproducer Profile (Real Test Is Law)

| Property | Measured value | Tool |
| -------- | -------------- | ---- |
| File | `/Users/raphaelmansuy/Downloads/guide_2606.24937v1-opt.pdf` | `ls -lh` |
| Pages | **603** | `pdfinfo` |
| File size | **11 043 120 bytes** (~10.5 MiB) | `pdfinfo` |
| Title | *The Hitchhiker's Guide to Agentic AI: From Foundations to Systems* | `pdfinfo` |
| Born-digital text | **1 443 139 bytes** extractable (~2 390 chars/page) | `pdftotext` |
| PDF version | 1.3, A4, unencrypted | `pdfinfo` |

**Implication:** This document is **not** a scan. EdgeParse should complete in **seconds–minutes**, not **hours**. Default `PdfParserBackend::Vision` is the wrong asymptotic class.

---

## The Evidence (Code Is Law)

| Symptom | Root cause (file) | Line(s) |
| ------- | ----------------- | ------- |
| Vision is default backend | `PdfParserBackend::Vision` is `#[default]` | `edgequake-pdf/src/backend/mod.rs:17–20` |
| 603-page vision timeout ≈ 82 min (cloud) | `120 + pages × 8` via `vision_outer_timeout_secs` | `safety_limits.rs:641–644` |
| Worker kills task at 7200 s | `processing_timeout_secs: 7200` | `edgequake-tasks/src/worker.rs:182` |
| Worker comment admits 3.3 h need | "1000+ pages … ≈ 3.3h" vs 2 h cap | `worker.rs:154–156` |
| Local 603-page: concurrency=1, dpi=120 | `page_count >= 200` branch | `pdf_processing.rs:93–94, 121–122` |
| EdgeParse is O(pages) CPU, no LLM | `spawn_blocking` + `convert_bytes` | `edgequake-pdf/src/backend/edgeparse.rs:51–61` |
| Post-conversion: ~603 chunks (1/page) | `ChunkStrategy::Pdf` + page markers | `ingestion_pipeline.rs:41–50`, `edgeparse.rs:15–19` |
| Extraction: 16 concurrent × 180 s/chunk | `max_concurrent_extractions` default 16 | `pipeline/config.rs:161` |
| Orchestrator hard-rejects >10 MB text | `MAX_DOCUMENT_SIZE_BYTES = 10MB` | `orchestrator/ingestion.rs:138` |
| Upload allows 50 MB | `MAX_UPLOAD_BYTES = 50MB` | `resource/budget.rs:21` |
| Circuit breaker after 3 timeouts | `check_circuit_breaker` | `edgequake-tasks/src/types/task.rs:206` |
| Vision admission: max 2 jobs | `PdfVisionSemaphore::new(2)` | `e2e_document_deletion_postgres.rs:219` (pattern) |
| **Upload UI fake 40% progress** | `progress: 40` on `fetch()` start | `use-file-upload.ts:170–171` |
| **"Step 2: Uploading…" copy** | i18n key `documents.upload.uploading` | `locales/en.json:112` |
| **Sync admit: BYTEA INSERT before 200** | `create_pdf` in upload handler | `upload.rs:605–616`, `pdf_storage_impl.rs:50–76` |
| **`file_data.clone()` on admit** | duplicate 11 MB in RAM | `upload.rs:613` |
| **No fetch timeout on upload** | bare `fetch()` in `apiClient` | `client.ts:333` |

---

## Pipeline Cost Model (603 Pages)

```text
                    ┌─────────────────────────────────────────────────────────┐
  Upload 11 MB      │  PHASE 0: HTTP ADMIT (sync — blocks UI "Step 2")       │
  ───────────────►  │  multipart + SHA-256 + BYTEA INSERT + enqueue          │
                    │  T ≈ O(bytes) network + O(bytes) DB  (not O(pages))    │
                    │  UI shows fixed 40% until 200 OK ← OBSERVED BLOCKER    │
                    └────────────────────────────┬────────────────────────────┘
                                                 │
                    ┌────────────────────────────▼────────────────────────────┐
                    │  PHASE A: PDF → Markdown                                  │
                    │  Vision (DEFAULT)          EdgeParse (CORRECT for repro)   │
                    │  T ≈ 120+603×8s = 4944s    T ≈ 30–120s (pdfium/edgeparse)│
                    │  ≈ 82 min (cloud est.)     ≈ 1–2 min                      │
                    └────────────────────────────┬────────────────────────────┘
                                                 │
                    ┌────────────────────────────▼────────────────────────────┐
                    │  PHASE B: Chunk → Extract → Embed → Merge               │
                    │  Chunks ≈ 603 (Pdf strategy, 1.44 MB text)                │
                    │  T_extract ≈ ⌈603/16⌉ × 25s ≈ 16 min (optimistic)       │
                    │  T_merge  = O(entities × DB_round_trips) — unbounded      │
                    └────────────────────────────┬────────────────────────────┘
                                                 │
                    ┌────────────────────────────▼────────────────────────────┐
                    │  WORKER TIMEOUT: 7200 s (2 h) — FIXED                   │
                    │  Vision path total: 82 min + 16 min + merge > 2 h       │
                    │  EdgeParse path total: ~2 min + 16 min + merge ≈ OK       │
                    └─────────────────────────────────────────────────────────┘
```

---

## Documents in this Spec

| File | Lens | Key Question |
| ---- | ---- | ------------ |
| [001-five-whys.md](./001-five-whys.md) | Root Cause | Why did the 603-page guide fail? |
| [002-first-principles.md](./002-first-principles.md) | First Principles | What are we really solving? |
| [003-product-owner-lens.md](./003-product-owner-lens.md) | Product Owner | What is the user/business value? |
| [004-ux-ui-designer-lens.md](./004-ux-ui-designer-lens.md) | UX/UI Designer | How should large-PDF UX behave? |
| [005-fullstack-developer-lens.md](./005-fullstack-developer-lens.md) | Full Stack Dev | How to implement (DRY/SOLID)? |
| [006-on-expert-lens.md](./006-on-expert-lens.md) | O(n) Expert | Complexity, bounds, bottlenecks |
| [007-decision-matrix.md](./007-decision-matrix.md) | Decision | Routing, timeout, UX options |
| [008-implementation-plan.md](./008-implementation-plan.md) | Implementation | Phased plan, tests, DoD |
| [009-cross-reference-matrix.md](./009-cross-reference-matrix.md) | Cross-Reference | Every claim → evidence |

---

## Requirements (REQ-038-xx)

| ID | Requirement |
| -- | ----------- |
| REQ-038-01 | Born-digital PDFs with extractable text layer auto-route to EdgeParse unless user forces Vision |
| REQ-038-02 | `LargeDocumentProfile` SSOT: pages, bytes, text_ratio, backend, provider → timeout/concurrency/ETA |
| REQ-038-03 | Task `processing_timeout_secs` scales with profile (not fixed 7200 s for 500+ pages) |
| REQ-038-04 | Pre-flight UI shows page count, estimated duration, recommended backend |
| REQ-038-05 | Progress UI shows phase + page/chunk counters for 500+ page docs |
| REQ-038-06 | Resume from checkpoint survives worker timeout (markdown persisted → skip Phase A) |
| REQ-038-07 | Gold test: `guide_2606.24937v1-opt.pdf` ingests via EdgeParse in CI (mock LLM) |
| REQ-038-08 | Align `MAX_DOCUMENT_SIZE_BYTES` orchestrator limit with upload SSOT (50 MB) or document explicitly |
| REQ-038-09 | Failure messages distinguish timeout vs size vs embedding vs circuit-breaker |
| REQ-038-10 | No regression: scanned PDFs still route to Vision when text probe fails |
| REQ-038-11 | Upload UX: real byte progress or fast-ack admit; no fake 40%; size-scaled client timeout |
| REQ-038-12 | Large PDF + resolved Vision parser → choice popup before upload; EdgeParse at any level proceeds silently |

---

## Parser Resolution Priority (Code Is Law)

```
Upload override (multipart `pdf_parser_backend`)
    ↓ if unset
Workspace default (`workspace.pdf_parser_backend`)
    ↓ if unset
Server env (`EDGEQUAKE_PDF_PARSER_BACKEND`)
    ↓ if unset
Fallback: Vision (`PdfParserBackend::default()`)
```

**Frontend SSOT:** `edgequake_webui/src/lib/pdf/resolve-pdf-parser-backend.ts`  
**Backend SSOT:** `edgequake-api/src/handlers/pdf_upload/types.rs` → `resolved_backend()`

**Admission gate (REQ-038-12):** Choice popup appears only when `page_count ≥ threshold` **and** resolved backend is `vision`. User may confirm EdgeParse or proceed with Vision (with slowdown warning).

## Decision Summary

```
CHOSEN: Adaptive routing (text probe → EdgeParse) + LargeDocumentProfile SSOT
         + scaled worker timeout + honest ETA UX + gold reproducer test.

REJECTED: Always Vision (current default for born-digital — fails at scale)
REJECTED: Raise timeout to 24 h globally (masks stuck tasks)
REJECTED: Client-side PDF splitting only (shifts burden, loses page lineage)
REJECTED: Skip entity extraction for large docs (breaks RAG value prop)
```

---

## Related Specs

| Spec | Relationship |
| ---- | ------------ |
| [mission/04-heavy-pdf.md](../../mission/04-heavy-pdf.md) | ADR-04-001..003 vision timeout/concurrency (partial fix) |
| [SPEC-011 pipeline reliability](../011-pipeline-reliabilty/docs/EDGE_CASES.md) | EC-001 embedding input count |
| [SPEC-016 ingestion audit](../016-datalayer-audit/005-ingestion/001-pipeline-flow.md) | Graph merge O(n) amplification |
| [SPEC-032 graph](../032-graph/000-index.md) | Pdf chunk strategy, page markers |
| [SPEC-006 resource budget](../../edgequake/crates/edgequake-core/src/resource/budget.rs) | Upload limits SSOT |
