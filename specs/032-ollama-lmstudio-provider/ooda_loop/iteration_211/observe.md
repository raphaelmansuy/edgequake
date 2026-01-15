# OODA Iteration 211 - Observe

## Focus: Deep Verification of Provider Switching in Document Processing

### Investigation Summary

Reviewed [`processor.rs`](../../../../edgequake/crates/edgequake-api/src/processor.rs) lines 1-600 to understand how workspace-specific providers are used.

### Key Observations

1. **Processor Architecture** (lines 50-100):

   - `DocumentTaskProcessor` has `workspace_service` and `models_config` fields
   - `with_workspace_support()` constructor enables workspace-specific processing

2. **Pipeline Selection Flow** (lines 136-285):

   ```
   get_workspace_pipeline(workspace_id)
   ├── Lookup workspace via workspace_service
   ├── Create LLM provider: ProviderFactory::create_safe_llm_provider(&ws.llm_provider, &ws.llm_model)
   ├── Create Embedding provider: ProviderFactory::create_safe_embedding_provider(...)
   ├── Create new Pipeline with workspace-specific providers
   └── Return Arc<Pipeline>
   ```

3. **Provider Lineage Tracking** (lines 378-425):

   - `get_workspace_provider_lineage()` captures which providers SHOULD be used
   - Returns `ProviderLineage` struct with extraction_provider, embedding_provider, etc.

4. **Processing Flow** (lines 427-550):

   ```rust
   // SPEC-032: Extract workspace_id
   let workspace_id = data.workspace_id or data.metadata["workspace_id"]

   // Get workspace-specific pipeline
   let pipeline = self.get_workspace_pipeline(workspace_id).await;

   // Capture lineage for tracking
   let provider_lineage = self.get_workspace_provider_lineage(workspace_id).await;

   // Log which providers are used
   info!(
       extraction_provider = %provider_lineage.extraction_provider,
       extraction_model = %provider_lineage.extraction_model,
       embedding_provider = %provider_lineage.embedding_provider,
       "Processing document with workspace-specific pipeline"
   );

   // Process through pipeline
   let result = pipeline.process(&document_id, &data.text).await;
   ```

### Critical Finding: Provider Actually Used?

The code **logs** which provider configuration is expected, but:

- The actual provider call happens inside `pipeline.process()`
- There's no explicit verification that the provider was invoked
- For tests with mock providers, we can't directly observe which provider was called

### Gap Identified

Current E2E tests verify:

- ✅ Configuration is updated in workspace
- ✅ Response contains correct provider info
- ✅ `get_workspace_pipeline()` is called

But do NOT verify:

- ❌ The actual LLM/embedding provider was invoked with the request
- ❌ Provider-specific side effects occurred (API calls, token counting, etc.)

### Next Steps (Orient)

Need to investigate:

1. How `ProviderFactory::create_safe_llm_provider()` works
2. If mock providers track invocation counts
3. If we can add test hooks to verify actual provider invocation
