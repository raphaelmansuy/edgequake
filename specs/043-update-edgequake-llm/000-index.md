# SPEC-043 — Update edgequake-llm to v0.10.1

**Spec:** `043-update-edgequake-llm`  
**Date:** 2026-07-05 (updated 2026-07-06)  
**Method:** Code is law — every claim maps to file, API route, or E2E proof  
**Trigger:** [edgequake-llm v0.10.1](https://github.com/raphaelmansuy/edgequake-llm/releases/tag/v0.10.1)

---

## Mission

Upgrade EdgeQuake from **edgequake-llm 0.6.26 → 0.10.1** and expose the new capabilities end-to-end:

| Track | Goal | Status |
| ----- | ---- | ------ |
| **A — Dependency** | Pin `0.10.1`, resolve pdf2md dual-version | ✅ |
| **B — Attribution** | Surface `ApplicationContext` + provider catalog in API | ✅ |
| **C — Providers** | Add missing catalog providers to `models.toml` | ✅ |
| **D — Discovery API** | Name + capability search via `ModelDiscoveryService` | ✅ |
| **E — UX/UI** | Provider-first model picker, capability filters, settings save | ✅ |

### v0.10.1 discovery note

Upstream [v0.10.1](https://github.com/raphaelmansuy/edgequake-llm/releases/tag/v0.10.1) fixes dynamic discovery for **LM Studio** ([#81](https://github.com/raphaelmansuy/edgequake-llm/issues/81)) and **Vertex AI** ([#82](https://github.com/raphaelmansuy/edgequake-llm/issues/82)). EdgeQuake removed the local `lmstudio_live_discovery.rs` shim; discovery is delegated to `ModelDiscoveryService` (SRP). EdgeQuake-only fallback: `toml_discovered_models()` when static registry misses local providers.

---

## Documents

| # | File | Lens |
| - | ---- | ---- |
| 01 | [001-five-whys.md](./001-five-whys.md) | Root-cause analysis |
| 02 | [002-first-principles.md](./002-first-principles.md) | Design axioms |
| 03 | [003-edgequake-llm-capability-matrix.md](./003-edgequake-llm-capability-matrix.md) | 0.6.26 → 0.10.0 delta |
| 04 | [004-application-attribution-api.md](./004-application-attribution-api.md) | API contract |
| 05 | [005-provider-expansion.md](./005-provider-expansion.md) | models.toml + factory |
| 06 | [006-ux-ui-model-picker.md](./006-ux-ui-model-picker.md) | ASCII screens + components |
| 07 | [007-settings-server-config.md](./007-settings-server-config.md) | Runtime config save |
| 08 | [008-implementation-plan.md](./008-implementation-plan.md) | Phased plan + battle tests |
| 09 | [009-cross-reference-matrix.md](./009-cross-reference-matrix.md) | FEAT/BR/API traceability |
| 10 | [010-model-picker-keyboard-scroll.md](./010-model-picker-keyboard-scroll.md) | List keyboard + wheel UX |
| 11 | [011-vertexai-authentication.md](./011-vertexai-authentication.md) | Vertex ADC / SA / token auth model |

---

## Dependency note (pdf2md)

`edgequake-pdf2md@0.9.2` still declares `edgequake-llm ^0.6.20`. Until pdf2md publishes `0.9.3+`, vision PDF extraction uses **provider_name + model** factory path inside pdf2md (no cross-version `Arc<dyn LLMProvider>` handoff). See [008-implementation-plan.md](./008-implementation-plan.md) §P0.

---

## Related

| Spec / Crate | Relationship |
| ------------ | ------------ |
| [SPEC-036](../036-update-dependant-crates/000-index.md) | Prior 0.6.26 upgrade |
| [SPEC-018/014](../018-observability/014-edgequake-llm/001-audit.md) | Header propagation audit |
| [edgequake-llm specs](https://github.com/raphaelmansuy/edgequake-llm/tree/main/specs/001-edgequake-llm) | Upstream discovery + attribution design |
