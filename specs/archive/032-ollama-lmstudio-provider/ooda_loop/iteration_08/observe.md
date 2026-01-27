# OODA Iteration 08: Observe

**Date:** 2026-01-11  
**Focus:** Workspace LLM Provider Selection & Provider/Model Naming Convention

## Observation Summary

### Mission Requirements (SPEC-032)

From [specs/032-ollama-lmstudio-provider.md](../../032-ollama-lmstudio-provider.md):

1. **VERY IMPORTANT**: Workspace must have default LLM provider for:

   - Knowledge graph generation
   - Document ingestion
   - Summarization
   - LLM provider CAN be different from query-time provider

2. **VERY IMPORTANT**: Model identifier = `provider/model_name`
   - Examples: `ollama/gemma3:12b`, `openai/gpt-4o`, `lmstudio/gemma-3n-e4b-it-mlxmodel`
   - Must be consistent across: config file, API, database, UI

### Current State Analysis

#### 1. Workspace Domain Model ([multitenancy.rs](../../../../../../edgequake/crates/edgequake-core/src/types/multitenancy.rs#L159))

```rust
pub struct Workspace {
    // ... other fields ...

    // Embedding Configuration ✅
    pub embedding_model: String,
    pub embedding_provider: String,
    pub embedding_dimension: usize,

    // LLM Configuration ❌ MISSING
    // pub llm_model: String,      // NOT PRESENT
    // pub llm_provider: String,   // NOT PRESENT
}
```

**Gap:** No LLM provider/model fields in Workspace struct.

#### 2. API DTOs ([workspaces_types.rs](../../../../../../edgequake/crates/edgequake-api/src/handlers/workspaces_types.rs#L55))

```rust
pub struct CreateWorkspaceApiRequest {
    // Embedding config ✅
    pub embedding_model: Option<String>,
    pub embedding_provider: Option<String>,
    pub embedding_dimension: Option<usize>,

    // LLM config ❌ MISSING
}

pub struct WorkspaceResponse {
    // Embedding config ✅
    pub embedding_model: String,
    pub embedding_provider: String,
    pub embedding_dimension: usize,

    // LLM config ❌ MISSING
}
```

**Gap:** API endpoints don't support LLM configuration for workspaces.

#### 3. Database Schema (PostgreSQL)

From `docker-compose exec postgres psql`:

```sql
\d workspaces
-- Column: settings JSONB DEFAULT '{}'
-- NO explicit llm_model, llm_provider columns
```

**Gap:** LLM configuration not stored in database.

#### 4. Model Identifier Format

| Location             | Current Format                                                   | Required Format                |
| -------------------- | ---------------------------------------------------------------- | ------------------------------ |
| models.toml defaults | `llm_provider = "ollama"`, `llm_model = "gemma3:12b"` (separate) | `ollama/gemma3:12b` (combined) |
| Workspace struct     | `embedding_provider`, `embedding_model` (separate)               | Should support both            |
| UI selector          | `provider:model`                                                 | `provider/model`               |
| API paths            | `/models/{provider}/{model}`                                     | Correct                        |

#### 5. What's Working ✅

- `models.toml` has `[defaults]` section with `llm_provider` and `llm_model`
- API has `/api/v1/models/{provider}/{model}` endpoints
- `ModelConfig` struct has `defaults.llm_provider` and `defaults.llm_model`
- UI components exist for model selection

### Files to Modify

1. **Core Types**:

   - [edgequake-core/src/types/multitenancy.rs](../../../../../../edgequake/crates/edgequake-core/src/types/multitenancy.rs) - Add `llm_model`, `llm_provider`

2. **API DTOs**:

   - [edgequake-api/src/handlers/workspaces_types.rs](../../../../../../edgequake/crates/edgequake-api/src/handlers/workspaces_types.rs) - Add LLM fields

3. **API Handlers**:

   - [edgequake-api/src/handlers/workspaces.rs](../../../../../../edgequake/crates/edgequake-api/src/handlers/workspaces.rs) - Map LLM fields

4. **Database Migrations**:

   - Consider adding explicit columns or using settings JSONB

5. **WebUI Components**:
   - [embedding-model-selector.tsx](../../../../../../edgequake_webui/src/components/workspace/embedding-model-selector.tsx)
   - Need parallel `LLMModelSelector` component

### Impact Assessment

| Component        | Risk   | Effort                            |
| ---------------- | ------ | --------------------------------- |
| Workspace struct | LOW    | Add 2 fields                      |
| API DTOs         | LOW    | Add 4 fields (request + response) |
| API handlers     | MEDIUM | Update create/update/get handlers |
| Database         | MEDIUM | Either JSONB or migration         |
| WebUI            | MEDIUM | Need new selector component       |

## Next Steps (Orient → Decide → Act)

1. Add `llm_model` and `llm_provider` to `Workspace` struct
2. Add helper function for combined format: `fn full_model_id(&self) -> String`
3. Update API DTOs with LLM configuration fields
4. Update workspace handlers to read/write LLM config
5. Update WebUI to display/select workspace LLM provider
