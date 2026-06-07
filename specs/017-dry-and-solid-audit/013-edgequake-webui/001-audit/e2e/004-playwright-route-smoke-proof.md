# Playwright — query + documents UI

**Status:** ✅ 4/4 webui screenshots + 5/5 route smoke (16.6s total)  
**Date:** 2026-06-04 19:13 UTC

```bash
bash /tmp/edgequake-start.sh   # backend :8081
cd edgequake_webui
PLAYWRIGHT_SKIP_STACK_CHECK=1 E2E_LIVE_STACK=1 EQ_BACKEND_URL=http://127.0.0.1:8081 \
  bunx playwright test e2e/spec017-webui-dry-solid.spec.ts e2e/spec017-barrel-smoke.spec.ts --project=chromium
```

## Screenshot analysis

### `03-query-mode-selector.png`

- Query nav active; **Hybrid** selected; Local / Global / Simple chips visible.
- Backend **v0.12.6** healthy (green status dot).
- Workspace context `spec017-webui-s…` in header.
- **Verdict:** Split API client + runtime config OK; no connection-error state.

### `04-query-main-panel.png`

- Suggestion cards, “Ask a question…” textarea, History sidebar.
- **Verdict:** Query shell renders after barrel split.

### `05-documents-upload-zone.png`

- Upload dropzone, parser dropdown, empty list skeleton (0 docs).
- **Verdict:** Document manager entry point healthy pre-ingestion.

### `06-sync-pipeline-completed.png`

- `spec017-webui-sync-*.md`: **Completed**, **9 entities**, **$0.00033**.
- **Verdict:** Sync path (`async_processing: false`) → chunk + extract → UI + `EnhancedStatusBadge`.

### `07-async-pipeline-completed.png`

- `spec017-webui-async-*.md`: **Completed**, **9 entities**, **$0.00033**.
- **Verdict:** Async task pipeline → poll terminal → UI shows Completed.
