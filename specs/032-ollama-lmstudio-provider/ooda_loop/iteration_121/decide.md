# OODA Iteration 121: Decide

## Date: 2026-01-14

## Decision 1: Sync Default Constants with models.toml

**Decision**: Update code constants to match models.toml defaults (Ollama for both LLM and embedding).

**Rationale**:

- Single source of truth principle
- models.toml is user-editable without recompilation
- Ollama is more accessible for development (no API key needed)

**Implementation**:

- Update `DEFAULT_EMBEDDING_MODEL` from "text-embedding-3-small" to "embeddinggemma"
- Update `DEFAULT_EMBEDDING_PROVIDER` from "openai" to "ollama"
- Update `DEFAULT_EMBEDDING_DIMENSION` from 1536 to 768

## Decision 2: Load Defaults from ModelsConfig

**Decision**: Inject ModelsConfig defaults into workspace/tenant creation.

**Approach**:

- Add a static method to read from ModelsConfig
- Use environment variables as override layer
- Maintain backward compatibility with env var configuration

**Implementation**:

- Add `Workspace::default_config_from_models(config: &ModelsConfig)` method
- Modify AppState initialization to pass models_config to workspace service

## Decision 3: Rebuild Embeddings with Document Reprocessing

**Decision**: Implement async re-embedding triggered by rebuild-embeddings endpoint.

**Approach**:

1. Clear vector storage for workspace (already done)
2. Queue all documents for re-processing via task queue
3. Use existing pipeline with workspace's new embedding config
4. Track progress via pipeline status endpoint

**Key insight**: The existing `reprocess-documents` endpoint already exists! We need to call it after clearing vectors.

## Decision 4: Workspace Isolation Invariant

**Decision**: All rebuild operations must be workspace-scoped.

**Already implemented**:

- `vector_storage.clear_workspace(&workspace_id)`
- `graph_storage.clear_workspace(&workspace_id)`

**Verify**: These methods only affect the specified workspace.

## Action Plan

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              ACTION PLAN                                     │
└─────────────────────────────────────────────────────────────────────────────┘

Step 1: Fix default constants in multitenancy.rs
        - Match models.toml defaults
        - Ollama for embedding

Step 2: Add ModelsConfig integration
        - New method to read defaults from ModelsConfig
        - Wire into workspace creation flow

Step 3: Update rebuild_embeddings to trigger reprocessing
        - After clearing vectors, queue documents for re-embedding
        - Return job_id for progress tracking

Step 4: Verify workspace isolation
        - Test that rebuild only affects target workspace
        - Other workspaces remain unaffected

Step 5: E2E test
        - Create workspace with Ollama defaults
        - Upload document (should use workspace LLM)
        - Query (should use workspace embedding)
        - Change embedding model
        - Rebuild embeddings (should re-embed with new model)
```

## Files to Modify

| File                                       | Change                                     |
| ------------------------------------------ | ------------------------------------------ |
| `edgequake-core/src/types/multitenancy.rs` | Sync constants, add ModelsConfig method    |
| `edgequake-api/src/handlers/workspaces.rs` | Trigger reprocessing in rebuild_embeddings |
| `edgequake-api/src/state.rs`               | Pass models_config to workspace defaults   |
