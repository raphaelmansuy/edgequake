# OODA 122: Embedding Dimension Auto-Detection

## Date: 2026-01-14 14:50 UTC

## Observe

- Backend was running in memory mode instead of PostgreSQL mode
- Rebuild embeddings was not persisting workspace config changes
- When changing embedding models, dimension was NOT being updated
- E.g., switching from openai/text-embedding-3-small (1536d) to ollama/embeddinggemma (768d) kept dimension at 1536

## Orient

- Memory mode: DATABASE_URL was not being passed to the backend process
- Dimension issue: Code was using `request.embedding_dimension.unwrap_or(workspace.embedding_dimension)` which only uses the old workspace dimension if not explicitly provided in request
- Correct behavior: When model changes, auto-detect dimension from model config

## Decide

1. Start backend with explicit DATABASE_URL environment variable
2. Update rebuild_embeddings to look up dimension from models_config when model changes
3. Verify PostgreSQL mode and dimension auto-detection

## Act

### Changed Files

- `edgequake/crates/edgequake-api/src/handlers/workspaces.rs` (+26/-3 lines)
  - Added dimension auto-detection logic using `state.models_config.get_model()`
  - Falls back to workspace default if model not found in config

### Verification

1. Backend confirmed running in PostgreSQL mode (`storage_mode: "postgresql"`)
2. Reset workspace to openai/text-embedding-3-small (1536d)
3. Called rebuild-embeddings with ollama/embeddinggemma
4. Response shows `embedding_dimension: 768` (auto-detected!)
5. Workspace GET confirms persistence: `embedding_dimension: 768`

### Logs Evidence

```
old_dimension=1536 new_dimension=768 document_count=0 model_context_length=2048
Workspace embedding configuration updated workspace_id=... embedding_dimension=768
```

## Commit

```
8905eef fix(SPEC-032): auto-detect embedding dimension from model config on rebuild
```

## Status

- [x] Backend PostgreSQL mode working
- [x] Rebuild embeddings persists config
- [x] Dimension auto-detection from model config
- [x] All tests pass
