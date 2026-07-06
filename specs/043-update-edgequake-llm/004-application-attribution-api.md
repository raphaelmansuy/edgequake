# SPEC-043 — Application Attribution API

## Goal

Surface **all** application attribution metadata that edgequake-llm 0.10.0 can propagate, so operators and SDK clients know exactly what EdgeQuake sends upstream.

---

## Endpoints

### `GET /api/v1/settings/attribution`

Returns effective application context and per-provider attribution catalog.

```json
{
  "effective_context": {
    "app_id": "edgequake",
    "app_name": "EdgeQuake",
    "app_url": "https://edgequake.example.com",
    "tenant_id": null,
    "request_id": null,
    "end_user_id": null,
    "sources": ["env:EDGEQUAKE_APP_ID", "env:EDGEQUAKE_APP_NAME"]
  },
  "providers": [
    {
      "id": "openai",
      "display_name": "OpenAI",
      "attribution_support": "full",
      "headers": ["X-Client-Request-Id"],
      "body_fields": ["user"]
    }
  ],
  "ingress_headers": [
    "x-edgequake-app-id",
    "x-edgequake-app-name",
    "x-edgequake-app-url",
    "x-edgequake-tenant-id",
    "x-edgequake-request-id"
  ],
  "environment_variables": [
    "EDGEQUAKE_APP_ID",
    "EDGEQUAKE_APP_NAME",
    "EDGEQUAKE_APP_URL"
  ]
}
```

### `GET /health` (extended)

Add under `attribution`:

```json
"attribution": {
  "app_id": "edgequake",
  "app_name": "EdgeQuake",
  "active": true
}
```

---

## Request-time context build

```
Request ──► observability_middleware (harvest headers)
         ──► build_application_context(state, request_extensions, user_id)
                ├── ApplicationContext::from_env()
                ├── merge ingress x-edgequake-* headers
                ├── merge propagation (traceparent, x-tenant-id)
                └── set end_user_id from auth
         ──► create_safe_llm_provider_with_context(provider, model, ctx)
```

**Code anchor:** `edgequake-api/src/attribution.rs` (new), `safety_limits.rs`, `providers/resolver.rs`.

---

## Policy

Default: `AttributionPolicy::BestEffort`. Set `EDGEQUAKE_ATTRIBUTION_POLICY=require_app_id` for strict mode.

---

## Cross-refs

- edgequake-llm: `src/application_context.rs`, `src/http/attribution.rs`
- SPEC-018: header propagation middleware
- FEAT: attribution catalog exposure (new)
