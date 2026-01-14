# OODA Iteration 123: Orient

## Date: 2026-01-14

## Analysis: Chunk-Embedding Compatibility

### Current State Assessment

**rebuild_embeddings handler** ([workspaces.rs#L814-L1140](../../../../edgequake/crates/edgequake-api/src/handlers/workspaces.rs#L814)):

| Feature | Status | Notes |
|---------|--------|-------|
| Clear vectors | ✅ Implemented | `vector_storage.clear_workspace()` |
| Update workspace config | ✅ Implemented | Updates embedding_model, provider, dimension |
| Queue documents for re-embedding | ✅ Implemented | Creates Task with `is_embedding_rebuild: true` |
| Dimension auto-detection | ✅ Implemented | Looks up from models_config |
| Chunk compatibility check | ✅ Implemented | Warning if chunk_size > context_length |
| Response with job_id | ✅ Implemented | Returns track_id for monitoring |

### Model Context Lengths (from models.toml)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    EMBEDDING MODEL CONTEXT LENGTHS                           │
├─────────────────────────────────────────────────────────────────────────────┤
│ PROVIDER     │ MODEL                    │ CONTEXT_LENGTH │ STATUS           │
├──────────────┼──────────────────────────┼────────────────┼──────────────────┤
│ OpenAI       │ text-embedding-3-small   │ 8191           │ ✅ Safe          │
│ OpenAI       │ text-embedding-3-large   │ 8191           │ ✅ Safe          │
│ Ollama       │ embeddinggemma           │ 2048           │ ✅ Safe (> 1200) │
│ Ollama       │ nomic-embed-text         │ 2048           │ ✅ Safe (> 1200) │
│ Ollama       │ mxbai-embed-large        │ 512            │ ⚠️ PROBLEM!      │
│ LM Studio    │ text-embedding-nomic     │ 8192           │ ✅ Safe          │
└─────────────────────────────────────────────────────────────────────────────┘

Default Chunk Size: 1200 tokens

CRITICAL: mxbai-embed-large (512) < Default Chunk (1200)
```

### Risk Analysis

**Current behavior**: Warning is logged but operation proceeds.

**Consequences**:
1. Chunks exceeding model limit will fail embedding
2. User may not see the warning (only in logs)
3. Partial re-embedding could leave workspace in inconsistent state

### Mitigation Options

1. **Strict Mode**: Block incompatible model changes (breaking change)
2. **Warn in Response**: ✅ Already implemented via `compatibility_warning`
3. **Re-chunking**: Future enhancement - re-chunk documents with new size
4. **Configurable Chunk Size**: Add chunk_size to workspace settings

### What's Missing

1. **WebUI Warning Display**: Need to show `compatibility_warning` to user
2. **Re-chunking Option**: Not implemented, would require significant changes
3. **Progress Tracking UI**: Need to verify pipeline status shows re-embedding

## Decision Points

| Decision | Priority | Action |
|----------|----------|--------|
| Display warning in WebUI | High | Check if frontend shows warning |
| Progress tracking works | High | Verify pipeline status endpoint |
| Re-chunking automation | Low | Document as future enhancement |
