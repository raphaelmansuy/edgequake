# SPEC-018 — WebUI X-Request-ID proof

**Status:** ✅ Proven  
**Date:** 2026-06-05

## Claim

`buildHeaders()` in `client.ts` sets `X-Request-ID` on every `apiClient` request; errors surface `request_id` from response headers.

## Evidence

```bash
cd edgequake_webui && bun test src/lib/api/__tests__/observability-client.test.ts
```

## Code (law)

- `edgequake_webui/src/lib/api/client.ts` — `buildHeaders()`, `handleErrorResponse()`
