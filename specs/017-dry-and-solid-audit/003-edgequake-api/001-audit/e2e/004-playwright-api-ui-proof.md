# Playwright — API-backed UI routes + full pipeline

**Status:** ✅ 6/6 passed (21.3s)  
**Date:** 2026-06-04 12:22 UTC (re-verified)

```bash
./specs/017-dry-and-solid-audit/003-edgequake-api/001-audit/e2e/run_playwright_proof.sh
# spec017-api-query-documents.spec.ts
```

## Screenshot analysis

### `01-query-page-header.png`

- EdgeQuake shell; **Query** nav active; Hybrid mode selected.
- Bootstrapped workspace selector (`spec017-api-que…`).
- Query textarea + mode chips (Local/Global/Hybrid/Simple).
- **Verdict:** `/query` route wired; SOTA query UI healthy before any ingestion.

### `02-query-main-panel.png`

- Suggestion cards, textarea, Send button; empty History sidebar.
- **Verdict:** Query handler + frontend integration OK; shared `execute_sota_query_*` path reachable from UI.

### `03-documents-page-header.png`

- **Documents** nav active; upload dropzone + parser dropdown; empty state.
- **Verdict:** Document manager entry point healthy; API list endpoint wired.

### `04-documents-main-panel.png`

- Search, filters, drag-and-drop zone, empty table.
- **Verdict:** Documents page controls render; pre-upload baseline capture.

### `05-sync-upload-completed.png`

- **Completed** badge; **8 entities**; cost **$0.00031**; `spec017-api-sync-*.md`.
- **Verdict:** Sync path (`async_processing: false`) → `WorkspacePipelineFactory` → chunks + extraction + UI list.

### `06-async-pipeline-completed.png`

- **Completed** badge for `spec017-api-async-*.md`; **8 entities**; cost **$0.00031**; NEW badge.
- **Verdict:** Background task pipeline (`async_processing: true`) → `AppState::enqueue_task` → processor strict path → terminal Completed.

## Health JSON (`002-health-response.json`)

- `status: healthy`, `storage_mode: postgresql`, all components true, Mistral LLM + embed, `pdf_storage_enabled: true`.
- **Verdict:** Production bootstrap (`query_bootstrap.rs`) + postgres adapters healthy.
