# ADR-003: Vector Rebuild Safety

**Status**: Accepted  
**Date**: 2025-01-27  
**Authors**: EdgeQuake Team  
**Implements**: SPEC-032 Ollama/LM Studio Provider Support  
**OODA Loop**: #48

## Context

When users change embedding providers or models, existing vector embeddings become incompatible. We need a safe rebuild mechanism that:

1. Prevents data loss during rebuild
2. Handles partial failures gracefully
3. Maintains query availability during rebuild
4. Provides clear progress feedback

## Decision

We implemented a **Safe Vector Rebuild** pattern with the following components:

### 1. Atomic Rebuild Operations

The rebuild process uses atomic operations to prevent partial state:

```rust
pub async fn rebuild_workspace_embeddings(
    &self,
    workspace_id: &str,
    new_provider: &dyn EmbeddingProvider,
) -> Result<RebuildResult> {
    // Phase 1: Validate new provider is available
    new_provider.embed(&["test".to_string()]).await?;

    // Phase 2: Fetch all documents (read-only)
    let documents = self.document_storage.list_all(workspace_id).await?;

    // Phase 3: Generate new embeddings
    let mut new_embeddings = Vec::with_capacity(documents.len());
    for doc in &documents {
        let embedding = new_provider.embed_one(&doc.content).await?;
        new_embeddings.push((doc.id.clone(), embedding, doc.metadata.clone()));
    }

    // Phase 4: Atomic swap - clear and insert
    self.vector_storage.clear().await?;
    self.vector_storage.upsert(&new_embeddings).await?;

    // Phase 5: Update workspace config
    self.update_workspace_embedding_config(workspace_id, new_provider).await?;

    Ok(RebuildResult {
        documents_processed: documents.len(),
        new_dimension: new_provider.dimension(),
    })
}
```

### 2. Rebuild Status Tracking

```rust
pub enum RebuildStatus {
    NotStarted,
    InProgress { processed: usize, total: usize },
    Completed { at: DateTime<Utc> },
    Failed { error: String, at: DateTime<Utc> },
}
```

### 3. Query Behavior During Rebuild

During rebuild, the system:

- Continues serving queries with old embeddings until swap
- Briefly blocks queries during atomic swap (~100ms)
- Returns empty results if rebuild failed mid-swap

### 4. Rollback Capability (Future)

For large workspaces, we plan to add:

- Backup of old embeddings before rebuild
- Automatic rollback on failure
- Parallel embedding generation

## Consequences

### Positive

- **No data loss**: Documents are never deleted, only embeddings
- **Atomic swap**: Users see either old or new state, never partial
- **Progress visibility**: UI shows rebuild progress
- **Safe failure**: System remains usable even if rebuild fails

### Negative

- **Memory overhead**: New embeddings held in memory during rebuild
- **Brief downtime**: ~100ms query pause during atomic swap
- **No partial rebuild**: Must rebuild all documents

### Mitigations

- Streaming rebuild for large workspaces (future)
- Batch embedding generation to reduce memory
- Progress endpoint for long-running rebuilds

## API Endpoint

### Rebuild Embeddings

```
POST /api/workspaces/{id}/rebuild-embeddings
Request: { provider: "ollama", model: "nomic-embed-text" }
Response: {
  "job_id": "uuid",
  "status": "in_progress",
  "documents_total": 1000,
  "documents_processed": 0
}
```

### Get Rebuild Status

```
GET /api/workspaces/{id}/rebuild-status
Response: {
  "status": "in_progress",
  "documents_total": 1000,
  "documents_processed": 500,
  "estimated_remaining_seconds": 120
}
```

## Test Coverage

The rebuild safety is verified by:

1. `test_memory_vector_clear_for_rebuild` - Clear operation works
2. `test_memory_vector_repopulate_after_clear` - Repopulate works after clear
3. `test_read_during_rebuild` - Concurrent reads during rebuild
4. `test_workspace_dimension_isolation` - Different dimensions don't interfere

## Related ADRs

- ADR-001: Provider Registry Pattern
- ADR-002: Workspace Embedding Strategy

## References

- [SPEC-032](../specs/032-ollama-lmstudio-provider/032-ollama-lmstudio-provider.md)
- [Storage Compatibility Tests](../edgequake/crates/edgequake-storage/tests/provider_storage_compat.rs)
