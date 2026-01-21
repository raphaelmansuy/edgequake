# OODA Loop Iteration 57 - Act

## Action Date

2025-01-27

## Changes Implemented

### 1. Updated tenant-guard.tsx for Model Selection

**File**: [edgequake_webui/src/components/layout/tenant-guard.tsx](../../../../edgequake_webui/src/components/layout/tenant-guard.tsx)

**Changes**:

1. Added import for `ModelSelector` component
2. Added state variables for model selection:
   - `tenantLlmModel`, `tenantEmbeddingModel` for tenant creation
   - `workspaceLlmModel`, `workspaceEmbeddingModel` for workspace creation
3. Added `parseModelValue()` helper to convert "provider:model" to separate fields
4. Updated `createTenantMutation` to accept model configuration
5. Updated `createWorkspaceMutation` to accept model configuration
6. Updated `handleCreateTenant()` to include model config in API call
7. Updated `handleCreateWorkspace()` to include model config in API call
8. Added ModelSelector components to tenant creation dialog
9. Added ModelSelector components to workspace creation dialog

**Code Changes Summary**:

```typescript
// New state variables
const [tenantLlmModel, setTenantLlmModel] = useState<string>();
const [tenantEmbeddingModel, setTenantEmbeddingModel] = useState<string>();
const [workspaceLlmModel, setWorkspaceLlmModel] = useState<string>();
const [workspaceEmbeddingModel, setWorkspaceEmbeddingModel] =
  useState<string>();

// Parse "provider:model" format
const parseModelValue = (value: string | undefined) => {
  if (!value) return {};
  const colonIndex = value.indexOf(":");
  if (colonIndex === -1) return { model: value };
  return {
    provider: value.substring(0, colonIndex),
    model: value.substring(colonIndex + 1),
  };
};

// Updated API calls to include model config
const tenantData = {
  name: newTenantName,
  ...(llmConfig.model && { default_llm_model: llmConfig.model }),
  ...(llmConfig.provider && { default_llm_provider: llmConfig.provider }),
  ...(embeddingConfig.model && {
    default_embedding_model: embeddingConfig.model,
  }),
  ...(embeddingConfig.provider && {
    default_embedding_provider: embeddingConfig.provider,
  }),
};
```

### 2. Verified tenant-workspace-selector.tsx

**File**: [edgequake_webui/src/components/shared/tenant-workspace-selector.tsx](../../../../edgequake_webui/src/components/shared/tenant-workspace-selector.tsx)

**Status**: Already has model selection implemented using `LLMModelSelector` and `EmbeddingModelSelector` components.

## Test Results

### TypeScript Compilation

```
✓ pnpm exec tsc --noEmit - No errors
```

## Verification Checklist

- [x] Tenant creation dialog shows LLM model selector
- [x] Tenant creation dialog shows embedding model selector
- [x] Workspace creation dialog shows LLM model selector
- [x] Workspace creation dialog shows embedding model selector
- [x] Model selection is optional (defaults used if not selected)
- [x] API calls include model configuration when provided
- [x] TypeScript compilation passes
- [ ] Visual verification in browser (pending)

## Next Steps

1. Commit changes for OODA 57
2. Run E2E tests to verify functionality
3. Continue with Focus 3: Query page LLM provider selection
