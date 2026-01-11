# OODA Loop Iteration 15 - Orient Phase

**Date:** 2026-01-11  
**Focus:** Gap Analysis - Provider Selection at Query Time  
**Status:** ✅ COMPLETE

## Architecture Analysis

### Current Query Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│ WebUI Query Interface                                                    │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │ ProviderModelSelector                                              │   │
│  │   ⚠️ SHOWS dropdown but provider NOT used!                        │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                   │                                      │
│                                   ▼                                      │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │ ChatCompletionRequest { provider: Some("ollama") }               │   │
│  │   ✅ Provider field EXISTS in request                             │   │
│  └──────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ Backend chat.rs Handler                                                  │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │ chat_completion()                                                  │   │
│  │   ❌ request.provider IGNORED!                                     │   │
│  │   ❌ Always uses global state.sota_engine                          │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                   │                                      │
│                                   ▼                                      │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │ state.sota_engine.query(engine_request)                           │   │
│  │   ❌ Uses global LLM provider, not request-specific               │   │
│  └──────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
```

## Gap Identified

**Location:** [chat.rs#L359-L362](../../edgequake/crates/edgequake-api/src/handlers/chat.rs#L359-L362)

```rust
// 3. Build and execute query using SOTA engine (LightRAG-style)
let mut engine_request = EngineQueryRequest::new(&request.message).with_mode(query_mode);

engine_request = engine_request.with_tenant_id(tenant_id.to_string());
if let Some(ref ws_id) = workspace_id {
    engine_request = engine_request.with_workspace_id(ws_id.to_string());
}

let result = state
    .sota_engine          // ❌ Uses global engine, ignores request.provider
    .query(engine_request)
    .await
```

## What's Needed

### Option 1: Pass Provider to Query Engine

The `EngineQueryRequest` struct needs a `provider` field, and the query engine needs to use it.

**Pros:**
- Clean separation of concerns
- Provider can be different from workspace default

**Cons:**
- Requires changes to query engine

### Option 2: Create Provider-Specific Engine

Similar to ingestion flow (OODA-11), create a workspace-specific engine with the requested provider.

**Pros:**
- Consistent with ingestion approach
- Clear isolation

**Cons:**
- More overhead per request

### Option 3: Use Request Provider for LLM Generation Only

The query flow uses:
1. **Embedding Provider** → Fixed to workspace config (for vector search)
2. **LLM Provider** → Can vary per request (for answer generation)

**Pros:**
- Only LLM varies, embedding stays consistent
- Most flexible for users

**Cons:**
- Need to wire provider selection into LLM generation step

## Recommendation

**Option 3 is best** - Allow LLM provider selection at query time while keeping embedding fixed to workspace.

This matches the spec requirement:
> "We want an easy way to change provider on the query dialogue for example with selection dropdown"

## Implementation Plan

1. Add `llm_provider` field to `EngineQueryRequest`
2. Modify `SOTAQueryEngine::query()` to accept optional LLM provider
3. In `chat.rs`, pass `request.provider` to the engine request
4. Update streaming handler similarly

## Files to Modify

| File | Change |
|------|--------|
| `edgequake-query/src/types.rs` | Add `llm_provider: Option<String>` to QueryRequest |
| `edgequake-query/src/engine.rs` | Handle provider in query execution |
| `edgequake-api/src/handlers/chat.rs` | Pass `request.provider` to engine request |
| `edgequake-api/src/handlers/chat.rs` | Same for streaming handler |

## Related SPEC-032 Requirements

From the spec:
> "VERY IMPORTANT: a model selected is in reality a combination of provider + model name. For example 'ollama/gemma3:12b'"

The current ProviderModelSelector uses format `provider/model`, so we need to:
1. Parse `request.provider` to extract provider name and model name
2. Use provider name to select the LLM provider
3. Use model name to override the model

## Risks

1. **Performance:** Creating provider per request may be slow
   - Mitigation: Cache providers in factory

2. **Consistency:** Different provider may generate different style responses
   - Mitigation: This is expected behavior, user chose it

3. **Error handling:** Selected provider may not be available
   - Mitigation: Fallback to workspace/server default with warning
