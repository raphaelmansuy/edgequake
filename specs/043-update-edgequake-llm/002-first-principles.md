# SPEC-043 — First Principles

## Axioms

1. **Single provider truth** — `edgequake_llm::ProviderCatalog` is canonical for IDs, features, attribution; `models.toml` adds deployment-specific model cards and costs.
2. **Attribution at the boundary** — Build `ApplicationContext` once per request in API middleware; merge env defaults + ingress headers + tenant/user IDs; pass to `ProviderFactory::create_llm_provider_with_context`.
3. **Discovery over heuristics** — UI search delegates to `find_static_models` / `ModelSearchQuery`; live discovery is opt-in (`?live=true`).
4. **Provider-before-model** — Humans choose provider first (availability, credentials), then model (capability, cost).
5. **Runtime config without redeploy** — Server-wide LLM defaults live in `server_config` JSONB; env vars remain override for ops.
6. **Identity ≠ API key** — Providers like `vertexai` and `bedrock` use OAuth2 / IAM credential ladders, not `api_key_env`. Health checks and UI copy must use `CredentialKind`, not a single key env var. See [011-vertexai-authentication.md](./011-vertexai-authentication.md).

---

## Architecture (target)

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         EdgeQuake WebUI                                  │
│  ModelPickerPanel ──► GET /models/search?provider=&q=&capability=       │
│  ProviderStatusHub ──► GET /settings/provider-catalog + /models/health │
│  ServerLlmConfigCard ──► GET/PATCH /settings/llm-defaults               │
└───────────────────────────────┬─────────────────────────────────────────┘
                                │ HTTP
┌───────────────────────────────▼─────────────────────────────────────────┐
│                      edgequake-api                                       │
│  attribution.rs ──► ApplicationContext (env + headers + tenant)         │
│  models_search.rs ──► find_static_models / ModelDiscoveryService        │
│  provider_catalog.rs ──► ProviderCatalog + models.toml merge            │
│  safety_limits.rs ──► create_safe_llm_provider_with_context             │
└───────────────────────────────┬─────────────────────────────────────────┘
                                │
┌───────────────────────────────▼─────────────────────────────────────────┐
│                   edgequake-llm 0.10.0                                   │
│  ProviderFactory::create_llm_provider_with_context                      │
│  http::attribution::resolve_attribution                                   │
│  discovery::{find_static_models, ModelSearchQuery}                        │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## SOLID mapping

| Principle | Application |
| --------- | ----------- |
| **SRP** | `attribution.rs` builds context; `models_search.rs` searches; UI `ModelPickerPanel` renders |
| **OCP** | New providers = `models.toml` entry + catalog auto-list; no factory match arms in API |
| **LSP** | All providers via `Arc<dyn LLMProvider>` + safety wrapper |
| **ISP** | Separate API responses for catalog vs model cards vs search hits |
| **DIP** | API depends on edgequake-llm traits, not provider HTTP details |

---

## DRY targets

| Duplicate today | Unified target |
| --------------- | -------------- |
| `LLMModelSelector`, `EmbeddingModelSelector`, `ProviderModelSelector` | `ModelPickerPanel` |
| Hardcoded provider display names in 4+ files | `provider-display.ts` + catalog API |
| `create_safe_llm_provider_with_headers` only merges extra_headers | `build_application_context()` merges all fields |
