# SPEC-043 — Implementation Plan

## Phase P0 — Dependency (blocker)

- [x] Bump `edgequake-llm = "0.10.1"` in workspace Cargo.toml (LM Studio + Vertex discovery)
- [x] Fix pdf2md dual-version: vision uses `provider_name` + `model` (not `Arc<dyn LLMProvider>`)
- [x] `cargo build --workspace` green
- [x] `cargo test -p edgequake-api --lib` green

**Battle test:** upload PDF with vision backend; query with OpenAI override.

---

## Phase P1 — Application attribution

- [x] Add `edgequake-api/src/attribution.rs` — `build_application_context`, catalog builder
- [x] `create_safe_llm_provider_with_context` in `safety_limits.rs`
- [x] Wire resolver + query handlers
- [x] `GET /settings/attribution`, extend `/health`
- [x] OpenAPI/Swagger — attribution paths + schemas in `openapi.rs`
- [x] REST + configuration docs (`rest-api.md`, `configuration.md`)
- [x] Unit tests for context merge

**Battle test:** curl `/settings/attribution` shows all providers; query with `x-edgequake-app-id` logged in spans.

---

## Phase P2 — Discovery API

- [x] `GET /models/search` — live + static via `ModelDiscoveryService`; mock excluded
- [x] `POST /models/discover/refresh` — invalidate discovery cache
- [x] `GET /models/llm` and `/models/embedding` merge static + live catalogs
- [x] `provider_visibility.rs` — mock never surfaced in UI/API pickers
- [x] `models.toml` parity with edgequake-llm chat providers (+ bedrock disabled block)
- [x] Attribution catalog on `/settings/attribution` (provider header catalog)
- [ ] `GET /settings/provider-catalog` — optional dedicated endpoint (catalog merged into attribution)

**Battle test:** `make spec043-e2e` — 13 Playwright tests; mock never in pickers/API; provider parity unit test in `provider_catalog`.

---

## Phase P3 — Provider expansion

- [x] Add nvidia, cohere, jina, huggingface, vscode-copilot to models.toml
- [x] Credential checks + provider icons
- [ ] Optional bedrock block (disabled default)

**Battle test:** `/settings/providers` lists new IDs; health reflects credential state.

---

## Phase P3.1 — Vertex AI identity auth

- [x] `vertex_auth_configured()` OR-ladder in `credentials.rs` (project + ADC/SA/token)
- [x] Fix `check_provider_health` — no `"API key not configured"` for vertexai
- [x] Resolver uses `from_env_vertex_ai_adc()` async path
- [x] Provider Status Hub shows identity requirements

**Battle test:** Vertex shows offline with actionable identity message, not API key; ADC login → online.

See [011-vertexai-authentication.md](./011-vertexai-authentication.md).

---

## Phase P4 — UX/UI

- [x] `ModelPickerPanel` shared component (provider chips, search, capability filters, remote `/models/search`)
- [x] `model-picker-mappers.ts` — DRY LLM/embedding → picker option mappers + colon/slash format adapters
- [x] `LLMModelSelector` → `ModelPickerPanel` (workspace, vision, tenant create)
- [x] `EmbeddingModelSelector` → `ModelPickerPanel` variant `embedding` (dimension subline, no capability chips)
- [x] `ProviderModelSelector` (query) → delegates to `LLMModelSelector`
- [x] `ModelSelector` legacy facade → delegates to workspace selectors (`provider:model` colon format)
- [x] `ProviderStatusHub` replaces flat badge card on workspace page
- [x] `AppAttributionSettingsCard` on settings page
- [ ] `ServerLlmConfigCard` on settings page (P5)

**Battle test:** `make spec043-e2e` — Playwright 17 tests (16 pass, 1 skip), screenshots in `specs/043-update-edgequake-llm/e2e/screenshots/`.

---

## Phase P5 — Server config persistence

- [x] PATCH `/settings/app-attribution` (admin, postgres)
- [x] OpenAPI: `GET/PATCH /settings/app-attribution`, `GET /settings/attribution`
- [ ] GET/PATCH `llm-defaults` server-wide defaults
- [x] Load `app_attribution` from `server_config` at startup (env still wins)
- [x] PATCH applies attribution immediately via `install_app_attribution`
- [ ] Resolution ladder reads server_config for defaults

**Battle test:** PATCH defaults → new workspace inherits without env change.

---

## Component tree (final)

```
ModelPickerPanel (shared)
├── variant: llm | embedding
├── ProviderFilterBar (optional)
├── CapabilityFilterChips (llm only)
└── ModelResultList + /models/search

Consumers (all unified):
├── workspace/llm-model-selector.tsx
├── workspace/embedding-model-selector.tsx
├── query/provider-model-selector.tsx → LLMModelSelector
├── models/model-selector.tsx → facade (tenant-guard colon format)
├── settings/vision-llm-settings-card.tsx → LLMModelSelector
└── workspace/* (model config grid, create section, header selector)
```

---

## Roadblocks

| Risk | Mitigation |
| ---- | ---------- |
| pdf2md stuck on 0.6.x | provider_name path; track pdf2md 0.9.3 |
| Bedrock feature size | optional feature flag on edgequake-api |
| OpenRouter missing referer | warn in attribution API when app_url unset |
| Admin-only save | reuse ApiRequireAdmin |
| Port 5432 conflict | Makefile LISTEN-only detect, fallback 5433–5449 |

---

## Verification commands

```bash
# Full stack + E2E proof (recommended)
make spec043-e2e

# Manual
make dev-bg
cd edgequake_webui
EQ_BACKEND_URL=http://localhost:8081 E2E_LIVE_STACK=1 PLAYWRIGHT_SKIP_STACK_CHECK=1 \
  PLAYWRIGHT_BASE_URL=http://localhost:3000 \
  pnpm exec playwright test e2e/spec043-llm-model-picker.spec.ts --project=chromium

# Backend
cd edgequake
cargo test -p edgequake-api --lib attribution
cargo clippy -p edgequake-api -- -D warnings
```

## E2E screenshots

| File | Screen |
| ---- | ------ |
| `01-workspace-model-picker-edit-mode.png` | Workspace LLM picker + provider/capability chips |
| `02-model-picker-dropdown-open.png` | LLM dropdown with search |
| `03-model-picker-vision-filter.png` | Vision capability filter active |
| `04-provider-status-hub-expanded.png` | Provider Status Hub expanded |
| `05-settings-attribution-card.png` | Application Attribution card |
| `06-embedding-model-picker-open.png` | Embedding picker (provider chips, dimension subline) |
| `07-query-model-selector.png` | Query settings unified picker |
| `08-model-picker-keyboard-focus.png` | Keyboard ↓ highlight in open dropdown |
| `09-model-picker-wheel-scroll.png` | Wheel scroll inside list (page does not scroll) |
| `10-lmstudio-live-discovery.png` | LM Studio chip + Live badge (edgequake-llm 0.10.1) |
| `11-vertexai-identity-auth.png` | Vertex AI Identity (ADC) badge + requirements |
