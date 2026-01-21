# OODA 184-185: Orient - Critical Bug Analysis

**Date**: 2025-01-15
**Focus**: Root cause analysis of silent provider fallback bug

## Critical Bug Found 🚨

### Location

[processor.rs#L170-192](../../edgequake/crates/edgequake-api/src/processor.rs#L170)

### Code Path

```rust
async fn get_workspace_pipeline(&self, workspace_id: Option<&str>) -> Arc<Pipeline> {
    // ... workspace lookup ...

    match workspace_service.get_workspace(workspace_uuid).await {
        Ok(Some(ws)) => {
            // Try to create workspace-specific LLM provider
            let llm_provider =
                ProviderFactory::create_llm_provider(&ws.llm_provider, &ws.llm_model);

            // Try to create workspace-specific embedding provider
            let embedding_provider = ProviderFactory::create_embedding_provider(...);

            // If both providers were created successfully, build workspace pipeline
            if let (Ok(llm), Ok(embedding)) = (llm_provider, embedding_provider) {
                // SUCCESS: Use workspace providers
                return Arc::new(Pipeline::default_pipeline()
                    .with_extractor(extractor)
                    .with_embedding_provider(embedding));
            }

            warn!("Failed to create workspace-specific providers, using default pipeline");
        }
        // ... error cases also fall back to default ...
    }

    Arc::clone(&self.pipeline)  // <-- RETURNS DEFAULT PIPELINE!
}
```

### Bug Behavior

| Scenario                                             | Expected                | Actual                                              |
| ---------------------------------------------------- | ----------------------- | --------------------------------------------------- |
| Workspace configured with OpenAI, API key missing    | ERROR - fail processing | ⚠️ SILENT fallback to default (Ollama/Mock)         |
| Workspace configured with Ollama, server unreachable | ERROR - fail processing | ⚠️ Provider created anyway (fails later at runtime) |
| Provider name typo (e.g., "opeanai")                 | ERROR - fail processing | ⚠️ SILENT fallback to default                       |

### Root Cause

The pattern `if let (Ok(llm), Ok(embedding)) = (...)` silently ignores errors and falls back to the default pipeline. This violates:

1. **User expectations**: User thinks they're using OpenAI, actually using Ollama
2. **Data integrity**: Extracted entities/embeddings use wrong model
3. **Cost tracking**: Billing assumes OpenAI usage but uses free Ollama
4. **Reproducibility**: Results differ from expected model characteristics

### Impact Assessment

| Impact                | Severity                                |
| --------------------- | --------------------------------------- |
| Data quality          | HIGH - wrong extraction model           |
| Embedding consistency | CRITICAL - dimension mismatch possible  |
| Cost tracking         | MEDIUM - billing discrepancy            |
| User trust            | HIGH - silent failures erode confidence |

## Required Fixes

1. **processor.rs**: Return error instead of silent fallback
2. **state.rs**: Already has better handling but needs consistency
3. **Add lineage**: Track which provider was actually used
4. **Add E2E tests**: Verify provider is actually used

## Decision

Proceed to OODA 186-189 to implement fixes and tests.
