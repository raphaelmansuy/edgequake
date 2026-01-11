# OODA Iteration 08: Decide

**Date:** 2026-01-11  
**Focus:** Implementation Plan for Workspace LLM Configuration

## Decision Summary

Add `llm_model` and `llm_provider` fields to Workspace, with helper methods for `provider/model` combined format.

## Implementation Checklist

### Phase 1: Core Types (OODA 08-10)

- [ ] Add `llm_model: String` to `Workspace` struct
- [ ] Add `llm_provider: String` to `Workspace` struct
- [ ] Add `llm_full_id()` helper method
- [ ] Add `embedding_full_id()` helper method
- [ ] Add `parse_model_id()` static method
- [ ] Add `DEFAULT_LLM_MODEL` and `DEFAULT_LLM_PROVIDER` constants
- [ ] Update `Workspace::new()` to set LLM defaults
- [ ] Update `Workspace::default_embedding_config()` → `default_config()` for both

### Phase 2: API DTOs (OODA 11-15)

- [ ] Add `llm_model: Option<String>` to `CreateWorkspaceApiRequest`
- [ ] Add `llm_provider: Option<String>` to `CreateWorkspaceApiRequest`
- [ ] Add `llm_model: String` to `WorkspaceResponse`
- [ ] Add `llm_provider: String` to `WorkspaceResponse`
- [ ] Add `llm_model: Option<String>` to `UpdateWorkspaceApiRequest` (for provider migration)
- [ ] Add `llm_provider: Option<String>` to `UpdateWorkspaceApiRequest`

### Phase 3: Handlers (OODA 16-20)

- [ ] Update `create_workspace` handler to use LLM config from request
- [ ] Update `get_workspace` handler to return LLM config
- [ ] Update `update_workspace` handler to allow LLM config changes
- [ ] Update `list_workspaces` handler (already returns WorkspaceResponse)

### Phase 4: Workspace Service (OODA 21-25)

- [ ] Update `WorkspaceService::create_workspace` to accept LLM config
- [ ] Update storage layer to persist LLM config (settings JSONB)
- [ ] Update storage layer to read LLM config

### Phase 5: Tests (OODA 26-30)

- [ ] Add unit tests for `llm_full_id()` and `parse_model_id()`
- [ ] Update e2e workspace tests to include LLM config
- [ ] Test provider auto-detection for LLM models

### Phase 6: WebUI (OODA 31-40)

- [ ] Create `LLMModelSelector` component (parallel to `EmbeddingModelSelector`)
- [ ] Add LLM selector to workspace creation dialog
- [ ] Update workspace details to show LLM configuration
- [ ] Update TypeScript types for workspace with LLM fields

### Phase 7: Documentation (OODA 41-50)

- [ ] Update API docs with new request/response fields
- [ ] Update PROVIDER_SETUP_GUIDE.md with LLM configuration
- [ ] Add examples of workspace creation with LLM config

## Code Changes Summary

### File: `edgequake-core/src/types/multitenancy.rs`

```diff
+ pub const DEFAULT_LLM_MODEL: &str = "gemma3:12b";
+ pub const DEFAULT_LLM_PROVIDER: &str = "ollama";

pub struct Workspace {
    // ... existing fields ...
    
+   /// LLM model name (e.g., "gemma3:12b", "gpt-4o-mini").
+   pub llm_model: String,
+   
+   /// LLM provider (e.g., "ollama", "openai", "lmstudio").
+   pub llm_provider: String,
}

impl Workspace {
+   /// Get fully qualified LLM model ID.
+   pub fn llm_full_id(&self) -> String {
+       format!("{}/{}", self.llm_provider, self.llm_model)
+   }
+   
+   /// Get fully qualified embedding model ID.
+   pub fn embedding_full_id(&self) -> String {
+       format!("{}/{}", self.embedding_provider, self.embedding_model)
+   }
}
```

### File: `edgequake-api/src/handlers/workspaces_types.rs`

```diff
pub struct CreateWorkspaceApiRequest {
    // ... existing embedding fields ...
    
+   /// LLM model for knowledge graph generation/summarization.
+   #[serde(skip_serializing_if = "Option::is_none")]
+   pub llm_model: Option<String>,
+   
+   /// LLM provider (auto-detected if not provided).
+   #[serde(skip_serializing_if = "Option::is_none")]
+   pub llm_provider: Option<String>,
}

pub struct WorkspaceResponse {
    // ... existing embedding fields ...
    
+   /// LLM model for this workspace.
+   pub llm_model: String,
+   
+   /// LLM provider.
+   pub llm_provider: String,
}
```

## Risk Mitigation

1. **Backward Compatibility**: Default values ensure existing workspaces work
2. **Database Migration**: Use settings JSONB, no schema change needed
3. **API Compatibility**: New fields are optional in request, present in response
