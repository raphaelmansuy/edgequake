# P2 — Playwright query UI visual proof

**Status:** ✅ Proven  
**Date:** 2026-06-03 16:19 UTC (re-captured)

## Spec

`edgequake_webui/e2e/spec017-query-pipeline.spec.ts`

## Screenshots

| File | Scene |
|------|-------|
| `screenshots/01-query-page-mode-selector.png` | `/query` with textarea input |
| `screenshots/02-query-main-panel.png` | Main panel crop |

## Commands

```bash
make db-start && make backend-bg BACKEND_PORT=8081 FRONTEND_PORT=3001
make frontend-bg BACKEND_PORT=8081 FRONTEND_PORT=3001
./specs/017-dry-and-solid-audit/006-edgequake-query/e2e/run_playwright_proof.sh
```

## Result

Playwright: **1 passed** (audit project, live stack `E2E_LIVE_STACK=1`).
