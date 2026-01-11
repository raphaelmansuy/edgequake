# OODA Iteration 09 - Act Phase

## Objective

Implement workspace-level LLM provider configuration to support knowledge graph generation, document ingestion, and summarization. The LLM provider for ingestion is separate from the query-time LLM that users select in the UI.

## Model ID Format

Models are identified by `provider/model_name` format:
- `"ollama/gemma3:12b"` - Ollama with Gemma 3 12B
- `"openai/gpt-4o-mini"` - OpenAI GPT-4o Mini
- `"lmstudio/gemma-3n-e4b-it"` - LM Studio local model

## Changes Implemented

### 1. Core Domain (multitenancy.rs)

**Added LLM fields to `Workspace` struct:**
```rust
pub struct Workspace {
    // ... existing fields ...
    pub llm_model: String,      // e.g., "gemma3:12b"
    pub llm_provider: String,   // e.g., "ollama"
    // ... embedding fields ...
}
```

**Added LLM constants:**
```rust
pub const DEFAULT_LLM_MODEL: &str = "gemma3:12b";
pub const DEFAULT_LLM_PROVIDER: &str = "ollama";
```

**Added helper methods:**
- `llm_full_id()` → Returns "provider/model" format (e.g., "ollama/gemma3:12b")
- `embedding_full_id()` → Returns "provider/model" format
- `parse_model_id(&str)` → Parses "provider/model" into tuple
- `default_llm_config()` → Returns default LLM config from environment
- `with_llm_model()`, `with_llm_provider()`, `with_llm_config()` → Builder methods

**Added LLM fields to `CreateWorkspaceRequest`:**
```rust
pub struct CreateWorkspaceRequest {
    // ... existing fields ...
    pub llm_model: Option<String>,
    pub llm_provider: Option<String>,
    // ... embedding fields ...
}
```

### 2. API Layer (workspaces_types.rs, workspaces.rs)

**Updated API DTOs:**
- `CreateWorkspaceApiRequest`: Added `llm_model`, `llm_provider`
- `UpdateWorkspaceApiRequest`: Added `llm_model`, `llm_provider`
- `WorkspaceResponse`: Added `llm_model`, `llm_provider`, `llm_full_id`, `embedding_full_id`

**Updated handler:**
- `create_workspace`: Now passes LLM config to service layer
- Logging includes `llm_full_id()` and `embedding_full_id()`

### 3. Service Layer (workspace_service.rs, workspace_service_impl.rs)

**InMemoryWorkspaceService:**
- Added LLM config handling in `create_workspace`

**PostgresWorkspaceServiceImpl:**
- Added LLM config handling in `create_workspace`
- Store LLM config in metadata JSONB: `llm_model`, `llm_provider`
- Updated `WorkspaceRow::into_workspace()` to extract LLM config from metadata

### 4. WebUI Types (types/index.ts)

**Updated TypeScript interfaces:**
```typescript
export interface Workspace {
  // ... existing fields ...
  llm_model?: string;
  llm_provider?: string;
  llm_full_id?: string;      // Combined format
  embedding_full_id?: string; // Combined format
}

export interface CreateWorkspaceRequest {
  // ... existing fields ...
  llm_model?: string;
  llm_provider?: string;
}
```

### 5. Constants Re-export (types/mod.rs)

Added LLM constants to public exports:
```rust
pub use multitenancy::{
    // ... existing exports ...
    DEFAULT_LLM_MODEL,
    DEFAULT_LLM_PROVIDER,
    // ... embedding exports ...
};
```

### 6. Test Updates

Fixed all test files to include new LLM fields:
- `workspace_service.rs` (3 locations)
- `e2e_workspace_service.rs` (9 locations)
- `e2e_provider_switching.rs` (7 locations)
- `workspaces_types.rs` (3 test functions)

## Verification

**Build Check:**
```bash
cargo check --workspace  # ✅ Passed
```

**Tests:**
```bash
cargo test --workspace   # ✅ 2400+ tests passed
```

## API Example

**Create Workspace with LLM Config:**
```json
POST /api/v1/tenants/{tenant_id}/workspaces
{
  "name": "Research Project",
  "llm_model": "gemma3:12b",
  "llm_provider": "ollama",
  "embedding_model": "embeddinggemma:latest"
}
```

**Response:**
```json
{
  "id": "...",
  "name": "Research Project",
  "llm_model": "gemma3:12b",
  "llm_provider": "ollama",
  "llm_full_id": "ollama/gemma3:12b",
  "embedding_model": "embeddinggemma:latest",
  "embedding_provider": "ollama",
  "embedding_dimension": 768,
  "embedding_full_id": "ollama/embeddinggemma:latest"
}
```

## Next Steps

1. **WebUI Enhancement**: Add LLM model selector to workspace creation dialog (parallel to EmbeddingModelSelector)
2. **Ingestion Integration**: Use workspace LLM config for entity extraction, summarization
3. **Database Migration**: Add dedicated columns for LLM config (currently in metadata JSONB)
4. **Environment Variables**: Document `EDGEQUAKE_DEFAULT_LLM_MODEL`, `EDGEQUAKE_DEFAULT_LLM_PROVIDER`
