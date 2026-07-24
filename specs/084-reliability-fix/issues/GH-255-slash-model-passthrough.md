# `GH-255` — Skip rewriting / double-joining slash model names (gateway passthrough)

> **Priority**: P1  
> **Audit status**: FIXED  
> **Sprint**: 1  
> **Laws**: LAW-14, LAW-3, LAW-8  
> **GitHub**: https://github.com/raphaelmansuy/edgequake/issues/255  
> **Verified against**: v0.21.0 / `19477c2d`  
> **Related PR**: https://github.com/raphaelmansuy/edgequake/pull/229 (unmerged; partial overlap)

---

## 1. WHY

Operators routing OpenAI-compatible traffic through gateways (Requesty, Portkey, corporate proxies) configure models as `provider/model` (e.g. `openai/gpt-4o-mini`, `deepinfra/minimax-m2.5`). EdgeQuake rewrites or double-joins these names → upstream `Invalid model` errors. Blocks EU-compliant / gateway deployments.

---

## 2. Audit (code is law)

| Field | Value |
|-------|-------|
| Issue proposed fix | `factory.rs` skip prefix if `contains('/')` |
| `edgequake-llm` 0.10.1 factory | **Does not auto-prefix** — passes model as-is |
| Actual breaker | `is_model_provider_mismatch` treats `model.contains('/')` as mismatch for `openai` (and others) → COMPAT-GUARD rewrites to default ([`safety_limits.rs`](../../../edgequake/crates/edgequake-api/src/safety_limits.rs) ~1173-1176) |
| Display ID | `llm_full_id()` always `format!("{}/{}", provider, model)` ([`workspace.rs`](../../../edgequake/crates/edgequake-core/src/types/multitenancy/workspace.rs)) → `openai/openai/...` |
| Verdict | **CONFIRMED** (diagnosis **updated** vs issue body) |

```rust
// safety_limits.rs — gateway breaker
"openai" | "anthropic" | "gemini" | "xai" | "minimax" => {
    is_local_style_model || model.contains('/')
}
```

---

## 3. Root cause (first principles)

**LAW-14**: When the operator already supplied a wire-ready model identity (`provider/model`), the platform must not mutate it for compatibility heuristics aimed at stale local model names.

COMPAT-GUARD conflates “slash means wrong provider” with “slash means gateway routing key.” Separately, display `llm_full_id` assumes model is bare.

---

## 4. Multi-lens analysis

### Product Owner

- Acceptance: With `EDGEQUAKE_LLM_PROVIDER=openai` (or openai-compatible) + custom base URL + model `deepinfra/minimax-m2.5`, chat/embed requests send that exact model string.
- Stale local models on cloud providers still get guarded (do not regress that safety).

### Full Stack

| Layer | Action |
|-------|--------|
| `safety_limits` | Allow slash when custom chat/base URL or explicit gateway allow flag |
| `llm_full_id` | If model contains `/`, return model as full id (or `provider` only once) |
| OpenAPI / workspace settings | Document gateway model format |
| PR #229 | Absorb: skip COMPAT-GUARD for openai when custom base URL |

### AI Engineer

- Gateways require stable `model` passthrough; silent rewrite to `gpt-4.1-nano` is worse than fail-fast.
- OpenRouter already expects slash models — keep that path working.
- Embeddings: same guard must not rewrite embedding model IDs inconsistently (SPEC-033 hybrid).

### O(n) / Systems

- Negligible perf; correctness/config SSOT only.

### Postgres Expert

- N/A.

---

## 5. ASCII causal diagram

```
  User sets model = "openai/gpt-4o-mini", provider = openai
        |
        +--> COMPAT-GUARD: contains('/') --> rewrite gpt-4.1-nano
        |
        +--> llm_full_id --> "openai/openai/gpt-4o-mini"
        |
        v
  Gateway rejects / wrong model billed
```

---

## 6. Solution (SOLID + DRY)

| Principle | Application |
|-----------|-------------|
| S | `WireModelId` / mismatch policy owns rewrite decisions |
| O | `MismatchPolicy::StrictCloud` vs `AllowGatewaySlash` |
| L | Vision + LLM + embed guards share same helper |
| I | `fn effective_model(provider, model, ctx) -> String` |
| D | Factory remains passthrough; EdgeQuake owns policy |
| DRY | One `is_model_provider_mismatch(provider, model, CompatContext)` |

### Implementation steps (locked)

1. Extend mismatch check: if `model.contains('/')` **and** (`OPENAI_BASE_URL` / `EDGEQUAKE_CHAT_BASE_URL` / openai-compatible base set **OR** `EDGEQUAKE_ALLOW_GATEWAY_MODEL_IDS=1`), **do not** treat as mismatch.
2. Still flag true local-style names (`llama3`, `gemma3:…`) on pure cloud OpenAI API without custom base.
3. Fix `llm_full_id()` / embedding full id: if model contains `/`, use model as the full id (no double prefix).
4. Merge intent of PR #229; add unit tests for Requesty-style IDs.
5. Docs: `.env.example` + FAQ gateway section.

**Explicitly not required:** change `edgequake-llm` factory to add skip-prefix (prefix does not exist there on 0.10.1).

---

## 7. Edge cases

| ID | Case | Mitigation |
|----|------|------------|
| EC-1 | Bare `gpt-4o-mini` on openai | Unchanged; no slash; OK |
| EC-2 | `ollama/gemma3` with provider ollama | Allowed; openrouter-style |
| EC-3 | Stale `llama3` on provider openai (no custom base) | Still mismatch → default |
| EC-4 | Slash model on anthropic without gateway | Allow only with custom base / flag |
| EC-5 | Vision provider same bug | Same helper |
| EC-6 | Hybrid SPEC-033 embed model with slash | Same passthrough rules |
| EC-7 | Empty model | No mismatch (existing) |

---

## 8. E2E / contract tests

| Test | Assertion |
|------|-----------|
| `issue255_gateway_slash_model_not_rewritten` | custom base + `deepinfra/minimax-m2.5` → effective_model unchanged |
| `issue255_llm_full_id_no_double_prefix` | provider openai + model `openai/gpt-4o-mini` → full_id `openai/gpt-4o-mini` |
| `issue255_local_model_on_openai_still_guarded` | no custom base + `gemma3:latest` → rewritten/warned |

---

## 9. Cross-refs

- PR #229  
- SPEC-032 provider/model full IDs  
- SPEC-033 hybrid providers  
- Issue body factory snippet — **obsolete relative to edgequake-llm 0.10.1**
