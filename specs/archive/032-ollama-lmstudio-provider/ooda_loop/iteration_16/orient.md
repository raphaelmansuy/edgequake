# OODA Loop Iteration 16 - Orient Phase

**Date:** 2026-01-11  
**Focus:** Architecture for LLM Provider Override  
**Status:** ✅ COMPLETE

## Architecture Analysis

### LLM Generation Flow

```
┌───────────────────────────────────────────────────────────┐
│ SOTAQueryEngine                                            │
│                                                            │
│  query(request) ──┐                                        │
│                   │                                        │
│         ┌────────▼────────┐                               │
│         │ generate_answer │                               │
│         └────────┬────────┘                               │
│                  │                                         │
│                  ▼                                         │
│    self.llm_provider.complete(&prompt)                    │
│         ❌ Always uses engine's default                    │
└───────────────────────────────────────────────────────────┘
```

### Solution Design

```
┌───────────────────────────────────────────────────────────┐
│ SOTAQueryEngine                                            │
│                                                            │
│  query_with_llm_provider(request, llm_override) ──┐       │
│                                                    │       │
│         ┌─────────────────────────────────────────▼──┐    │
│         │ generate_answer_with_provider(           │      │
│         │   query, context, Some(&llm_override))   │      │
│         └──────────────────────────────────────────┬─┘    │
│                                                    │       │
│                  ▼                                 ▼       │
│    llm_override.complete(&prompt)  OR  self.llm_provider  │
│         ✅ Uses override if provided                       │
└───────────────────────────────────────────────────────────┘
```

## Implementation Approach

1. **Internal method:** `generate_answer_with_provider(query, context, llm_override)`

   - Accepts `Option<&Arc<dyn LLMProvider>>`
   - Uses override if `Some`, else default

2. **Public method:** `query_with_llm_provider(request, llm_provider)`

   - Similar to `query_with_embedding_provider`
   - Calls `generate_answer_with_provider` with the override

3. **chat.rs changes:**
   - Parse `request.provider` to get provider name and model
   - Call `ProviderFactory::create_llm_provider`
   - Use `query_with_llm_provider` if override is created

## Files to Modify

| File             | Change                                     |
| ---------------- | ------------------------------------------ |
| `sota_engine.rs` | Add `generate_answer_with_provider` method |
| `sota_engine.rs` | Add `query_with_llm_provider` method       |
| `chat.rs`        | Create LLM provider and use new method     |
| `lib.rs`         | Re-export `LLMProvider` trait              |
