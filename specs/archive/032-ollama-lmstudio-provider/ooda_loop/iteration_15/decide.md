# OODA Loop Iteration 15 - Decide Phase

**Date:** 2026-01-11  
**Focus:** Provider Selection at Query Time  
**Status:** ✅ COMPLETE

## Decision: Add LLM Provider to Query Request

### Approach

Add `llm_provider` and `llm_model` fields to `QueryRequest`, and use them in the LLM generation step.

The embedding provider stays fixed to the workspace configuration (required for correct vector search).

### Implementation Steps

1. **Add fields to QueryRequest:**

   ```rust
   pub llm_provider: Option<String>,
   pub llm_model: Option<String>,
   ```

2. **Add builder methods:**

   ```rust
   pub fn with_llm_provider(mut self, provider: impl Into<String>) -> Self
   pub fn with_llm_model(mut self, model: impl Into<String>) -> Self
   ```

3. **Update SOTAQueryEngine to accept provider override:**

   - Add method to create LLM provider from string
   - Use override if provided, else use default

4. **Wire through chat.rs:**

   ```rust
   if let Some(ref provider) = request.provider {
       engine_request = engine_request.with_llm_provider(provider);
   }
   ```

5. **Parse full model ID:**
   The WebUI sends format `provider/model`, e.g., "ollama/gemma3:12b"
   - Parse to extract provider and model
   - Set both on the request

### Files to Modify

| File                                 | Lines | Change                                  |
| ------------------------------------ | ----- | --------------------------------------- |
| `edgequake-query/src/engine.rs`      | ~20   | Add fields and builders to QueryRequest |
| `edgequake-api/src/handlers/chat.rs` | ~10   | Parse and pass provider from request    |
| `edgequake-api/src/handlers/chat.rs` | ~10   | Same for streaming handler              |

### Edge Cases

1. **Provider not available:** Fallback to workspace default, log warning
2. **Model not found:** Use provider's default model
3. **Empty provider string:** Treat as None

### Test Plan

1. Unit test: QueryRequest builder methods
2. Integration test: Query with provider override
3. E2E test: WebUI selector → API → correct provider used

### Risk Mitigation

1. **Breaking change:** QueryRequest fields are `Option<String>`, non-breaking
2. **Performance:** Provider creation is cached in factory
3. **Complexity:** Minimal changes, clear data flow
