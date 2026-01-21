# OODA-263: Pipeline Creation Duplication Audit

**Date**: 2026-01-17
**Status**: ✅ ANALYSIS COMPLETE (Deferred Consolidation)

## Problem Statement

Both `processor.rs` and `state.rs` contain very similar pipeline creation logic:

1. Parse workspace ID to UUID
2. Lookup workspace from WorkspaceService
3. Create LLM provider via ProviderFactory::create_safe_llm_provider
4. Create embedding provider via ProviderFactory::create_safe_embedding_provider
5. Build Pipeline with extractor and embedding
6. Fallback to global pipeline on failure

## Files Analyzed

### processor.rs (lines 200-280)

- `get_pipeline_for_workspace_async()` method
- Used by: DocumentTaskProcessor for background document processing
- Context: Async task worker

### state.rs (lines 970-1050)

- `get_pipeline_for_workspace()` method (similar name, different impl)
- Used by: AppState for request handling
- Context: Request handler

## Duplication Impact

| Metric             | Value                        |
| ------------------ | ---------------------------- |
| Duplicated Lines   | ~70 lines                    |
| Functions Affected | 2                            |
| Risk Level         | Medium (code drift possible) |

## Recommended Consolidation

Create a `PipelineFactory` struct in a new module:

```rust
// providers/pipeline_factory.rs
pub struct PipelineFactory {
    workspace_service: Arc<WorkspaceService>,
    global_pipeline: Arc<Pipeline>,
}

impl PipelineFactory {
    pub async fn create_for_workspace(
        &self,
        workspace_id: Option<&str>,
    ) -> Arc<Pipeline> {
        // Consolidated logic here
    }
}
```

## Decision: Deferred

**Reason**: The current duplication is acceptable because:

1. Both contexts have slightly different error handling needs
2. processor.rs runs in background tasks (different tracing requirements)
3. state.rs runs in request handlers (different logging patterns)
4. Changes are infrequent (providers don't change often)

**When to consolidate**:

- If a third location needs this pattern
- If provider creation logic changes
- If bugs appear in one but not the other

## Task Logs

- **Actions**: Analyzed processor.rs and state.rs, documented duplication patterns
- **Decisions**: Deferred consolidation due to acceptable technical debt
- **Next steps**: OODA-264: Create E2E test for provider switching
- **Lessons**: Not all duplication requires immediate consolidation; context matters
