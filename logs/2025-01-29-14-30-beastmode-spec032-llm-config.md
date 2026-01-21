# Task Log: SPEC-032 Workspace LLM Configuration

**Date:** 2025-01-29

## Actions

1. Added `llm_model` and `llm_provider` fields to `Workspace` struct in `multitenancy.rs`
2. Added `DEFAULT_LLM_MODEL` ("gemma3:12b") and `DEFAULT_LLM_PROVIDER` ("ollama") constants
3. Implemented `llm_full_id()`, `embedding_full_id()`, `parse_model_id()` helper methods
4. Updated `CreateWorkspaceRequest` with optional LLM configuration fields
5. Updated API DTOs (`CreateWorkspaceApiRequest`, `UpdateWorkspaceApiRequest`, `WorkspaceResponse`)
6. Updated `create_workspace` handler to pass LLM config to service
7. Updated both `InMemoryWorkspaceService` and `PostgresWorkspaceServiceImpl` to handle LLM config
8. Added LLM fields to TypeScript types (`Workspace`, `CreateWorkspaceRequest`)
9. Fixed 20+ test locations to include new LLM fields
10. Verified all 2400+ tests pass

## Decisions

- Use `provider/model_name` format for combined model IDs (e.g., "ollama/gemma3:12b")
- Store LLM config in metadata JSONB (same pattern as embedding config)
- Default to Ollama gemma3:12b as the default LLM provider
- LLM for ingestion is separate from query-time LLM selection

## Next Steps

1. Add `LLMModelSelector` component to WebUI workspace creation dialog
2. Integrate workspace LLM config into entity extraction and summarization pipelines
3. Consider database migration to add dedicated LLM columns (currently in metadata JSONB)
4. Document environment variables in `.env.example`

## Lessons/Insights

- The `provider/model` format provides clear identification without ambiguity
- Storing config in metadata JSONB is flexible but requires extraction in `into_workspace()`
- Test files need systematic updates when adding struct fields - sed helped but manual verification needed
