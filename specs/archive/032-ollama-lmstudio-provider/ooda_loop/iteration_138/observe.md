# Observe - Iteration 138

## Focus: Tenant/Workspace Creation with Model Selection (Items 1, 2, 12)

Verifying SPEC-032 requirements:

- **Item 1**: Tenant creation dialog has LLM and embedding provider/model selection
- **Item 2**: Workspace creation dialog has LLM and embedding provider/model selection
- **Item 12**: Default provider/model selection available for both tenant and workspace creation

## Investigation

### File: `tenant-workspace-selector.tsx`

Location: `edgequake_webui/src/components/shared/tenant-workspace-selector.tsx`
Lines: 674

Key elements:

- State variables: `tenantDefaultLLM`, `tenantDefaultEmbedding`, `selectedLLM`, `selectedEmbedding`
- Components: `LLMModelSelector`, `EmbeddingModelSelector` imported from workspace components
- Mutations pass model config to API

### API: `CreateTenantRequest`

Location: `edgequake_webui/src/lib/api/edgequake.ts`

```typescript
export interface CreateTenantRequest {
  name: string;
  description?: string;
  plan?: string;
  // SPEC-032 fields
  default_llm_model?: string;
  default_llm_provider?: string;
  default_embedding_model?: string;
  default_embedding_provider?: string;
  default_embedding_dimension?: number;
}
```

### Tenant Creation Dialog (lines 469-568)

- Includes name/description fields
- Has `LLMModelSelector` with `tenantDefaultLLM` state
- Has `EmbeddingModelSelector` with `tenantDefaultEmbedding` state
- Hint text: "New workspaces will inherit this default"
- Passes all model config to API on create

### Workspace Creation Dialog (lines 571-656)

- Includes name/description fields
- Has `LLMModelSelector` with `selectedLLM` state
- Has `EmbeddingModelSelector` with `selectedEmbedding` state
- Passes all model config to API on create

## Findings

All three items are fully implemented with proper UI components and API integration.
