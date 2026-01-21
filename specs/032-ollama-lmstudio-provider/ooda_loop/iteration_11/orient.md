# Iteration 11: Orient

**Date:** 2025-01-30
**Focus:** Ingestion Pipeline Workspace LLM Integration

## Analysis

### Problem

Each workspace has its own LLM configuration (`llm_model`, `llm_provider`), but the ingestion pipeline uses a global LLM provider configured at server startup. This means:

- All workspaces use the same LLM for entity extraction
- Workspace-level model selection has no effect on ingestion
- Cannot leverage different models for different workspaces

### Solution Architecture

```
AppState
├── pipeline: Arc<Pipeline>                    ← Global fallback
├── create_workspace_pipeline(workspace_id)    ← NEW: Dynamic factory
    ├── Lookup workspace config
    ├── Create workspace-specific LLM provider
    ├── Create workspace-specific embedding provider
    └── Return configured Pipeline
```

### Implementation Strategy

1. **Add `create_llm_provider()` to ProviderFactory**

   - Mirror `create_embedding_provider()` structure
   - Support all provider types: OpenAI, Ollama, LM Studio, Mock
   - Use environment variables for base URLs/API keys

2. **Add `create_workspace_pipeline()` to AppState**

   - Take workspace_id as parameter
   - Lookup workspace configuration
   - Create providers using ProviderFactory
   - Return configured Pipeline
   - Fall back to global pipeline on errors

3. **Update document upload handler**
   - Call `create_workspace_pipeline()` instead of `state.pipeline`
   - Pass workspace_id from tenant context

### Key Insight

Creating pipelines dynamically is lightweight:

- Provider creation is cheap (just config + HTTP client)
- No pre-warming or connection pooling needed
- LLM calls are the expensive part, not pipeline creation
