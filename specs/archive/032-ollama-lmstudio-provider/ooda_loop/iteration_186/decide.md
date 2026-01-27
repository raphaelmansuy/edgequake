# OODA 186: Decide - Fix Strategy for Silent Provider Fallback

**Date**: 2025-01-15
**Focus**: Design comprehensive fix for provider switching during document ingestion

## Problem Statement

When a workspace is configured with OpenAI but OPENAI_API_KEY is not set:

1. `ProviderFactory::create_llm_provider("openai", "gpt-4o-mini")` returns `Err`
2. `processor.rs` catches error and silently falls back to default pipeline
3. User's documents are processed with wrong provider (Ollama/Mock instead of OpenAI)

## Decision: Multi-Layered Fix

### Layer 1: Add Explicit Provider Verification

Add a method to verify provider configuration at workspace creation time:

- Check if required credentials exist
- Warn user if provider won't work
- Store verification status in workspace metadata

### Layer 2: Fix processor.rs to Propagate Errors

Instead of:

```rust
if let (Ok(llm), Ok(embedding)) = (llm_provider, embedding_provider) {
    // Use workspace pipeline
} else {
    warn!("Failed, using default");
    return Arc::clone(&self.pipeline);  // SILENT FALLBACK
}
```

Change to:

```rust
let llm = llm_provider.map_err(|e| {
    error!("Cannot use workspace LLM provider: {}", e);
    e
})?;
// ... or return TaskError if provider unavailable
```

### Layer 3: Add Provider Lineage Tracking

Store which provider was actually used for each extraction:

- Add `extractor_provider` field to document metadata
- Add `extractor_model` field to document metadata
- Display in UI for verification

### Layer 4: Comprehensive E2E Tests

Already implemented in OODA 187:

- `test_workspace_pipeline_uses_configured_mock_provider`
- `test_workspace_openai_without_api_key_behavior`
- `test_workspace_pipeline_handles_invalid_provider`
- `test_multiple_workspaces_provider_isolation`

## Implementation Plan

1. **processor.rs**: Add `fail_fast: bool` option to get_workspace_pipeline
2. **processor.rs**: Log ERROR (not WARN) when provider creation fails
3. **processor.rs**: Consider returning TaskError for unrecoverable provider failures
4. **Document lineage**: Store provider used in extraction results
5. **state.rs**: Ensure consistency with processor.rs behavior

## Risk Assessment

| Risk                       | Mitigation                             |
| -------------------------- | -------------------------------------- |
| Breaking existing behavior | Add feature flag for strict mode       |
| Test failures              | Update tests to expect new behavior    |
| User disruption            | Clear error messages with action steps |

## Next Step

OODA 189: Implement the fix in processor.rs with explicit error handling.
