# OODA 71 - Orient: Rebuild Embeddings API Testing

## Analysis

### Rebuild API Endpoint
```
POST /api/v1/workspaces/{workspace_id}/rebuild-embeddings
Request: RebuildEmbeddingsRequest {
  embedding_model?: string,
  embedding_provider?: string,
  embedding_dimension?: number,
  force?: boolean
}
Response: RebuildEmbeddingsResponse
```

### Implementation Notes from Code
1. Clears all vector embeddings for workspace
2. Optionally updates embedding model configuration
3. Documents need to be re-processed to regenerate embeddings
4. Returns error if config unchanged and `force: false`

### Test Strategy

1. **API Level Test**: Verify endpoint exists and responds correctly
   - Success case: force rebuild with `force: true`
   - Error case: no changes without force

2. **UI Level Test**: Already have button visibility (OODA 70)
   - Could add button click interaction test

### Recommendation

Add 1 API test for rebuild endpoint:
- "rebuild embeddings API endpoint exists and responds"

This validates Focus 5 at API level. UI button visibility already covered.
