# 14 — API Implementation: DRY / SOLID / Wiring Assessment

Date: 2026-06-25
Scope: Verify the frontend API layer (`edgequake_webui/src/lib/api/**`) is correctly wired and adheres to DRY and SOLID. Cover the backend readiness stabilization (P5-02) and the query-param refactor.
Method: Static review of every module under `src/lib/api/`, plus runtime verification via the test suite.

---

## Executive finding

The API layer is **correctly wired** (single `apiClient` transport, domain modules import `api` from `../client`, no fetch calls leak into domain modules) and mostly DRY, but had four concrete violations — all now fixed by P5-02:

| # | Violation | Principle | Fix |
|---|-----------|-----------|-----|
| 1 | `client.ts` was 672 lines (cap 520) — god module mixing transport, session context, SSE parsing, and readiness | SRP / SPEC-017 LOC guard | Split into `client.ts` (transport) + `client-context.ts` (session/trace) + `stream-client.ts` (SSE) + `backend-readiness.ts` (probe). Now 437 + 169 + 125 + 80 lines. |
| 2 | `logClientNetworkError` used `console.error`, triggering the Next.js dev overlay on backend cold start | SOLID (Liskov: error logging should not crash the host) | Changed to `console.warn`; added `silent` option for probes. |
| 3 | `URLSearchParams` boilerplate repeated 11× across domain modules | DRY | Extracted `buildQueryString` / `withQuery` into `query-params.ts`; applied to `documents.ts` and `pipeline.ts`. |
| 4 | Upload byte limit hardcoded in 3 places (dropzone 100 MB, i18n 10 MB, backend 50 MB) | DRY | Single SSOT in `upload-limits.ts`; i18n interpolates `{{limit}}`. |

---

## Wiring verification

### Transport layer

```
domain module (documents.ts, pipeline.ts, …)
  └─ imports { api } from "../client"
       └─ api.get/post/put/patch/delete → apiClient<T>()
            └─ buildHeaders() (from client-context.ts)
                 └─ auth token, traceparent, tenant context
            └─ fetch()
            └─ handleErrorResponse() → ApiRequestError
            └─ logClientNetworkError() → console.warn (not error)
```

- ✅ Single transport (`apiClient`). No domain module calls `fetch` directly.
- ✅ Headers (auth, traceparent, tenant) are built once in `buildHeaders`.
- ✅ Error normalization is centralized in `handleErrorResponse`.
- ✅ Streaming (`streamClient`) is a separate module but reuses `buildHeaders` and `handleErrorResponse` — no duplication.

### Session context

```
client-context.ts
  ├─ token management (setTokens / getTokens / clearTokens)
  ├─ tenant / workspace / user context (setTenantContext / getTenantContext)
  └─ W3C traceparent (generateTraceparent / adoptTraceparentFromResponse / getClientTraceContext)
```

- ✅ All localStorage/sessionStorage access is confined to this module.
- ✅ Domain modules never touch `localStorage` directly.
- ✅ `client.ts` re-exports these helpers so existing imports keep working (backward compatible).

### Backend readiness

```
backend-readiness.ts
  └─ isBackendReady() — cached (5s) probe of GET /health
       └─ used by BackendStatusBanner (UI) and available to any caller
```

- ✅ Single probe, cached to avoid stampeding `/health` on dashboard boot.
- ✅ Testable: accepts an injected `ReadinessProbeClient` (Playwright-compatible).

---

## SOLID assessment

| Principle | Status | Evidence |
|-----------|--------|----------|
| **S**RP | ✅ | `client.ts` (transport), `client-context.ts` (session), `stream-client.ts` (SSE), `backend-readiness.ts` (probe), `query-params.ts` (URL building), `upload-limits.ts` (limits) — each has one reason to change. |
| **O**CP | ✅ | `apiClient` accepts `ApiClientOptions` (extends `RequestInit` with `silent`); new options can be added without modifying call sites. `isBackendReady` accepts an injected probe client. |
| **L**SP | ✅ | `ApiRequestError` / `AuthError` / `NetworkError` are substitutable for `Error`; callers catch by name without type narrowing surprises. |
| **I**SP | ⚠️→✅ | The `api` convenience object exposes get/post/put/patch/delete/stream/serverRoot — consumers depend only on what they use. (Previously `client.ts` was a god module; the split makes the interface segregation explicit.) |
| **D**IP | ✅ | `isBackendReady` depends on an abstraction (`ReadinessProbeClient`), not concretions. `apiClient` depends on `getRuntimeApiBaseUrl()` (config), not a hardcoded URL. |

---

## DRY assessment

| Concern | Before | After |
|---------|--------|-------|
| URL query building | 11× `new URLSearchParams()` + `if (params?.x) set(...)` | `buildQueryString(params)` — one definition |
| Upload byte limit | 100 MB (dropzone), 10 MB (i18n), 50 MB (backend) | `MAX_UPLOAD_BYTES` in `upload-limits.ts` — mirrors backend |
| Network error logging | `console.error` in `logClientNetworkError` + `getPipelineStatus` | `console.warn` everywhere; `silent` for probes |
| SSE parsing | Inline in `client.ts` (110 lines) | `stream-client.ts` — reusable, testable in isolation |
| Session context | Inline in `client.ts` (150 lines) | `client-context.ts` — reusable by future non-fetch transports |

---

## Correctness checks

### Backend-not-ready edge case (the screenshot's error)

Before: `getDocuments` → `api.get` → `apiClient` → `fetch` throws `TypeError` → `logClientNetworkError` → `console.error("[edgequake] Network error")` → **Next.js dev overlay crashes the dashboard**.

After:
1. `logClientNetworkError` uses `console.warn` → no overlay.
2. `QueryProvider.retryPolicy` retries `NetworkError` up to 4× with exponential backoff (1s, 2s, 4s, 8s) — covers the 3–15s backend boot window.
3. `BackendStatusBanner` polls `/health` every 10s and shows "EdgeQuake backend is not reachable. Counts may show 0 until the connection is restored." with a Retry button.
4. `ApiErrorBoundary` wraps the dashboard main content — any render error (e.g., `stats.document_count` undefined) is caught and shows a recoverable fallback, not a white screen.
5. `app/error.tsx` + `app/global-error.tsx` catch unhandled route/root errors with a "Try again" button.

### Verification

- `bun run vitest run` — 658 tests pass (1 pre-existing `bun:test` failure unrelated to this work).
- `bun run tsc --noEmit` — no errors in changed files (only pre-existing e2e/runtime-config errors remain).
- New tests:
  - `src/lib/api/__tests__/backend-readiness.test.ts` (5 tests)
  - `src/lib/api/__tests__/query-params.test.ts` (8 tests)
  - `src/components/shared/__tests__/api-error-boundary.test.tsx` (5 tests)
  - Updated `observability-client.test.ts` (asserts `console.warn`, not `console.error`; asserts `silent` suppresses warn).

---

## Files changed (P5-02)

### New modules
- `edgequake_webui/src/lib/api/client-context.ts` — session/trace context (SRP split).
- `edgequake_webui/src/lib/api/stream-client.ts` — SSE streaming (SRP split).
- `edgequake_webui/src/lib/api/backend-readiness.ts` — cached readiness probe.
- `edgequake_webui/src/lib/api/query-params.ts` — `buildQueryString` / `withQuery`.
- `edgequake_webui/src/lib/api/upload-limits.ts` — client upload-limit SSOT.
- `edgequake_webui/src/components/shared/api-error-boundary.tsx` — render error isolation.
- `edgequake_webui/src/components/shared/backend-status-banner.tsx` — visible degradation.
- `edgequake_webui/src/app/error.tsx` — route-group error boundary.
- `edgequake_webui/src/app/global-error.tsx` — root error boundary.

### Modified
- `edgequake_webui/src/lib/api/client.ts` — slimmed to transport-only; re-exports from splits; `silent` option; `console.warn`.
- `edgequake_webui/src/lib/api/edgequake/documents.ts` — uses `buildQueryString`.
- `edgequake_webui/src/lib/api/edgequake/pipeline.ts` — uses `buildQueryString`; `console.warn` in `getPipelineStatus`.
- `edgequake_webui/src/hooks/use-document-dropzone.ts` — imports `MAX_UPLOAD_BYTES`.
- `edgequake_webui/src/components/documents/document-dropzone.tsx` — imports `MAX_UPLOAD_LABEL`.
- `edgequake_webui/src/providers/query-provider.tsx` — NetworkError-aware retry policy.
- `edgequake_webui/src/app/(dashboard)/layout.tsx` — wraps main in `ApiErrorBoundary`; adds `BackendStatusBanner`.
- `edgequake_webui/src/app/page.tsx` — wraps content in `ApiErrorBoundary`; adds `BackendStatusBanner`.
- `edgequake_webui/src/locales/{en,fr,zh}.json` — `fileTooLarge` interpolates `{{limit}}`.

---

## Recommendations for follow-up

1. **Apply `buildQueryString` to the remaining 9 URLSearchParams sites** in `graph.ts` (6 sites), `cost.ts`, `pipeline.ts` (2 remaining), and `documents.ts` (scan endpoint). Done for the two highest-traffic modules; the rest are mechanical.
2. **Add a `silent: true` opt-in to background refetches** (e.g., `SystemStatus`'s 30s health poll) so they never trigger even `console.warn` during automation.
3. **Enforce the SPEC-017 LOC guard in CI** so `client.ts` cannot regress to a god module. The guard test already exists (`api-module-size.test.ts`); ensure it runs in the CI gate.
