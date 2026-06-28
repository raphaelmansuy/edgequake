# 008 — DRY Refactor: Generation Lens

**Spec:** 028-edgequake-query-service  
**Date:** 2026-06-28  
**Cross-ref:** [004-context-service-architecture.md](./004-context-service-architecture.md) | [017-006](../017-dry-and-solid-audit/006-edgequake-query/001-audit.md)  
**Goal:** Single retrieval SSOT; generation becomes a thin second step

---

## Current Duplication (QRY-002)

```
  BEFORE (today)
  ══════════════

  query_execute.rs ──────► build EngineRequest
        │                  map QueryContext → sources (150 LOC)
        │                  call execute_sota_query
        │
  chat/mod.rs ───────────► build EngineRequest (duplicate)
        │                  build_sources() (150 LOC duplicate)
        │                  call execute_sota_query
        │
  query_stream.rs ───────► run_context_pipeline
        │                  map context → stream event
        │                  stream LLM
```

---

## Target Architecture

```
  AFTER (SPEC-028 phase 3)
  ═══════════════════════

                    ┌─────────────────────────┐
                    │  QueryContextService    │
                    │  .retrieve() → Bundle   │
                    └───────────┬─────────────┘
                                │
            ┌───────────────────┼───────────────────┐
            │                   │                   │
            v                   v                   v
  /query/context          /query (gen)        /chat/completions
  (return bundle)              │                   │
                               v                   v
                    ┌─────────────────────────┐
                    │ QueryGenerationService  │
                    │ .generate(bundle, req)  │
                    └─────────────────────────┘
```

---

## Refactor Steps

### Step 1 — Extract `context_bundle_mapper.rs`

Single function:

```rust
pub fn map_engine_response_to_bundle(
    response: &QueryResponse,
    options: &MappingOptions,
    document_meta: &HashMap<String, DocumentMeta>,
) -> ContextBundle;
```

Unit tests with fixtures from `edgequake-query/tests/e2e_sota_engine.rs`.

### Step 2 — Implement `QueryContextService`

Move from handlers into service:
- `resolve_workspace_query_resources` call
- `document_filter_resolver` call
- `execute_sota_query` with `context_only=true`
- KV document title enrichment
- Bundle mapping

### Step 3 — Refactor `query_execute.rs`

```rust
// Pseudocode
let bundle_response = context_service.retrieve(ctx_request, auth).await?;

if request.context_only {
    return Ok(LegacyQueryResponse::from_bundle(&bundle_response, Citation));
}

let answer = generation_service
    .generate(&bundle_response.bundle, &gen_request, llm)
    .await?;

Ok(QueryResponse {
    answer,
    sources: bundle_response.to_legacy_sources(Citation),
    stats: merge_stats(bundle_response.stats, answer.stats),
    ..
})
```

### Step 4 — Refactor `chat/mod.rs`

Delete `build_sources`. Call `context_service.retrieve()` then `generation_service.generate()`.

Fix default mode: align to **Mix** (QRY-001) or document explicit chat default.

### Step 5 — Refactor `query_stream.rs`

```
  bundle = context_service.retrieve()
  emit ContextEvent (v2: legacy sources; v3: full bundle)
  stream = generation_service.stream(bundle, ...)
```

---

## What Stays in `edgequake-query`

| Concern | Location | Reason |
|---------|----------|--------|
| prepare/retrieve/finalize | `query_pipeline.rs` | Engine core |
| Prompt building | `prompt.rs` | Used by generation service via re-export OR move to shared crate |
| LLM streaming | `query_stream.rs` (engine) | Provider abstraction |
| Result cache | `query_result_cache.rs` | Engine-level |

**Do NOT** move pipeline logic to API crate.

---

## Shared Prompt Building

Option A (minimal): `QueryGenerationService` calls `QueryContext::to_context_string()` on internal conversion from `ContextBundle`.

Option B (cleaner): Extract `edgequake-query/src/prompt/` as public API:

```rust
pub fn build_rag_prompt(context: &str, query: &str, system_extension: Option<&str>) -> String;
```

Recommend **Option B** in phase 3 — deduplicates `prompt.rs` usage.

---

## Legacy Compatibility Matrix

| Client | Endpoint | Phase 3 behavior |
|--------|----------|------------------|
| WebUI query page | POST /query | Unchanged response shape |
| WebUI (future agent) | POST /query/context | New bundle shape |
| Ollama search | /ollama search | Uses context_service internally |
| SDK edgequake-core | `EdgeQuake::query()` | Unchanged; optional `retrieve_context()` added phase 4 |
| spec027 tests | /query | Unchanged |

---

## Extraction Checklist

| Item | From | To | LOC saved (est.) |
|------|------|----|--------------------|
| Source mapping | query_execute.rs | context_bundle_mapper.rs | ~120 |
| Source mapping | chat/mod.rs | context_bundle_mapper.rs | ~120 |
| Document title resolve | 2 handlers | context_service | ~40 |
| Engine request build | 2 handlers | context_service | ~60 |
| Workspace resolve | inline | context_service | ~30 |

**Total DRY gain:** ~370 LOC removed from handlers.

---

## Testing Strategy

| Test | Validates |
|------|-----------|
| `spec028_parity_legacy_context_only` | `/query?context_only` == pre-refactor snapshot |
| `spec028_parity_answer` | `/query` answer unchanged for fixture queries |
| `spec028_chat_parity` | `/chat/completions` sources match `/query` |
| `spec028_stream_v2_unchanged` | SSE v2 events byte-compatible |

Run alongside existing:
```bash
cargo test -p edgequake-api --test spec027_e2e
cargo test -p edgequake-api --test spec028_context_e2e
```

---

## Rollout

| Release | Action |
|---------|--------|
| v0.x + SPEC-028 phase 2 | Ship `/query/context` — no breaking changes |
| v0.x + SPEC-028 phase 3 | Internal refactor — parity tests gate merge |
| v0.x+1 | Deprecation header on `context_only` |
| v0.x+2 | Remove `context_only` (major version) |

Ascending compatibility per SPEC-027 IMP pattern.
