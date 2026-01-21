# OODA 221: OBSERVE - Workspace Pipeline Provider Integration

## Objective

Test the integration between workspace configuration and pipeline creation via `create_workspace_pipeline()` function in AppState.

## Current Flow Analysis

### Code Path: state.rs:933-1000

```rust
pub async fn create_workspace_pipeline(&self, workspace_id: &str) -> Arc<Pipeline> {
    // 1. Parse workspace_id to UUID
    let workspace_uuid = uuid::Uuid::parse_str(workspace_id)?;

    // 2. Lookup workspace configuration
    let workspace_result = self.workspace_service.get_workspace(workspace_uuid).await;

    match workspace_result {
        Ok(Some(ws)) => {
            // 3. Create workspace-specific LLM provider
            let llm_provider = ProviderFactory::create_safe_llm_provider(
                &ws.llm_provider,
                &ws.llm_model
            );

            // 4. Create workspace-specific embedding provider
            let embedding_provider = ProviderFactory::create_safe_embedding_provider(
                &ws.embedding_provider,
                &ws.embedding_model,
                ws.embedding_dimension,
            );

            // 5. If both succeed, return workspace pipeline
            if let (Ok(llm), Ok(embedding)) = (llm_provider, embedding_provider) {
                let extractor = Arc::new(LLMExtractor::new(llm));
                return Arc::new(
                    Pipeline::default_pipeline()
                        .with_extractor(extractor)
                        .with_embedding_provider(embedding),
                );
            }
        }
        _ => // Fall back to global pipeline
    }
}
```

## Key Integration Points

1. **WorkspaceService.get_workspace()** - Retrieves workspace config from storage
2. **ProviderFactory.create_safe_llm_provider()** - Creates LLM provider with safety limits
3. **ProviderFactory.create_safe_embedding_provider()** - Creates embedding provider
4. **Pipeline construction** - New pipeline with workspace-specific providers

## Test Scenarios Needed

### Scenario 1: Workspace with Ollama config

- Create workspace with ollama provider
- Call create_workspace_pipeline()
- Verify pipeline uses workspace config (not global)

### Scenario 2: Workspace with OpenAI config (requires API key)

- Create workspace with openai provider
- Without API key: should fall back to global pipeline
- With API key: should use workspace-specific OpenAI

### Scenario 3: Provider switch verification

- Create workspace with Ollama
- Verify pipeline uses Ollama
- Update workspace to OpenAI
- Verify pipeline uses new config

### Scenario 4: Invalid workspace ID

- Call create_workspace_pipeline with invalid UUID
- Should return global pipeline

### Scenario 5: Non-existent workspace

- Call create_workspace_pipeline with valid but non-existent UUID
- Should return global pipeline

## Files to Create

`edgequake/crates/edgequake-api/tests/e2e_workspace_pipeline_integration.rs`

## Dependencies

- edgequake-api with full test infrastructure
- Mock workspace service for isolation
- ProviderFactory (already tested in OODA 220)
