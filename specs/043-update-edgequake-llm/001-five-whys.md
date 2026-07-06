# SPEC-043 — 5 WHY Analysis

## Problem statement

Users cannot efficiently select LLM providers/models, providers added in edgequake-llm are invisible in EdgeQuake, and application attribution headers never reach upstream LLM APIs.

---

## WHY 1: Why is LLM selection UX poor?

The WebUI renders a **flat scroll list** grouped by provider with no provider filter, no capability chips, and no unified search across the discovery API. Workspace selectors (`LLMModelSelector`) use raw `<Select>` while query selectors (`ProviderModelSelector`) use `<Command>` — **two divergent implementations** (DRY violation).

## WHY 2: Why are there two divergent picker implementations?

Model data comes from `GET /models/llm` (models.toml cards) but **edgequake-llm 0.10.0** adds `ModelDiscoveryService`, `CapabilityFilter`, and `ModelSearchQuery` that EdgeQuake never calls.

## WHY 3: Why doesn't EdgeQuake call the discovery API?

EdgeQuake pins **0.6.26** which lacks unified `ProviderCatalog`, `ApplicationContext`, and Cohere/Bedrock catalog entries. The API layer was built around static `models.toml` only.

## WHY 4: Why is EdgeQuake still on 0.6.26?

No spec tracked the 0.7–0.10 feature releases. `edgequake-pdf2md` transitive dep blocks naive bump (dual `LLMProvider` trait versions).

## WHY 5: Why does pdf2md block the bump?

Vision PDF passes `Arc<dyn LLMProvider>` across the pdf2md boundary; pdf2md compiles against its own edgequake-llm version. **Root fix:** decouple via `provider_name` + `model` factory path until pdf2md@0.9.3 aligns.

---

## Root causes (actionable)

| # | Root cause | Fix |
| - | ---------- | --- |
| R1 | Stale dependency pin | Bump to `0.10.0`, pdf2md workaround |
| R2 | No discovery API surface | `GET /models/search`, `/settings/provider-catalog` |
| R3 | No ApplicationContext wiring | `create_llm_provider_with_context` + attribution API |
| R4 | Duplicate UI pickers | Single `ModelPickerPanel` component |
| R5 | Settings read-only | `PATCH /settings/llm-defaults` → `server_config` table |

---

## Success criteria

1. `cargo build` with single edgequake-llm 0.10.0 (pdf2md isolated)
2. `/health` and `/settings/attribution` expose full attribution catalog
3. All ProviderCatalog LLM providers appear in models API (or documented opt-out)
4. Model picker: provider filter + search + capability chips in workspace + query + settings
5. Settings: save server LLM defaults without shell env editing
