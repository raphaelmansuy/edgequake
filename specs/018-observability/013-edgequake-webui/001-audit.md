# edgequake-webui — Observability Audit

**Path:** `edgequake_webui/`  
**Role:** Next.js 16 client — API + WebSocket

---

## Executive Summary

| Area | Grade | Notes |
|------|-------|-------|
| Client correlation | F | No `X-Request-ID` / `traceparent` in `client.ts` |
| Structured logging | F | ~45 `console.*` in `src/` |
| OTEL / instrumentation | F | No `instrumentation.ts` |
| Error surfacing | C | `ApiRequestError` without request_id from headers |
| Tenant context headers | A | `X-Tenant-ID`, `X-Workspace-ID`, `X-User-ID` |

---

## HTTP Client (code is law)

`src/lib/api/client.ts` `buildHeaders()` sets:

- `Authorization` (if token)
- `X-Tenant-ID`, `X-Workspace-ID`, `X-User-ID`

Does **not** set:

- `X-Request-ID`
- `traceparent`
- `X-Correlation-ID`

Evidence: lines 146-171.

---

## Findings

| ID | P | Finding | Evidence | Remediation |
|----|---|---------|----------|-------------|
| UI-OBS-001 | P0 | No request correlation | `client.ts` | `crypto.randomUUID()` per request |
| UI-OBS-002 | P2 | `console.log` in production paths | `graph-renderer.tsx` (11), stores | `no-console` + dev guard |
| UI-OBS-003 | P2 | WebSocket untraced | `progress-websocket.ts` | Pass `request_id` in WS auth payload |
| UI-OBS-004 | P2 | Errors drop response headers | `handleErrorResponse` | Parse `x-request-id` into `ApiRequestError` |
| UI-OBS-005 | P3 | OTEL API transitive only | `pnpm-lock.yaml` | Optional `@vercel/otel` |
| UI-OBS-006 | P3 | E2E uses console | `e2e/*.spec.ts` | OK |

---

## Target Client Header Builder (DRY)

```
buildHeaders()
  ├── Content-Type (existing)
  ├── Authorization (existing)
  ├── X-Tenant-ID / X-Workspace-ID / X-User-ID (existing)
  ├── X-Request-ID (NEW — per HTTP call)
  └── traceparent (NEW — when RUM span active)
```

---

## WebSocket Correlation

```
  Browser                    API
     │  WS connect + ?request_id=
     │──────────────────────▶
     │  progress events        (same id in logs)
```

File: `src/lib/websocket/progress-websocket.ts` (5 console calls today).

---

## Verify

```bash
rg 'console\.(log|warn|error|debug)' edgequake_webui/src -c
rg 'X-Request-ID|traceparent' edgequake_webui/src/lib/api
```

---

## Relation to 017 audit

UI-DRY-003 centralized fetch — **extend** `client.ts` for observability (same SRP module).
