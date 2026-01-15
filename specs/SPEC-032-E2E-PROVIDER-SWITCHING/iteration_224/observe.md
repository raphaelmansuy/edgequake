# OODA 224: OBSERVE - Vector Storage with Workspace Configuration

## Objective

Test that vector storage operations use workspace-specific embedding dimension configuration. This is critical for:

1. Each workspace having its own embedding dimension
2. Dimension changes invalidating existing vectors
3. Vector search working with correct dimension

## Current Flow

### get_workspace_vector_storage (documents.rs:73)

```rust
async fn get_workspace_vector_storage(
    state: &AppState,
    workspace_id: &str,
) -> Arc<dyn VectorStorage> {
    // Get workspace config
    let workspace = state.workspace_service.get_workspace(uuid).await;
    
    // Create vector storage with workspace dimension
    match workspace {
        Ok(Some(ws)) => {
            // Use workspace-specific dimension
            state.create_vector_storage_with_dimension(ws.embedding_dimension)
        }
        _ => {
            // Fallback to global default
            state.vector_storage.clone()
        }
    }
}
```

## Key Integration Points

1. **workspace.embedding_dimension** - Stored dimension per workspace
2. **create_vector_storage_with_dimension()** - Creates storage with specific dimension
3. **VectorStorage.upsert()** - Stores embeddings with correct dimension
4. **VectorStorage.search()** - Searches with correct dimension

## Test Scenarios Needed

### Scenario 1: Workspace with standard dimension (768)
- Create workspace with 768 dimension
- Verify vector operations use 768

### Scenario 2: Workspace with large dimension (1536)
- Create workspace with OpenAI's 1536 dimension
- Verify vector operations use 1536

### Scenario 3: Dimension change between workspaces
- Workspace A: 768 dimension
- Workspace B: 1536 dimension
- Verify isolation

### Scenario 4: Dimension update on existing workspace
- Create workspace with 768
- Update to 1536
- Verify dimension change detected

## Files to Create

`edgequake/crates/edgequake-api/tests/e2e_vector_storage_dimension.rs`
