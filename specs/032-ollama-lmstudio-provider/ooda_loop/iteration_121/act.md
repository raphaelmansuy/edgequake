# OODA Iteration 121: Act

## Date: 2026-01-14

## Changes Made

### 1. Synced Default Constants with models.toml

**File**: [multitenancy.rs](../../../../edgequake/crates/edgequake-core/src/types/multitenancy.rs#L314-L344)

**Before**:

```rust
pub const DEFAULT_EMBEDDING_MODEL: &str = "text-embedding-3-small";
pub const DEFAULT_EMBEDDING_PROVIDER: &str = "openai";
pub const DEFAULT_EMBEDDING_DIMENSION: usize = 1536;
```

**After**:

```rust
pub const DEFAULT_EMBEDDING_MODEL: &str = "embeddinggemma";
pub const DEFAULT_EMBEDDING_PROVIDER: &str = "ollama";
pub const DEFAULT_EMBEDDING_DIMENSION: usize = 768;
```

**Rationale**: Ensures code defaults match models.toml configuration, enabling development without API keys.

### 2. Enhanced rebuild_embeddings with Document Reprocessing

**File**: [workspaces.rs](../../../../edgequake/crates/edgequake-api/src/handlers/workspaces.rs#L897-L1070)

**Changes**:

- Added workspace embedding config update when config changes
- Added document queuing for re-embedding
- Updated response status to "processing" when documents are queued
- Added track_id for progress monitoring

**Key addition**:

```rust
// 6. Update workspace embedding config if changed (SPEC-032)
if config_changed {
    let update_request = UpdateWorkspaceRequest {
        embedding_model: Some(new_model.clone()),
        embedding_provider: Some(new_provider.clone()),
        embedding_dimension: Some(new_dimension),
        ..Default::default()
    };

    state.workspace_service.update_workspace(workspace_id, update_request).await?;
}

// 7. Queue documents for re-embedding (SPEC-032 REQ-25)
// ... document queuing logic
```

### 3. Enhanced rebuild_knowledge_graph with Document Reprocessing

**File**: [workspaces.rs](../../../../edgequake/crates/edgequake-api/src/handlers/workspaces.rs#L1230-L1410)

**Changes**:

- Added workspace LLM config update when config changes
- Added document queuing for reprocessing (extraction + embedding)
- Updated response status to "processing" when documents are queued
- Added track_id for progress monitoring

**Key addition**:

```rust
// 8. Update workspace LLM config if changed (SPEC-032)
if config_changed {
    let update_request = UpdateWorkspaceRequest {
        llm_model: Some(new_llm_model.clone()),
        llm_provider: Some(new_llm_provider.clone()),
        ..Default::default()
    };

    state.workspace_service.update_workspace(workspace_id, update_request).await?;
}

// 9. Queue all documents for reprocessing (SPEC-032 REQ-24)
// ... document queuing logic
```

## Verification

### Build Check

```bash
cargo check --package edgequake-api
# ✅ Finished `dev` profile [unoptimized + debuginfo] target(s) in 7.84s
```

## Summary of SPEC-032 Requirements Addressed

| Requirement                                      | Status             | Implementation                                 |
| ------------------------------------------------ | ------------------ | ---------------------------------------------- |
| Item 22: Defaults from models.toml               | ✅ Fixed           | Synced constants to match Ollama defaults      |
| Item 23: Document upload uses workspace LLM      | ✅ Already working | create_workspace_pipeline() in state.rs        |
| Item 24: Query uses workspace embedding          | ✅ Already working | get_workspace_embedding_provider() in query.rs |
| Item 25: Rebuild embeddings with workspace model | ✅ Fixed           | Full implementation with reprocessing          |
| Item 24 (dup): Rebuild KG uses workspace LLM     | ✅ Fixed           | Full implementation with reprocessing          |

## Next OODA Iteration

- Run full test suite to verify no regressions
- E2E test with real providers
- Document the rebuild workflow in user docs
