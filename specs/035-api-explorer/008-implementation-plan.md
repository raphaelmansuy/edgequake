# SPEC-035 — Implementation Plan

**Lens:** Full Stack Implementation  
**Decision:** Replace `api-explorer.tsx` with `@scalar/api-reference-react` consuming live OpenAPI spec  
**Status:** ✅ **IMPLEMENTED + REMEDIATED** (2026-07-01)  
**Actual effort:** ~6 hours  

---

## Phase 2 — UX Remediation (2026-07-01)

User-reported issues from live `/api-explorer` review:

| Issue                    | Root cause                                                                                         | Fix                                                                                                                            |
| ------------------------ | -------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| **Ask AI visible**       | Scalar agent enabled by default                                                                    | `agent: { disabled: true }` + CSS hide fallback                                                                                |
| **Theme not harmonized** | Hardcoded dark HSL overrides                                                                       | Map Scalar vars → EdgeQuake `--background`, `--foreground`, etc.; sync `forceDarkModeState` with `next-themes`                 |
| **Swagger 401**          | Link pointed at raw `:8080` (wrong service) + JWT middleware only allowed exact `/swagger-ui` path | Same-origin `/swagger-ui/` proxy + `openSwaggerUi()` prefills bearer; backend `is_public_documentation_path()` allows subpaths |
| **Broken navigation**    | `layout: modern` + Scalar developer toolbar on mobile                                              | `layout: classic`, `showToolbar: 'never'`, sidebar min-width CSS                                                               |

### New / updated files (Phase 2)

| File                                                | Change                                                  |
| --------------------------------------------------- | ------------------------------------------------------- |
| `src/lib/swagger-ui-launcher.ts`                    | **NEW** — proxied URL + auth localStorage prefill       |
| `src/lib/api-explorer-theme.ts`                     | Token bridge via CSS vars (light + dark)                |
| `src/lib/api-explorer-config.ts`                    | Classic layout, hide toolbar/client, agent disabled     |
| `src/hooks/use-api-explorer-config.ts`              | `useTheme()` → `forceDarkModeState`                     |
| `src/components/api-explorer/api-explorer-view.tsx` | Swagger button + `bg-background` chrome                 |
| `edgequake-api/src/middleware.rs`                   | Public docs subpaths for JWT middleware                 |
| `e2e/api-explorer.spec.ts`                          | +4 tests (Ask AI hidden, swagger proxy, theme, sidebar) |

---

## Implementation Summary

The custom 400-line hardcoded explorer (`components/shared/api-explorer.tsx`, 30 endpoints) was replaced with an OpenAPI-native Scalar integration. The frontend now has **zero hardcoded endpoint knowledge** — only one URL: `/api-docs/openapi.json`.

### Architecture (DRY / SOLID)

```
Rust #[utoipa::path] annotations
  └── edgequake-api/src/openapi.rs
        └── GET /api-docs/openapi.json
              └── Next.js dev proxy (/api-docs/* rewrite)
                    └── @scalar/api-reference-react (lazy-loaded)
                          ├── Auth: bearer_auth + tenant_id + workspace_id (Zustand)
                          ├── Theme: api-explorer-theme.ts (EdgeQuake tokens)
                          └── Config: api-explorer-config.ts (pure functions, unit-tested)
```

| Principle | Application                                                                                                                                                  |
| --------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **S**     | `ApiExplorerView` renders UI; `useApiExplorerConfig` binds stores; `buildScalarApiReferenceConfiguration` builds config; `api-explorer-theme.ts` handles CSS |
| **O**     | New Rust endpoints appear automatically — no frontend changes                                                                                                |
| **D**     | Page depends on config abstractions, not endpoint list                                                                                                       |
| **DRY**   | Single spec URL; dev proxy reuses `resolveDevProxyBackend()`                                                                                                 |

---

## Files Created / Modified

| File                                                                | Action                                                  |
| ------------------------------------------------------------------- | ------------------------------------------------------- |
| `edgequake_webui/package.json`                                      | Added `@scalar/api-reference-react@0.9.50`              |
| `edgequake_webui/next.config.ts`                                    | Added `/api-docs/*` and `/swagger-ui/*` dev rewrites    |
| `edgequake_webui/src/lib/api-explorer-config.ts`                    | **NEW** — pure config builders (SRP, testable)          |
| `edgequake_webui/src/lib/api-explorer-theme.ts`                     | **NEW** — Scalar CSS token mapping                      |
| `edgequake_webui/src/hooks/use-api-explorer-config.ts`              | **NEW** — React hook binding auth + tenant context      |
| `edgequake_webui/src/components/api-explorer/api-explorer-view.tsx` | **NEW** — explorer shell with `data-id` / `data-testid` |
| `edgequake_webui/src/app/(dashboard)/api-explorer/page.tsx`         | Rewritten — lazy Scalar + style import                  |
| `edgequake_webui/src/app/globals.css`                               | Scalar/Tailwind v4 layer ordering                       |
| `edgequake_webui/src/lib/__tests__/api-explorer-config.test.ts`     | **NEW** — 12 unit tests                                 |
| `edgequake_webui/src/lib/__tests__/swagger-ui-launcher.test.ts`     | **NEW** — 3 unit tests                                  |
| `edgequake_webui/e2e/api-explorer.spec.ts`                          | **NEW** — 10 E2E tests + screenshots                    |
| `edgequake_webui/src/components/shared/api-explorer.tsx`            | **DELETED** (400 lines removed)                         |

---

## Package Correction (Code is Law)

The spec originally referenced `@scalar/api-reference`. The correct React package is:

```bash
pnpm add @scalar/api-reference-react
```

Configuration uses top-level `url` (not `spec: { url }`). Auth scheme keys match OpenAPI: `bearer_auth`, `tenant_id`, `workspace_id`.

---

## E2E Instrumentation (`data-id` / `data-testid`)

| Attribute                   | Element                   |
| --------------------------- | ------------------------- |
| `api-explorer-page`         | Root page container       |
| `api-explorer-loading`      | Lazy-load spinner         |
| `api-explorer-header`       | Header bar                |
| `api-explorer-spec-url`     | Spec URL display          |
| `api-explorer-swagger-link` | "Open in Swagger UI" link |
| `api-explorer-scalar`       | Scalar mount container    |

Run E2E proof:

```bash
make dev-bg
cd edgequake_webui
E2E_LIVE_STACK=1 EQ_BACKEND_URL=http://localhost:8081 \
  pnpm exec playwright test e2e/api-explorer.spec.ts
```

Screenshots: `specs/035-api-explorer/e2e/screenshots/`

| Screenshot                        | Proof                             |
| --------------------------------- | --------------------------------- |
| `01-api-explorer-loaded.png`      | EdgeQuake API title visible       |
| `02-endpoints-visible.png`        | Health + Documents in sidebar     |
| `03-operation-count-proof.png`    | >30 OpenAPI paths (SSOT)          |
| `04-dark-theme.png`               | Dark theme matches dashboard      |
| `05-header-chrome.png`            | Spec URL + Swagger link           |
| `06-health-endpoint-selected.png` | Health endpoint selectable        |
| `07-no-ask-ai-clean-chrome.png`   | Ask AI + developer toolbar hidden |
| `08-theme-harmonized.png`         | Header/scalar match app theme     |
| `09-sidebar-navigation.png`       | Classic sidebar tag groups        |

---

## Migration Checklist

```
RESEARCH
- [x] Installed @scalar/api-reference-react@0.9.50
- [x] Verified ApiReferenceReact configuration interface (url, authentication.securitySchemes)
- [x] Confirmed auth scheme names: bearer_auth, tenant_id, workspace_id

IMPLEMENTATION
- [x] Created src/lib/api-explorer-config.ts (pure functions)
- [x] Created src/lib/api-explorer-theme.ts
- [x] Created src/hooks/use-api-explorer-config.ts
- [x] Created src/components/api-explorer/api-explorer-view.tsx
- [x] Updated src/app/(dashboard)/api-explorer/page.tsx
- [x] Added /api-docs dev proxy rewrite in next.config.ts

VERIFICATION (Phase 2)
- [x] Ask AI hidden (config + CSS)
- [x] Theme follows EdgeQuake light/dark via CSS var bridge
- [x] Swagger opens via same-origin proxy with bearer prefill
- [x] Backend allows `/swagger-ui/*` subpaths when auth enabled
- [x] Modern sidebar layout (Scalar default); developer toolbar hidden
- [x] Unit tests: 12/12 pass
- [x] E2E tests: 12 scenarios with visual QC (run with live stack)
- [x] Manual: /api-explorer loads Scalar with EdgeQuake API title
- [x] Manual: dark mode matches app design
- [x] Manual: >122 OpenAPI paths served (was 30 hardcoded)
- [x] Manual: auth + tenant/workspace prefilled from Zustand stores
- [x] Screenshots captured in specs/035-api-explorer/e2e/screenshots/

PHASE 3 — Visual QC & embedded UX polish (2026-07-01)
- [x] Root cause: `.references-sidebar { max-width: 22rem }` collapsed entire Scalar root to 352px
- [x] Switched to `layout: 'modern'` (Scalar official default) for tag sidebar navigation
- [x] Constrained Scalar React wrapper divs to dashboard pane height (flex chain)
- [x] Scroll parent: `.narrow-references-container` (not `.references-rendered`)
- [x] E2E visual QC: container width, viewport visibility, sidebar nav click, content scroll
- [x] Integration follows official docs: `@scalar/api-reference-react`, `theme: 'none'` + customCss,
      Tailwind `@layer` order in globals.css, `'use client'` + dynamic import (SSR off)

PHASE 4 — Tenant/Workspace auth layout polish (2026-07-01)
- [x] Auth prefill uses Scalar official apiKey shape: `{ name, in: 'header', value }` per configuration docs
- [x] Intro cards + section-columns forced to stack in embedded dashboard (no 3-col squeeze)
- [x] Redundant OpenAPI-fixed Name rows hidden; Value rows retain prefilled tenant/workspace UUIDs
- [x] E2E: `12-auth-tenant-workspace-polished.png` + 13/13 tests pass

PHASE 5 — Swagger UI proxy fix (2026-07-01)
- [x] Root cause: Next.js 308 `/swagger-ui/` → `/swagger-ui` vs backend 303 → `/swagger-ui/` redirect loop
- [x] `skipTrailingSlashRedirect: true` + `beforeFiles` swagger rewrites (both slash variants → backend `/swagger-ui/`)
- [x] E2E verifies `/swagger-ui/` returns 200 without redirect loop

CLEANUP
- [x] Removed src/components/shared/api-explorer.tsx
- [x] No remaining imports of legacy component
```

---

## Definition of Done — Status

| #   | Criterion                                    | Status                       |
| --- | -------------------------------------------- | ---------------------------- |
| 1   | Unit tests pass                              | ✅ 12/12                      |
| 2   | E2E suite passes                             | ✅ 12/12 (visual QC)          |
| 3   | GET /health reachable from explorer          | ✅                            |
| 4   | Bearer token prefilled when authenticated    | ✅ (via `bearer_auth` scheme) |
| 5   | >100 endpoints in OpenAPI spec               | ✅ 122 paths                  |
| 6   | Dark mode matches dashboard                  | ✅                            |
| 7   | Legacy component removed                     | ✅                            |
| 8   | No hardcoded endpoint imports                | ✅                            |
| 9   | Sidebar navigation + scrollable content pane | ✅                            |

---

## Rollback Plan

1. **30 seconds:** Redirect `/api-explorer` to backend `/swagger-ui/`
2. **2 hours:** Switch to `swagger-ui-react` (same architecture)
3. Legacy component preserved in git history

---

## Long-Term

"Open in Swagger UI" link is implemented in the explorer header (`api-explorer-swagger-link`).
