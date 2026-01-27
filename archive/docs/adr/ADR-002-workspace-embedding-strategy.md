# ADR-002: Workspace Embedding Strategy

**Status**: Accepted  
**Date**: 2025-01-27  
**Authors**: EdgeQuake Team  
**Implements**: SPEC-032 Ollama/LM Studio Provider Support  
**OODA Loop**: #47

## Context

When supporting multiple embedding providers with different dimensions (OpenAI: 1536, Ollama: 768), we need a strategy for:

1. Storing workspace-specific embedding configuration
2. Handling provider changes gracefully
3. Ensuring query-time embeddings match stored embeddings
4. Supporting mixed-provider environments

## Decision

We implemented a **Workspace-Level Embedding Configuration** strategy:

### 1. Workspace Embedding Metadata

Each workspace stores embedding configuration in its metadata:

```json
{
  "id": "workspace-uuid",
  "name": "My Workspace",
  "embedding_config": {
    "provider": "ollama",
    "model": "nomic-embed-text",
    "dimension": 768
  }
}
```

### 2. Query-Time Provider Selection

When processing queries, the system:

1. Retrieves workspace embedding config
2. Selects the matching provider
3. Generates query embedding with correct dimension
4. Performs vector similarity search

```rust
async fn process_query(&self, workspace_id: &str, query: &str) -> Result<Vec<SearchResult>> {
    let config = self.get_workspace_embedding_config(workspace_id).await?;
    let provider = self.provider_registry.get(&config.provider)?;

    // Ensure provider dimension matches stored dimension
    assert_eq!(provider.dimension(), config.dimension);

    let query_embedding = provider.embed_one(query).await?;
    self.vector_storage.query(&query_embedding, 10, None).await
}
```

### 3. Provider Change Flow

When a user changes the embedding provider for a workspace:

1. UI shows warning about dimension mismatch
2. User confirms provider change
3. System updates workspace embedding config
4. User triggers "Rebuild Embeddings" to re-embed all documents

## Consequences

### Positive

- **Workspace isolation**: Each workspace can use different providers
- **Explicit configuration**: No implicit provider selection during queries
- **Safe provider changes**: User must explicitly rebuild after changing
- **Audit trail**: Workspace metadata records which provider was used

### Negative

- **Rebuild overhead**: Changing providers requires re-processing all documents
- **Storage complexity**: Must track embedding config per workspace
- **UI complexity**: Need to display provider info and rebuild status

### Mitigations

- Rebuild endpoint shows progress and estimated time
- UI displays clear warning before provider changes
- Background job system for large rebuilds (future enhancement)

## API Endpoints

### Get Workspace Embedding Config

```
GET /api/workspaces/{id}/embedding-config
Response: { provider, model, dimension }
```

### Update Workspace Embedding Config

```
PUT /api/workspaces/{id}/embedding-config
Body: { provider, model }
Response: { success, requires_rebuild }
```

### Rebuild Workspace Embeddings

```
POST /api/workspaces/{id}/rebuild-embeddings
Response: { job_id, status, documents_processed }
```

## Related ADRs

- ADR-001: Provider Registry Pattern
- ADR-003: Vector Rebuild Safety

## References

- [SPEC-032](../specs/032-ollama-lmstudio-provider/032-ollama-lmstudio-provider.md)
- [WebUI Provider Selector](../edgequake_webui/src/components/settings/EmbeddingProviderSelector.tsx)
