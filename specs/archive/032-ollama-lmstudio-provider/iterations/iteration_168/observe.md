# OODA 168: Observe - Tenant/Workspace LLM/Embedding Selection Gap

## Date: 2026-01-14

## Observation Summary

Analyzed the current implementation for SPEC-032 Focus areas 1 & 2:

1. **Tenant creation with default LLM/embedding provider/model selection**
2. **Workspace creation with default LLM/embedding provider/model selection**

## Current State

### WebUI Components Found:

| Component                 | Location                                                                                             | LLM Selection | Embedding Selection         |
| ------------------------- | ---------------------------------------------------------------------------------------------------- | ------------- | --------------------------- |
| `HeaderTenantSelector`    | [header-tenant-selector.tsx](edgequake_webui/src/components/layout/header-tenant-selector.tsx)       | ❌ Missing    | ✅ Partial (embedding only) |
| `TenantWorkspaceSelector` | [tenant-workspace-selector.tsx](edgequake_webui/src/components/shared/tenant-workspace-selector.tsx) | ✅ Complete   | ✅ Complete                 |
| `TenantGuard`             | [tenant-guard.tsx](edgequake_webui/src/components/layout/tenant-guard.tsx)                           | ✅ Complete   | ✅ Complete                 |

### Gap Analysis:

1. **HeaderTenantSelector** (main header component):
   - Tenant creation: NO LLM model selection
   - Workspace creation: Has `EmbeddingModelSelector` but NO `LLMModelSelector`
2. **TenantWorkspaceSelector** (shared component):
   - Full support for both LLM and embedding selection
   - But not used in main header

### Backend Support:

- ✅ `createTenant` API accepts `default_llm_provider`, `default_llm_model`
- ✅ `createWorkspace` API accepts `embedding_provider`, `embedding_model`, `llm_provider`, `llm_model`
- ✅ Models API provides `/api/v1/models/llm` and `/api/v1/models/embedding`

## Files to Modify

1. [header-tenant-selector.tsx](edgequake_webui/src/components/layout/header-tenant-selector.tsx#L360-L405)
   - Add `LLMModelSelector` to tenant creation dialog
   - Add `LLMModelSelector` to workspace creation dialog

## Next Step

Orient phase: Design the UI integration for LLM model selection in both dialogs.
