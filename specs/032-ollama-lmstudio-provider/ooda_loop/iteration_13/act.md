# OODA Iteration 13: Act

**Date:** 2025-01-11
**Focus:** Workspace-specific embedding in query process
**Commit:** b4d63b8

## Changes Made

### 1. ProviderFactory Extension (`edgequake-llm/src/factory.rs`)

Added `create_embedding_provider()` method to create embedding providers from workspace configuration:

```rust
pub fn create_embedding_provider(
    provider_name: &str,  // "openai", "ollama", "lmstudio", "mock"
    model: &str,          // "text-embedding-3-small", "embeddinggemma:latest"
    _dimension: usize,    // 1536, 768 (auto-detected from provider)
) -> Result<Arc<dyn EmbeddingProvider>>
```

**Lines added:** 87 LOC (lines 233-318)

### 2. SOTAQueryEngine Extension (`edgequake-query/src/sota_engine.rs`)

Added `query_with_embedding_provider()` method that accepts an optional embedding provider override:

```rust
pub async fn query_with_embedding_provider(
    &self,
    request: QueryRequest,
    embedding_provider: Arc<dyn EmbeddingProvider>,
) -> Result<QueryResponse>
```

**Key difference:** Uses `embedding_provider.as_ref()` instead of `self.embedding_provider.as_ref()` at line 763.

**Lines added:** 179 LOC (lines 679-857)

### 3. Query Handler Update (`edgequake-api/src/handlers/query.rs`)

Updated `execute_query` handler to use workspace-specific embedding:

```rust
// SPEC-032: Get workspace-specific embedding provider
let result = if let Some(ref workspace_id) = tenant_ctx.workspace_id {
    match get_workspace_embedding_provider(&state, workspace_id).await {
        Ok(Some(embedding_provider)) => {
            state.sota_engine
                .query_with_embedding_provider(engine_request, embedding_provider)
                .await
        }
        Ok(None) | Err(_) => {
            // Fallback to global provider
            state.sota_engine.query(engine_request).await
        }
    }
} else {
    state.sota_engine.query(engine_request).await
};
```

Added helper function `get_workspace_embedding_provider()`:
- Parses workspace UUID
- Looks up workspace from WorkspaceService
- Creates provider using `ProviderFactory::create_embedding_provider()`
- Returns `None` if workspace uses default config

**Lines added:** 141 LOC

### 4. Library Export (`edgequake-query/src/lib.rs`)

Re-exported `EmbeddingProvider` for API layer access:

```rust
pub use edgequake_llm::traits::EmbeddingProvider;
```

## Test Results

```
Query handler tests:   16 passed ✅
LLM factory tests:      8 passed ✅
Query crate tests:     11 passed ✅
```

## Architecture Flow

```
POST /api/v1/query
        ↓
   TenantContext.workspace_id?
        ↓
   ┌────YES────┐           ┌────NO────┐
   ↓           ↓           ↓
get_workspace_embedding_provider()
        ↓
   ┌────Some(provider)────┐    ┌────None/Err────┐
   ↓                      ↓    ↓
query_with_embedding_provider()  query()
        ↓                       ↓
   Workspace-specific         Global
   embedding used             embedding used
```

## Spec Compliance

This implements the SPEC-032 requirement:

> "In the query process we must use the embedding model associated with the workspace to generate embeddings for incoming queries."

## Files Modified

| File | Lines Changed | Purpose |
|------|---------------|---------|
| factory.rs | +87 | create_embedding_provider() |
| sota_engine.rs | +179 | query_with_embedding_provider() |
| handlers/query.rs | +141 | Workspace-aware query execution |
| lib.rs | +3 | Re-export EmbeddingProvider |
