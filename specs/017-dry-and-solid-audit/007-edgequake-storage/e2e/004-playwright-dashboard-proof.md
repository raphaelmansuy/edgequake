# E2E Proof — Playwright Dashboard Workspace Stats (UI)

**Date:** 2026-06-02  
**Spec:** SPEC-017 P0 — workspace-scoped graph stats visible in dashboard

## First-principle objective

The storage-layer fix is only complete when user-visible dashboard stats are fetched for the active workspace (`/workspaces/{id}/stats`).

## Run

```bash
# Rust contracts + compile
./specs/017-dry-and-solid-audit/007-edgequake-storage/e2e/run_storage_e2e.sh

# UI proof (starts stack if needed; detects EdgeQuake port 3000 vs 3001)
./specs/017-dry-and-solid-audit/007-edgequake-storage/e2e/run_playwright_proof.sh
# or: ./specs/.../run_storage_e2e.sh --playwright
```

## Current real state

- Playwright spec exists: `edgequake_webui/e2e/spec017-storage-workspace-stats.spec.ts`
- Runner is wired: `e2e/run_storage_e2e.sh --playwright`
- Rust-side e2e suite is green and logged in `e2e/001-test-run.log` (97 tests).
- UI proof captured (refreshed 2026-06-03):
  - `screenshots/05-dashboard-workspace-stats.png`
  - `screenshots/06-dashboard-stats-main.png`
  - `screenshots/07-conversations-query-panel.png`
  - `screenshots/08-conversations-history-header.png`

### Root-cause and fix (2026-06-02)

```bash
# Root cause: using 127.0.0.1 for frontend base URL triggers Next.js dev
# cross-origin blocking for /_next/webpack-hmr.
# Fix: use localhost for frontend URL.

# Prefer the proof script (auto-starts stack, detects UI port):
./specs/017-dry-and-solid-audit/007-edgequake-storage/e2e/run_playwright_proof.sh

# Manual (example ports after make dev-bg):
# E2E_LIVE_STACK=1 EQ_BACKEND_URL=http://127.0.0.1:8081 PLAYWRIGHT_BASE_URL=http://localhost:3001 \
#   bunx playwright test e2e/spec017-storage-*.spec.ts --project=audit --workers=1
```

## Acceptance criteria

1. Dashboard stats cards are visible (`[data-testid="stats-card"]`)
2. A `/stats` request includes the bootstrapped workspace ID
3. PNG artifacts are written in:
   - `screenshots/05-dashboard-workspace-stats.png`
   - `screenshots/06-dashboard-stats-main.png`
