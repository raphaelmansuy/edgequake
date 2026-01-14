# OODA Loop Iteration 57 - Observe

## Observation Date

2025-01-27

## Focus Area

Focus 1 & 2: Model selection in Tenant and Workspace creation dialogs

## Current State Analysis

### Backend API Status ✅

The backend fully supports model configuration for tenants and workspaces:

1. **Tenant Creation** ([workspaces.rs#L145-180](../../edgequake/crates/edgequake-api/src/handlers/workspaces.rs))

   - Supports `default_llm_model`, `default_llm_provider`
   - Supports `default_embedding_model`, `default_embedding_provider`, `default_embedding_dimension`
   - Auto-detects provider from model name if not specified

2. **Workspace Creation** ([workspaces.rs#L465-520](../../edgequake/crates/edgequake-api/src/handlers/workspaces.rs))

   - Supports `llm_model`, `llm_provider`
   - Supports `embedding_model`, `embedding_provider`, `embedding_dimension`
   - Falls back to tenant defaults if not specified

3. **Models API** ([models.rs#L113](../../edgequake/crates/edgequake-api/src/handlers/models.rs))
   - GET `/api/models` - lists all providers and models
   - GET `/api/llm/models` - lists LLM models
   - GET `/api/embedding/models` - lists embedding models

### Frontend Component Status

1. **ModelSelector Component** ✅

   - Location: `edgequake_webui/src/components/models/model-selector.tsx`
   - Fully implemented with provider grouping, capabilities display, cost indicators
   - Supports both `llm` and `embedding` types
   - Uses hooks: `useLlmModels()`, `useEmbeddingModels()`

2. **Tenant Guard Dialog** ❌ Missing model selection

   - Location: `edgequake_webui/src/components/layout/tenant-guard.tsx`
   - Current: Only name input field
   - Missing: LLM provider/model selection, embedding provider/model selection

3. **Tenant Workspace Selector** ❌ Missing model selection
   - Location: `edgequake_webui/src/components/shared/tenant-workspace-selector.tsx`
   - Current: Basic name/slug input
   - Missing: LLM and embedding model selection

### Gap Analysis

| Feature                                 | Backend | Frontend | Gap                                   |
| --------------------------------------- | ------- | -------- | ------------------------------------- |
| Tenant creation with LLM model          | ✅      | ❌       | Add ModelSelector to tenant dialog    |
| Tenant creation with embedding model    | ✅      | ❌       | Add ModelSelector to tenant dialog    |
| Workspace creation with LLM model       | ✅      | ❌       | Add ModelSelector to workspace dialog |
| Workspace creation with embedding model | ✅      | ❌       | Add ModelSelector to workspace dialog |

### Files to Modify

1. `edgequake_webui/src/components/layout/tenant-guard.tsx`

   - Add ModelSelector for LLM selection in tenant creation dialog
   - Add ModelSelector for embedding selection in tenant creation dialog

2. `edgequake_webui/src/lib/api/edgequake.ts`

   - Update `createTenant()` to pass model config
   - Update `createWorkspace()` to pass model config

3. `edgequake_webui/src/components/shared/tenant-workspace-selector.tsx`
   - Add ModelSelector for LLM selection in workspace creation dialog
   - Add ModelSelector for embedding selection in workspace creation dialog

## Key Metrics

- Lines of code to add: ~200-300
- Components affected: 3
- New dependencies: None (ModelSelector already exists)
