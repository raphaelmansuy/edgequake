# OODA Iteration 219 - Observe

## Focus: Rebuild Endpoint with Provider Switching

### Objective

Analyze the rebuild endpoint to verify that when embedding/LLM provider is switched, the rebuild operation uses the new provider.

### Key Question from User

> "When I change to an OpenAI provider for embedding and LLM extraction, is the extraction really done with the OpenAI provider?"

### Rebuild Flow Analysis

From [rebuild.rs](../../../../edgequake/crates/edgequake-api/src/handlers/rebuild.rs):

1. **Rebuild request** received with workspace_id
2. **Get workspace config** to retrieve current provider settings
3. **Create workspace-specific providers** using ProviderFactory
4. **Re-extract entities** using workspace LLM provider
5. **Re-embed chunks** using workspace embedding provider
6. **Store lineage** in ProcessingStats

### Critical Code Path

```rust
// In processor.rs
pub async fn rebuild_workspace_embeddings(
    state: &AppState,
    workspace_id: &str,
) -> Result<RebuildStats> {
    // Get workspace config (includes provider settings)
    let workspace = state.workspace_service.get_workspace(uuid).await?;

    // Create workspace-specific embedding provider
    let embedding_provider = ProviderFactory::create_embedding_provider(
        &workspace.embedding_provider,   // "openai" or "ollama"
        &workspace.embedding_model,      // model name
        workspace.embedding_dimension,   // 1536 or 768
    )?;

    // Re-embed all chunks with new provider
    for chunk in workspace_chunks {
        let new_embedding = embedding_provider.embed(&chunk.content).await?;
        update_chunk_embedding(chunk.id, new_embedding).await?;
    }
}
```

### Verification Points

1. **Provider switch is persisted** - workspace.update_embedding_config() saves new config
2. **Rebuild reads updated config** - get_workspace() returns new provider settings
3. **Provider is created from config** - ProviderFactory uses workspace.embedding_provider
4. **Lineage is recorded** - ProcessingStats includes provider info

### Test Strategy

Create tests that:

1. Create workspace with provider A
2. Upload document (uses provider A)
3. Switch to provider B
4. Trigger rebuild
5. Verify rebuild uses provider B (via logs/stats/lineage)

### Next Steps

Create E2E tests for rebuild endpoint with provider switching.
