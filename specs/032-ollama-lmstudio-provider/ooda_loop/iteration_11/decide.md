# Iteration 11: Decide

**Date:** 2025-01-30
**Focus:** Ingestion Pipeline Workspace LLM Integration

## Decision

Implement dynamic workspace-specific pipeline creation for document ingestion.

## Implementation Plan

### 1. Add `create_llm_provider()` to ProviderFactory

**File:** `edgequake-llm/src/factory.rs`

```rust
pub fn create_llm_provider(
    provider_name: &str,
    model: &str,
) -> Result<Arc<dyn LLMProvider>>
```

### 2. Add `create_workspace_pipeline()` to AppState

**File:** `edgequake-api/src/state.rs`

```rust
pub async fn create_workspace_pipeline(&self, workspace_id: &str) -> Arc<Pipeline> {
    // 1. Parse workspace_id to UUID
    // 2. Lookup workspace from WorkspaceService
    // 3. Create workspace-specific LLM provider
    // 4. Create workspace-specific embedding provider
    // 5. Build and return Pipeline
    // 6. Fall back to global pipeline on any error
}
```

### 3. Update Document Upload Handler

**File:** `edgequake-api/src/handlers/documents.rs`

```rust
// Before:
let result = state.pipeline.process(&document_id, &request.content).await?;

// After:
let workspace_pipeline = state.create_workspace_pipeline(&workspace_id_for_storage).await;
let result = workspace_pipeline.process(&document_id, &request.content).await?;
```

## Acceptance Criteria

- [ ] `create_llm_provider()` added to ProviderFactory
- [ ] `create_workspace_pipeline()` added to AppState
- [ ] Document handler uses workspace-specific pipeline
- [ ] Graceful fallback to global pipeline on errors
- [ ] All tests pass (396 + 188 tests)
