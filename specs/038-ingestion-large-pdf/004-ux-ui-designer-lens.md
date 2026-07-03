# SPEC-038 — UX/UI Designer Lens

**Lens:** UX/UI Designer  
**Persona:** User uploading a 603-page PDF via Documents page  
**Evidence:** Existing progress patterns in `pipeline_progress_callback.rs`, document status UI

---

## Design Problem

Large PDF ingestion is a **long-running background job** (minutes to hours).  
Current UX treats all documents as homogeneous "Processing" with insufficient:

- **Pre-commit informed consent** (time cost)  
- **Phase differentiation** (converting vs extracting vs indexing)  
- **Scale-aware progress** (page 47/603 vs spinner)  
- **Failure taxonomy** (timeout vs size vs provider)

---

## Information Architecture

```text
┌─────────────────────────────────────────────────────────────────────┐
│  DOCUMENTS LIST                                                      │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │ 📄 guide_2606.24937v1-opt.pdf          603 pages · 11 MB       │  │
│  │ ████████████░░░░░░░░  Converting  412/603 pages  ·  34m left │  │
│  │    Phase: PDF → Markdown (EdgeParse)                          │  │
│  └───────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘

States:
  reading → uploading (HTTP admit) → admitted → converting → extracting → embedding → indexed
                    ↘ failed (typed)     ↘ cancelled

**Observed failure (2026-07-01):** User stuck at `uploading` / "Step 2" — conversion never starts.
See [001-five-whys.md](./001-five-whys.md) Symptom D and [002-first-principles.md](./002-first-principles.md) P9–P10.
```

---

## Pre-Upload Dialog (New: Large PDF Admission Card)

Shown when `page_count ≥ 100` **and** resolved parser is **Vision** (REQ-038-12):

Parser resolution at upload time:

```text
Upload selector  →  Workspace default  →  Server env  →  Vision fallback
     (explicit)         (explicit)          (implicit)       (default)
```

If resolved parser is **EdgeParse** (at upload or workspace level), upload proceeds **silently** — no popup.

```text
┌─ Upload Preview ─────────────────────────────────────────────┐
│  guide_2606.24937v1-opt.pdf                                   │
│  603 pages · 10.5 MB                                          │
│                                                               │
│  ┌─ Detected ─────────────────────────────────────────────┐  │
│  │  ✓ Text layer found (~2,400 chars/page)               │  │
│  │  Recommended: Fast parse (EdgeParse) — ~25 min total    │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                               │
│  Parser:  (●) Fast parse (recommended)  ( ) Vision OCR       │
│                                                               │
│  ⚠ Vision OCR on this file may take 2+ hours and can fail.    │
│                                                               │
│            [ Cancel ]              [ Upload & Process ]         │
└───────────────────────────────────────────────────────────────┘
```

**Rules:**

| Condition | Default selection | User override | Popup shown? |
| --------- | ----------------- | ------------- | ------------ |
| Text layer detected + Vision resolved | EdgeParse | Vision with warning | Yes |
| EdgeParse at upload or workspace | EdgeParse | — | **No** (silent) |
| No text / image-only | Vision | EdgeParse disabled with tooltip | Yes (if large) |
| `page_count < 100` | Current behavior (no card) | — | No |

---

## Upload Progress Panel (Current Gap — Symptom D)

**What the user sees today** (`UploadProgressList` + `use-file-upload`):

```text
┌─ Processing Files ─────────────────────────── 0/1 files complete ─┐
│  Reading → Uploading → Extracting → Done                         │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │ guide_2606.24937v1-opt.pdf · 10784.3 KB                    │  │
│  │ Step 2: Uploading to server...                              │  │
│  │ ████████░░░░░░░░░░  (~40% — STATIC, not bytes sent)        │  │
│  └────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
```

| UX sin | Code truth | User impact |
| ------ | ---------- | ----------- |
| Label says "Uploading" | Entire sync **admit** pipeline (BYTEA write, enqueue) | Blames network when DB is slow |
| 40% progress bar | Hard-coded when `fetch()` starts | Implies progress that does not exist |
| No timeout message | `apiClient` has no `AbortSignal` | Infinite spinner |
| "Extracting" step in legend | Only reached **after** HTTP 200 | Misleading step order during hang |

**Target UX (REQ-038-11):**

```text
Phase A — Transfer:  "Sending file… 8.2 / 10.5 MB"  (XHR/fetch upload progress)
Phase B — Admit:     "Saving to workspace…"         (after bytes sent; poll track_id)
Phase C — Convert+:  existing pipeline stages
```

Or: **202 Accepted** immediately after headers validated → skip Phase B spinner entirely.

**`data-testid` hooks (implemented):** `spec038-upload-progress-list` — extend with `spec038-upload-bytes-sent` when byte progress lands.

---

## In-Progress Document Row

Extend existing `current_stage` / `stage_message` metadata (already written in `pdf_processing.rs:246–264`):

| Phase | `current_stage` | `stage_message` template | Progress bar source |
| ----- | --------------- | ------------------------ | ------------------- |
| Convert | `converting` | `Converting page {n}/{total}` | WebSocket `PipelinePhase::PdfConversion` |
| Extract | `extracting` | `Extracting entities {n}/{chunks}` | `PipelinePhase::Extraction` |
| Embed | `embedding` | `Embedding {n}/{chunks}` | `PipelinePhase::Embedding` |
| Index | `indexing` | `Saving to knowledge graph` | `PipelinePhase::Persist` |

**Large-doc layout (≥200 pages):**

- Show **numeric counters** always (not just spinner)  
- Show **elapsed + ETA** (computed client-side from `stage_progress` velocity)  
- **Cancel** button remains visible (uses `track_id` — already in metadata)

```text
┌─ Side-by-side viewer header (processing) ────────────────────┐
│  Processing · 41 min elapsed · ~18 min remaining              │
│  █████████████████░░░░░░░  68%                                │
│  Extracting entities — chunk 412 of 603                       │
│  [ Cancel processing ]                                        │
└───────────────────────────────────────────────────────────────┘
```

---

## Failure States (Typed)

Replace generic "Failed" with **failure_class** surfaced in UI:

| `failure_class` | User message | Primary CTA |
| --------------- | ------------ | ------------- |
| `timeout_phase_convert` | Conversion timed out. Try Fast parse or fewer pages. | Reprocess with EdgeParse |
| `timeout_phase_extract` | Entity extraction timed out. Try faster model or disable gleaning. | Open settings |
| `circuit_breaker` | Too many timeouts. Check LLM provider. | Reprocess (fresh) |
| `document_too_large` | Text exceeds 10 MB limit. Split the PDF. | Download + help link |
| `embedding_limit` | Too many entities for embedding batch. | Retry (after fix) / support |
| `provider_unavailable` | LLM provider not responding. | Check provider status |

**Visual:** Red badge with icon per class; expand for technical `error_code` (collapsed by default).

---

## Accessibility

| Requirement | Implementation |
| ----------- | -------------- |
| Progress announced | `aria-live="polite"` on stage_message updates |
| ETA not sole indicator | Always show absolute counts (412/603) |
| Cancel confirm | Dialog: "Cancel processing? Progress may be lost after page X." |
| Color + text | Never rely on color alone for Failed vs Processing |

---

## Mobile / Narrow Viewport

- Pre-upload card stacks vertically; parser radio remains tappable (44px min)  
- Progress row truncates filename; page count remains visible  
- ETA moves below progress bar on `<640px`

---

## Component Targets (No New Design System)

| Change | File | Pattern source |
| ------ | ---- | -------------- |
| Upload preview card | `documents/upload` flow | Existing upload modal |
| Phase progress | Document list row | `pipeline_progress_callback` events |
| Typed failure banner | Document detail | Existing status badges |
| Parser override | Upload form / workspace settings | `pdf_parser_backend` field exists |

---

## UX Acceptance Criteria

| ID | Criterion |
| -- | --------- |
| UX-038-01 | User sees page count within 2 s of file select (client-side `pdf.js` or server HEAD) |
| UX-038-02 | ≥200 pages triggers admission card before upload starts |
| UX-038-03 | Progress shows `N/total` for converting and extracting phases |
| UX-038-04 | Failed state shows human message + one primary action |
| UX-038-05 | Forced Vision on born-digital shows explicit slowdown warning |
| UX-038-06 | EdgeParse at upload or workspace level skips admission popup (silent upload) |

---

## Anti-Patterns (Do Not)

| Anti-pattern | Why |
| ------------ | --- |
| Infinite spinner with no counts | User assumes crash; refreshes; duplicates upload |
| "Processing" for 2 hours with no ETA | Support burden |
| Silent fallback to EdgeParse | User can't audit which path ran |
| Blocking modal when EdgeParse already selected | Unnecessary friction |
| Blocking modal for <100 page PDFs | Friction without value |
