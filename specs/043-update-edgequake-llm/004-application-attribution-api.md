# SPEC-043 — Application Attribution API

**Status:** Implemented (code is law)  
**Code anchors:** `edgequake-api/src/attribution.rs`, `handlers/attribution.rs`, `handlers/app_attribution.rs`, `safety_limits.rs`, `providers/resolver.rs`

## Goal

Surface **all** application attribution metadata that edgequake-llm 0.10.x propagates upstream, so operators, SDK clients, and the Settings UI know exactly what EdgeQuake sends to each LLM provider.

---

## Endpoints

### `GET /api/v1/settings/attribution`

Returns effective application context and per-provider attribution catalog.

**Handler:** `handlers::get_attribution_settings`  
**OpenAPI tag:** Settings  
**Auth:** Bearer / API key (tenant context headers optional)

```json
{
  "effective_context": {
    "app_id": "edgequake",
    "app_name": "EdgeQuake",
    "app_url": "http://localhost:3000",
    "tenant_id": null,
    "request_id": null,
    "end_user_id": null,
    "active": true,
    "sources": ["env:EDGEQUAKE_APP_ID", "env:EDGEQUAKE_APP_NAME"]
  },
  "providers": [
    {
      "id": "openai",
      "display_name": "OpenAI",
      "attribution_support": "full",
      "headers": ["X-Client-Request-Id"],
      "body_fields": ["user"]
    },
    {
      "id": "anthropic",
      "display_name": "Anthropic",
      "attribution_support": "full",
      "headers": ["x-application-id", "x-request-id"],
      "body_fields": []
    },
    {
      "id": "openrouter",
      "display_name": "OpenRouter",
      "attribution_support": "full",
      "headers": ["HTTP-Referer", "X-OpenRouter-Title", "X-Title"],
      "body_fields": []
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
    "EDGEQUAKE_APP_URL",
    "EDGEQUAKE_TENANT_ID"
  ]
}
```

**Catalog rules (DRY):**

- Provider list = `ProviderCatalog::all()` filtered by `provider_visibility::is_ui_visible_provider_id` and `features.chat`.
- Header/body lists = `edgequake_llm::http::attribution::resolve_attribution()` with sample context (not hardcoded per provider in API).
- Mock provider excluded from catalog.

### `GET /api/v1/settings/app-attribution`

Alias response — same `AttributionSettingsResponse` as above. Used by `AppAttributionSettingsCard`.

**Handler:** `handlers::get_app_attribution_settings`

### `PATCH /api/v1/settings/app-attribution` (admin)

Persist `{ app_id, app_name, app_url }` to PostgreSQL `server_config` key `app_attribution`.

**Handler:** `handlers::update_app_attribution`  
**Auth:** `ApiRequireAdmin`  
**Storage:** Requires `postgres` feature + `pg_pool`

```json
// Request
{ "app_id": "edgequake", "app_name": "EdgeQuake", "app_url": "http://localhost:3000" }

// Response
{ "saved": true, "note": "Saved to server_config. Env vars (EDGEQUAKE_APP_*) still apply at process start." }
```

> **P5:** Startup and PATCH both load `server_config.app_attribution` into `baseline_application_context()`; env vars win on conflict.

### `GET /health` (extended)

Under `attribution`:

```json
"attribution": {
  "app_id": "edgequake",
  "app_name": "EdgeQuake",
  "active": true
}
```

**Builder:** `attribution::health_attribution_summary()` from `ApplicationContext::from_env()`.

---

## Request-time context build

```
Request ──► observability_middleware (harvest headers)
         ──► build_application_context(propagation_headers, end_user_id)
                ├── ApplicationContext::from_env()
                ├── merge ingress x-edgequake-* (via from_ingress_headers)
                ├── merge propagation (x-request-id, traceparent, x-tenant-id)
                └── set end_user_id from auth
         ──► create_safe_llm_provider_with_context(provider, model, ctx)
```

**Code:** `attribution.rs::build_application_context`, `safety_limits.rs`, `providers/resolver.rs`.

---

## SOLID / DRY layout

| Module | Responsibility (SRP) |
| ------ | -------------------- |
| `attribution.rs` | Context merge + catalog DTOs + health summary |
| `handlers/attribution.rs` | GET `/settings/attribution` |
| `handlers/app_attribution.rs` | GET/PATCH `/settings/app-attribution` |
| `edgequake_llm::http::attribution` | Provider-specific header resolution (OCP — new providers need no API changes) |
| `AppAttributionSettingsCard` | Settings UI consumer |

---

## Policy

Default: `AttributionPolicy::BestEffort` (edgequake-llm). Set `EDGEQUAKE_ATTRIBUTION_POLICY=require_app_id` for strict mode.

---

## OpenAPI / docs

| Artifact | Location |
| -------- | -------- |
| Swagger paths | `openapi.rs` — `get_attribution_settings`, `get_app_attribution_settings`, `update_app_attribution` |
| Schemas | `AttributionSettingsResponse`, `EffectiveContextResponse`, `ProviderAttributionInfo`, `HealthAttributionSummary`, `UpdateAppAttributionRequest/Response` |
| REST docs | `docs/api-reference/rest-api.md` |
| Config docs | `docs/operations/configuration.md` § Application Attribution |

---

## Cross-refs

- edgequake-llm: `application_context.rs`, `http/attribution.rs`, `provider_catalog.rs`
- SPEC-018: header propagation middleware
- SPEC-043-007: `server_config` persistence
- FEAT04310, FEAT04311, FEAT04312, FEAT04351
