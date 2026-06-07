# E2E Proof — Full Pipeline (chunk → extract → merge → graph)

**Date:** 2026-06-04 (re-verified 10:15 UTC)  
**Result:** ✅ Rust integration + live API (sync + async + PDF) + Playwright UI (6/6)

## Rust — deterministic full path

```bash
cargo test -p edgequake-pipeline --test spec017_full_pipeline_integration
# 2/2 passed
```

| Test | Proves |
|------|--------|
| `spec017_full_pipeline_chunk_extract_merge_graph` | Chunk → mock LLM + `JsonExtractionParser` → `KnowledgeGraphMerger` → graph nodes `EDGEQUAKE`, `SARAH_CHEN`, edge `SARAH_CHEN→EDGEQUAKE` |
| `spec017_full_pipeline_resilience_path` | `process_with_resilience` retains partial extractions when one chunk returns invalid JSON |

Uses `MockProvider` with pre-seeded extraction JSON — **canonical proof** of refactored pipeline internals (DRY-001/003/005, SOLID-L-002).

## Live API — sync ingestion (Mistral workspace)

```bash
POST /api/v1/documents  { content, async_processing: false }
```

Playwright: `spec017-pipeline-full-ingestion.spec.ts` test 1

- **chunk_count > 0** ✅
- **entity_count > 0** ✅ (varies with LLM run)
- **status:** `processed` / **Completed** in UI ✅

## Live API — async background task pipeline

```bash
POST /api/v1/documents  { content, async_processing: true }
GET  /api/v1/documents/{id}  # poll until processed/completed
```

Playwright test 5 — polls up to 5 min, asserts entities + chunks, UI **Completed** badge.

## Live API — PDF path (pdf_conversion → text_insert)

```bash
POST /api/v1/documents/pdf  multipart (001_simple_text.pdf, enable_vision=false, pdf_parser_backend=text)
GET  /api/v1/documents/pdf/{pdf_id}  # poll until completed + document_id
GET  /api/v1/documents/{document_id}  # chunk_count > 0
```

Playwright test 6 — **passed in 3.1s** on 2026-06-04 re-run.

## Live API — mock workspace (chunk-only on live stack)

Mock workspace without pre-queued JSON responses:

- **chunk_count > 0** ✅
- **entity_count** may be 0 → UI shows **Partial Failure**
- Honest limit: live `MockProvider` in API server does not mirror test `add_response()` queue

## Playwright UI — async upload stages

Test 4 captures upload progress banner: Reading → Uploading → Extracting → Done.

## Screenshot analysis (all artifacts)

### `01-documents-page-header.png`

- EdgeQuake shell, **Documents** nav active, empty state **Documents (0)**.
- Upload dropzone + **Workspace Default** parser dropdown.
- **Verdict:** UI entry point healthy before ingestion.

### `02-documents-main-panel.png`

- Search, filters, drag-and-drop zone, empty-state CTA.
- **Verdict:** Document manager controls render; `/documents` API wired.

### `03-sync-upload-completed.png`

- **Completed** badge; entity count + cost visible; `spec017-pipeline-*.md`.
- **Verdict:** Sync Mistral pipeline end-to-end.

### `04-documents-after-full-pipeline.png`

- Main panel: Completed row, entity count, NEW badge.
- **Verdict:** API + UI list confirmation after sync upload.

### `05-ui-upload-chunking-stage.png` / `06-ui-upload-processing-panel.png`

- Multi-step upload progress (Reading/Uploading/Extracting/Done).
- **Verdict:** Async UI path reaches upload stage.

### `07-async-pipeline-completed.png`

- **Completed** badge for `spec017-async-*.md`; **13 entities**; cost **$0.00045**.
- **Verdict:** Background task pipeline (async_processing=true) finishes with extraction.

### Legacy / duplicate

- `03-sync-upload-api-result.png`, `06-ui-upload-processing.png` — superseded captures from earlier runs.

## Acceptance

| Layer | Full pipeline proven? |
|-------|----------------------|
| Rust integration (mock) | ✅ chunk + extract + merge + graph |
| Live API sync (Mistral) | ✅ chunks + entities |
| Live API async (Mistral) | ✅ poll to Completed + entities |
| Live API PDF (text parser) | ✅ pdf_conversion → text_insert → chunks |
| Live API mock | 🟡 Chunk only; extraction partial without seeded mock |
| Playwright async UI banner | 🟡 Stage UI only (terminal status via API poll in test 5) |
