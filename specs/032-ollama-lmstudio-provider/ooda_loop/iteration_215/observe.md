# OODA Iteration 215 - Observe

## Focus: Query-Time Workspace Provider Verification

### Investigation Summary

Reviewed [`query.rs`](../../../../edgequake/crates/edgequake-api/src/handlers/query.rs) to understand how workspace-specific providers are used at query time.

### Key Observations

1. **Workspace Embedding Provider Selection** (lines 166-200):
   ```rust
   let result = if let Some(ref workspace_id) = tenant_ctx.workspace_id {
       let embedding_result = get_workspace_embedding_provider(&state, workspace_id).await;
       let vector_result = get_workspace_vector_storage(&state, workspace_id).await;
       
       match (embedding_result, vector_result) {
           (Ok(Some(embedding_provider)), Ok(Some(vector_storage))) => {
               // Full workspace isolation
               state.sota_engine.query_with_workspace_config(
                   engine_request, embedding_provider, vector_storage
               ).await
           }
           // ... fallback cases
       }
   }
   ```

2. **`get_workspace_embedding_provider()`** (lines 466-518):
   - Parses workspace_id to UUID
   - Retrieves workspace from service
   - Checks if workspace has custom embedding config
   - Creates workspace-specific provider via `ProviderFactory::create_embedding_provider()`
   - Returns `Ok(Some(provider))` if custom, `Ok(None)` if default

3. **LLM Provider for Query Response** (lines 602+):
   - `get_workspace_llm_info()` provides LLM metadata for response
   - Used for lineage tracking in query responses

### Current Test Gap

The existing `e2e_query.rs` tests do NOT verify:
- ❌ Query uses workspace-specific embedding provider
- ❌ Query response contains correct provider metadata
- ❌ Workspace isolation of query results

### Recommendation

Create tests that:
1. Create workspace with specific embedding config
2. Execute query with workspace context (X-Workspace-Id header)
3. Verify query response contains workspace provider metadata
4. Verify workspace isolation (different workspaces get different providers)
