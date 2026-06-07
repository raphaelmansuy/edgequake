# edgequake-webui — DRY & SOLID Audit

**Path:** `edgequake_webui/`  
**Role:** Next.js 16 + React 19 client for EdgeQuake REST/WebSocket API  
**Last verified:** 2026-06-04 23:11 UTC (vitest 614/614, Playwright 9/9, Rust fmt+clippy)

---

## Executive Summary

| Area | Status |
|------|--------|
| P1 API god-module split | ✅ Done |
| P1 status badges | ✅ Composition (base + enhanced) |
| P2 QueryMode parity | ✅ `mix` / `bypass` in types; UI shows 4 selector modes |
| P2 types split | ✅ Domain barrels under `src/types/` |
| P2 raw fetch | ✅ Centralized in `client.ts` (+ PDF HEAD in viewer) |
| E2E proof | ✅ 4/4 webui + 5/5 route smoke + sync/async pipeline (see [e2e index](001-audit/e2e/000-e2e-index.md)) |
| Remaining debt | P2–P3: large page components, `client.ts` 507 LOC, OpenAPI codegen optional |

The original audit overstated current debt: **`edgequake.ts` is no longer ~2,035 LOC** — it was split before this verification pass. Remaining work is mostly **documentation, E2E ergonomics, and incremental SRP** on pages.

---

## Remediation Status

### DRY

| ID | P | Was | Now | Evidence |
|----|---|-----|-----|----------|
| UI-DRY-001 | P1 | Monolithic API ~2,035 LOC | ✅ Split | `lib/api/edgequake/*.ts` + barrel; [001-api-barrel-split-proof](001-audit/e2e/001-api-barrel-split-proof.md) |
| UI-DRY-002 | P1 | Dual status badges | ✅ Compose | `EnhancedStatusBadge` wraps `StatusBadge`; [003-status-badge-dry-proof](001-audit/e2e/003-status-badge-dry-proof.md) |
| UI-DRY-003 | P2 | Raw fetch in API | ✅ Fixed | Only `client.ts` (+ `pdf-viewer` HEAD); `server-root-client.test.ts` |
| UI-DRY-004 | P2 | QueryMode in stores | ✅ Fixed | Stores import `@/types` |
| UI-DRY-005 | P2 | Missing mix/bypass | ✅ Fixed | `QUERY_MODES` + `QUERY_MODES_SELECTOR`; [002-query-mode-parity-proof](001-audit/e2e/002-query-mode-parity-proof.md) |
| UI-DRY-006 | P2 | types monolith | ✅ Split | `types/index.ts` barrel → domain files |
| UI-DRY-007 | P3 | Parallel query stores | ✅ Documented | Boundary in `use-query-ui-store.ts` |
| UI-DRY-008 | P2 | Duplicate math render paths | ✅ Fixed | `MathTokenRenderer` + `math-marked-extensions` + `katex-render`; [LaTeX e2e](e2e/001-latex-rendering-proof.md) |

### SOLID

| ID | P | Status | Notes |
|----|---|--------|-------|
| UI-SOLID-S-001 | P1 | ✅ | API SRP via domain modules |
| UI-SOLID-S-002 | P2 | ⚠️ Open | Document manager / query interface still large |
| UI-SOLID-O-001 | P2 | ✅ | New endpoints → add domain file, not god module |
| UI-SOLID-I-001 | P2 | ⚠️ Partial | Prefer selectors over time |
| UI-SOLID-D-001 | P2 | ⚠️ Partial | Hooks pattern exists; not universal |

---

## Verification (2026-06-04 23:04 UTC)

| Check | Result |
|-------|--------|
| `bun run test` (vitest) | ✅ 614/614 |
| Playwright webui + smoke | ✅ 9/9 (14.7s) via `./001-audit/e2e/run_playwright_proof.sh` |
| `cargo fmt --check` | ✅ |
| `cargo clippy --workspace -D warnings` | ✅ |

```bash
bash /tmp/edgequake-start.sh   # or make backend-bg BACKEND_PORT=8081
cd edgequake_webui
PLAYWRIGHT_SKIP_STACK_CHECK=1 E2E_LIVE_STACK=1 EQ_BACKEND_URL=http://127.0.0.1:8081 \
  bunx playwright test e2e/spec017-webui-dry-solid.spec.ts e2e/spec017-barrel-smoke.spec.ts --project=chromium
```

**LOC guard:** `api-module-size.test.ts` — no `lib/api` file > 500 LOC except `client.ts` (520 cap).

**E2E ergonomics:** `playwright.config.ts` wires `EDGEQUAKE_API_URL` into `webServer`; `global-setup.ts` skips 90s poll when `PLAYWRIGHT_SKIP_STACK_CHECK=1` (backend must already be up).

---

## E2E Proof Index

See [001-audit/e2e/000-e2e-index.md](001-audit/e2e/000-e2e-index.md) and screenshots `03`–`07` with analysis in [004-playwright-route-smoke-proof.md](001-audit/e2e/004-playwright-route-smoke-proof.md).

---

## Brutal honesty

1. **Original LOC figures are stale** — do not use “~2,035 LOC edgequake.ts” in new docs; use domain file list.
2. **E2E is environment-sensitive** — port 8080 may be a non-EdgeQuake service; use `8081` + `make dev-bg` or `EQ_BACKEND_URL`.
3. **`bun test` without vitest** picks up Playwright specs and fails — use `bun run test` (vitest).
4. **`safe-build.sh` typecheck** still reports legacy errors in unrelated `e2e/*.spec.ts` files; `bun run test` and focused `tsc` on `src/` are clean for this change set.
5. **PDF UI path** not in webui spec; use `e2e/spec017-api-query-documents.spec.ts` (API audit) for multipart PDF proof.
6. **Sync + async text pipeline** proven in webui (`06`, `07` screenshots).

---

## Positive Patterns (unchanged)

- Central `apiClient` with auth refresh (`client.ts`)
- React Query for server state
- Feature ID traceability in JSDoc
- Dedicated hooks (`use-document-mutations.ts`, `use-query-page-state.ts`)
