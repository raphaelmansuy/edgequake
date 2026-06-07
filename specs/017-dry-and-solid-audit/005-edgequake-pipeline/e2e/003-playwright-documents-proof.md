# E2E Proof — Playwright Documents / Pipeline UI

**Date:** 2026-06-04 (full pipeline pass)  
**Result:** ✅ 4 passed (9.9s) — see also `008-full-pipeline-proof.md`

## Run

```bash
./specs/017-dry-and-solid-audit/005-edgequake-pipeline/e2e/run_playwright_proof.sh
# Runs spec017-pipeline-documents.spec.ts + spec017-pipeline-full-ingestion.spec.ts
```

## Screenshot analysis

### `01-documents-page-header.png`

- EdgeQuake shell with **Documents** nav active; breadcrumb `EdgeQuake > Documents`.
- Subtitle: *Upload and manage documents for knowledge graph extraction* — confirms pipeline ingestion route.
- Upload dropzone lists TXT, MD, JSON, PDF, images (100MB max); parser dropdown **Workspace Default**.
- Empty state **Documents (0)** with bootstrapped workspace tab `spec017-pipeline-docs`; version **v0.12.6**; no error overlay.
- Left nav shows full app shell (Dashboard, Knowledge Graph, Pipeline, Query, etc.) — confirms routing intact after pipeline refactor.
- **Verdict:** UI entry point healthy; backend health check passed EdgeQuake contract (`healthy` + `storage_mode`).

### `02-documents-main-panel.png`

- Search, **All Status (0)**, Created/Updated sort, drag-and-drop upload zone.
- Secondary **Upload Documents** CTA in empty state.
- **Parser for this upload** dropdown set to **Workspace Default** — ingestion config surface visible (relevant to pipeline parser registry on backend).
- **Verdict:** Document manager controls render; Playwright confirms `/documents` API request on load. Empty workspace expected for isolated E2E bootstrap.

## Limits

- `01`–`02`: UI shell + empty state only.
- Full pipeline proof: `03`–`06` in `008-full-pipeline-proof.md` (Completed + 13 entities via Mistral sync).
