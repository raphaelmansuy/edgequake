# OODA-228: Fix Dimension Mismatch in Chat Query Handler

## Problem Statement

When using the chat query endpoint (`/api/v1/chat/completions`) with a workspace configured with Ollama embedding provider (768 dimensions), queries would fail with error:

```
Vector query failed: different vector dimensions 1536 and 768
```

**Root Cause**: The chat handler used `query_with_llm_provider()` method which only overrides the LLM provider for answer generation, but continues using the global/default embedding provider (1536 dims for OpenAI) instead of workspace-specific embedding provider (768 dims for Ollama).

## Solution Overview

The fix ensures that **both** the chat endpoint and streaming endpoints respect workspace-specific embedding and vector storage configurations, not just the query endpoint.

### Key Changes

#### 1. Made Helper Functions Public (query.rs)

**Functions made public**:

- `get_workspace_embedding_provider()` - Retrieves workspace-specific embedding provider
- `get_workspace_vector_storage()` - Retrieves workspace-specific vector storage with proper dimensions

**Why**: These helper functions were previously private to query.rs, preventing reuse in chat.rs handler.

#### 2. Added `query_with_full_config()` Method (sota_engine.rs)

**New method signature**:

```rust
pub async fn query_with_full_config(
    &self,
    request: QueryRequest,
    embedding_provider: Arc<dyn EmbeddingProvider>,
    vector_storage: Arc<dyn VectorStorage>,
    llm_provider: Option<Arc<dyn LLMProvider>>,
) -> Result<QueryResponse>
```

**What it does**:

- Computes query embeddings using **workspace-specific embedding provider** (not default)
- Retrieves context from **workspace-specific vector storage** (not default)
- Generates answer using **optional LLM override** (for user-selected model)
- Ensures end-to-end dimension consistency

**Why**: Combines all three components (embedding + storage + LLM) which was previously not possible in a single method.

#### 3. Added `query_stream_with_full_config()` Method (sota_engine.rs)

**New method signature**:

```rust
pub async fn query_stream_with_full_config(
    &self,
    request: QueryRequest,
    embedding_provider: Arc<dyn EmbeddingProvider>,
    vector_storage: Arc<dyn VectorStorage>,
    llm_provider: Option<Arc<dyn LLMProvider>>,
) -> Result<(QueryContext, QueryMode, BoxStream<'static, Result<String>>)>
```

**What it does**:

- Streaming variant of `query_with_full_config`
- Returns context first, then streams tokens from answer generation
- Same workspace isolation as non-streaming variant

#### 4. Updated `chat_completion` Handler (chat.rs)

**Before**:

```rust
// Only supports LLM override
state.sota_engine.query_with_llm_provider(request, llm_override).await
```

**After**:

```rust
// Get workspace embedding + vector storage
let ws_embedding = get_workspace_embedding_provider(&state, ws_id)?;
let ws_vector = get_workspace_vector_storage(&state, ws_id)?;

// Use full config if available
match (ws_embedding, ws_vector) {
    (Some(embed), Some(vector)) => {
        // Use workspace config + LLM override
        sota_engine.query_with_full_config(
            request,
            embed,
            vector,
            llm_override
        ).await
    }
    // Fallback for partial configs...
}
```

#### 5. Updated `chat_completion_stream` Handler (chat.rs)

**Applied same logic as non-streaming handler** for streaming queries, ensuring:

- Streaming uses workspace embedding dimensions
- Streaming uses workspace vector storage
- Streaming still respects LLM override for answer generation

## Testing Verification

### Scenario: Ollama Workspace with 768-dim embedding

**Setup**:

```bash
# Workspace configuration
embedding_provider: ollama
embedding_model: nomic-embed-text  (768 dimensions)
llm_provider: ollama
llm_model: gemma3:latest
```

**Before Fix**:

```
POST /api/v1/chat/completions
Query: "Tell me about document"

ERROR: Vector query failed: different vector dimensions 1536 and 768
```

**After Fix**:

```
POST /api/v1/chat/completions
Query: "Tell me about document"

✅ SUCCESS: Query embedding uses 768 dims (from workspace)
✅ SUCCESS: Vector storage retrieves 768-dim vectors (from workspace)
✅ SUCCESS: Answer generated with selected LLM model
```

### Key Test Cases Covered

1. **Workspace embedding + storage + LLM override**: ✅ Uses all three
2. **Workspace embedding + storage (no override)**: ✅ Uses default LLM
3. **Workspace embedding only (no vector storage)**: ✅ Falls back gracefully
4. **No workspace config**: ✅ Uses all defaults
5. **Streaming with full config**: ✅ Returns context then streams tokens
6. **Non-streaming with full config**: ✅ Returns full response with stats

## Impact on Architecture

### Before (SPEC-032 v1)

- Query endpoint: ✅ Respected workspace embedding/storage
- Chat endpoint: ❌ Ignored workspace embedding/storage (dimension mismatch)

### After (SPEC-032 v2)

- Query endpoint: ✅ Respected workspace embedding/storage (unchanged)
- Chat endpoint: ✅ **Now respects workspace embedding/storage** (FIXED)
- Streaming: ✅ **Now respects workspace embedding/storage** (NEW)

## Documentation Updates

### Implementation Details

- `@implements SPEC-032`: Workspace-specific embedding in query process
- `@implements SPEC-033`: Workspace vector isolation
- `@implements OODA-228`: Fix dimension mismatch in chat handler

### Code References

- [query_with_full_config](./edgequake/crates/edgequake-query/src/sota_engine.rs#L1046)
- [query_stream_with_full_config](./edgequake/crates/edgequake-query/src/sota_engine.rs#L1237)
- [Updated chat_completion](./edgequake/crates/edgequake-api/src/handlers/chat.rs#L450)
- [Updated chat_completion_stream](./edgequake/crates/edgequake-api/src/handlers/chat.rs#L900)

## Compatibility

✅ **Backward Compatible**:

- Query endpoint behavior unchanged
- Chat endpoint now works correctly (previously broken)
- No breaking changes to API contracts
- Graceful fallback when workspace config unavailable

## Performance Impact

**Minimal** - Same number of operations, just applied in correct order:

1. Get workspace config (cached in workspace service)
2. Compute embedding with workspace provider (same as before, just different provider)
3. Retrieve vectors with workspace storage (same as before, just different storage)
4. Generate answer with optional LLM (same as before)

## Future Improvements

1. Consider caching workspace provider instances
2. Add metrics for workspace embedding dimension mismatches
3. Add health check for provider dimension compatibility
4. Consider async provider initialization

## Files Modified

- `edgequake/crates/edgequake-query/src/sota_engine.rs` (+400 lines)
- `edgequake/crates/edgequake-api/src/handlers/query.rs` (+2 lines for visibility)
- `edgequake/crates/edgequake-api/src/handlers/chat.rs` (+120 lines)

## Total Changes Summary

- ✅ 2 helper functions made public
- ✅ 2 new query methods added (non-streaming + streaming)
- ✅ 2 handlers updated (non-streaming + streaming)
- ✅ Full dimension isolation from input to storage to generation
- ✅ Graceful fallback behavior for edge cases
- ✅ Comprehensive debug/warn logging for troubleshooting
