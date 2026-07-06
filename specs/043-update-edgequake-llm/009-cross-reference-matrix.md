# SPEC-043 — Cross-Reference Matrix

| ID | Requirement | Spec | Backend | Frontend | Test |
| -- | ----------- | ---- | ------- | -------- | ---- |
| BR04301 | Pin edgequake-llm 0.10.1 | 003 | `Cargo.toml` | — | build |
| BR04302 | No dual LLMProvider trait | 008 P0 | `vision.rs` | — | pdf e2e |
| FEAT04310 | Attribution catalog API | 004 | `handlers/attribution.rs`, `attribution.rs` | `app-attribution-settings-card.tsx` | unit + OpenAPI |
| FEAT04311 | ApplicationContext on LLM calls | 004 | `attribution.rs`, `safety_limits.rs` | — | query e2e |
| FEAT04312 | Health attribution block | 004 | `handlers/health.rs`, `health_types.rs` | header badge | curl + OpenAPI |
| FEAT04320 | Model search API (live + static) | 002 | `handlers/models_search.rs`, `model_catalog.rs` | `model-picker-panel.tsx` | unit |
| FEAT04322 | Dynamic model catalog merge | 002 | `model_catalog.rs`, `/models/llm`, `/models/embedding` | `use-providers.ts`, mappers | e2e |
| FEAT04323 | Discovery cache refresh | 002 | `POST /models/discover/refresh` | `refreshDynamicModels()` | curl |
| FEAT04321 | Provider catalog API | 003 | `handlers/provider_catalog_api.rs` | `use-providers.ts` | unit |
| FEAT04330 | NVIDIA provider | 005 | `models.toml` | `provider-icon.tsx` | catalog test |
| FEAT04331 | Cohere provider | 005 | `models.toml` | icon | catalog test |
| FEAT04332 | Jina embeddings | 005 | `models.toml` | picker | embed test |
| FEAT04333 | HuggingFace | 005 | `models.toml` | icon | catalog test |
| FEAT04334 | VS Code Copilot | 005 | `models.toml` | icon | catalog test |
| FEAT04340 | ModelPickerPanel | 006 | — | `model-picker-panel.tsx` | bun test |
| FEAT04341 | Provider filter + search | 006 | — | selectors refactor | playwright |
| FEAT04343 | Model list keyboard + wheel | 010 | — | `use-scroll-contained-wheel.ts`, `model-picker-panel.tsx` | playwright 08–09 |
| FEAT04344 | edgequake-llm 0.10.1 discovery (LM Studio + Vertex) | 008 P0 | — | `model_catalog.rs` (no shim) | API + playwright 10 |
| FEAT04342 | ProviderStatusHub | 006 | — | `provider-status-hub.tsx` | visual |
| FEAT04350 | Server LLM defaults save | 007 | `handlers/llm_defaults.rs` | `server-llm-config-card.tsx` | admin e2e |
| FEAT04351 | App attribution save | 007 | `handlers/app_attribution.rs`, `server_config` | `app-attribution-settings-card.tsx` | admin e2e + OpenAPI |
| FEAT04360 | Vertex AI identity auth | 011 | `credentials.rs`, `models.rs`, resolver | provider-status-hub | health curl + e2e |

---

## Upstream cross-refs (edgequake-llm)

| edgequake-llm doc | EdgeQuake use |
| ----------------- | ------------- |
| `application_context.rs` | attribution build |
| `http/attribution.rs` | catalog header lists |
| `provider_catalog.rs` | `/settings/provider-catalog` |
| `discovery/service.rs` | `ModelCatalog` + `/models/llm` merge |
| `discovery/search.rs` | `/models/search` live queries |
| `factory.rs#create_llm_provider_with_context` | safety_limits |

---

## Prior specs superseded / extended

| Prior | Relationship |
| ----- | ------------ |
| SPEC-036 | Dependency bump methodology |
| SPEC-032 | Provider UI — extended by 043 |
| SPEC-018/014 | Header propagation — extended with ApplicationContext |
