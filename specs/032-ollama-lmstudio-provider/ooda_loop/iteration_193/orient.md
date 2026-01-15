# OODA 193: Orient - PostgreSQL Provider Persistence Analysis

**Date**: 2025-01-15
**Focus**: Understanding PostgreSQL-specific provider config persistence

## Key Findings

### Storage Pattern

Provider configuration is stored in the `metadata` JSONB column of `workspaces` table:

- `llm_model`: string (default: "gemma3:12b")
- `llm_provider`: string (default: "ollama")
- `embedding_model`: string (default: "embeddinggemma")
- `embedding_provider`: string (default: "ollama")
- `embedding_dimension`: integer (default: 768)

### Read Path

[workspace_service_impl.rs#L926-L980](../../../../edgequake/crates/edgequake-core/src/workspace_service_impl.rs#L926)

```rust
fn into_workspace(self) -> Workspace {
    let llm_model = metadata.get("llm_model")
        .and_then(|v| v.as_str())
        .unwrap_or(DEFAULT_LLM_MODEL)
        .to_string();
    // ... similar for other fields
}
```

### Write Path

[workspace_service_impl.rs#L526-L560](../../../../edgequake/crates/edgequake-core/src/workspace_service_impl.rs#L526)

```rust
// SPEC-032: LLM model configuration updates
if let Some(llm_model) = request.llm_model {
    workspace.llm_model = llm_model.clone();
    workspace.metadata.insert("llm_model".to_string(), serde_json::json!(llm_model));
}
```

## PostgreSQL-Specific Concerns

1. **JSONB Extraction**: Provider config must be correctly serialized/deserialized
2. **Default Fallbacks**: When metadata is empty, defaults are used
3. **Atomicity**: Updates must be atomic for all provider fields
4. **No Caching**: Each get_workspace() queries DB fresh (no stale config issue)

## Test Scenarios Required

1. **Persistence Test**: Create workspace → close connection → reopen → verify provider config
2. **Update Test**: Update provider via API → verify get_workspace returns new config
3. **Rebuild Flow**: Update provider → trigger rebuild → verify new provider is used
4. **Edge Cases**:
   - Empty metadata (legacy workspaces)
   - Invalid provider name in metadata
   - Missing dimension field

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                         API Handler                                  │
│   POST /api/workspaces/:id/update                                   │
│   { "llm_provider": "openai", "llm_model": "gpt-4o-mini" }          │
└──────────────────────────────┬──────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│                   WorkspaceService.update_workspace()               │
│   1. Get existing workspace                                          │
│   2. Update workspace.llm_provider = "openai"                        │
│   3. Update workspace.metadata["llm_provider"] = "openai"           │
│   4. Persist to PostgreSQL                                           │
└──────────────────────────────┬──────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│                       PostgreSQL                                     │
│   workspaces.metadata JSONB:                                         │
│   {                                                                  │
│     "llm_provider": "openai",                                        │
│     "llm_model": "gpt-4o-mini",                                      │
│     "embedding_provider": "openai",                                  │
│     "embedding_model": "text-embedding-3-small"                      │
│   }                                                                  │
└──────────────────────────────┬──────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│                  DocumentTaskProcessor.get_workspace_pipeline()      │
│   1. Get workspace from DB (fresh read)                              │
│   2. Extract provider config from workspace                          │
│   3. ProviderFactory::create_safe_llm_provider("openai", "gpt-4o")  │
│   4. Return pipeline with workspace-specific provider                │
└─────────────────────────────────────────────────────────────────────┘
```

## Next Step

OODA 194: Decide - Design specific PostgreSQL E2E test scenarios
