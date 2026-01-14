# OODA Loop Iteration 57 - Orient

## Analysis Date
2025-01-27

## Strategic Assessment

### Problem Context
The spec (Focus 1 & 2) requires:
1. Tenant creation dialog must have LLM and embedding provider/model selection
2. Workspace creation dialog must have LLM and embedding provider/model selection

Currently, the dialogs only have name/description fields. Users cannot configure default models.

### Architecture Alignment

```
┌─────────────────────────────────────────────────────────────────┐
│                    CURRENT STATE                                │
├─────────────────────────────────────────────────────────────────┤
│  Tenant Dialog        Workspace Dialog        Backend API       │
│  ┌───────────┐       ┌───────────┐          ┌───────────┐      │
│  │ Name      │       │ Name      │   POST   │ Supports  │      │
│  │ [input]   │  →    │ Slug      │   ───→   │ llm_model │      │
│  │           │       │ [input]   │   !!!    │ embedding │      │
│  └───────────┘       └───────────┘   GAP    │ config    │      │
│                                              └───────────┘      │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                    TARGET STATE                                 │
├─────────────────────────────────────────────────────────────────┤
│  Tenant Dialog        Workspace Dialog        Backend API       │
│  ┌───────────┐       ┌───────────┐          ┌───────────┐      │
│  │ Name      │       │ Name      │   POST   │ Supports  │      │
│  │ LLM Model │  →    │ Slug      │   ───→   │ llm_model │      │
│  │ Embedding │       │ LLM Model │   ✓      │ embedding │      │
│  │ Model     │       │ Embedding │          │ config    │      │
│  └───────────┘       └───────────┘          └───────────┘      │
└─────────────────────────────────────────────────────────────────┘
```

### Design Decision: Model Value Format

The ModelSelector component returns values in format `provider:model` (e.g., `openai:gpt-4o-mini`).

The API expects separate fields:
- `llm_provider`: string (e.g., "openai")
- `llm_model`: string (e.g., "gpt-4o-mini")

**Decision**: Parse the ModelSelector value to extract provider and model.

```typescript
function parseModelValue(value: string): { provider: string; model: string } {
  const [provider, ...modelParts] = value.split(':');
  return { provider, model: modelParts.join(':') };
}
```

### Incremental Approach

1. **Step 1**: Add state variables for model selection in dialogs
2. **Step 2**: Add ModelSelector components to dialog JSX
3. **Step 3**: Update API calls to include model configuration
4. **Step 4**: Test with real models API

### Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| ModelSelector API not loading | Medium | Add loading states, graceful fallback to defaults |
| User confusion with options | Low | Add helpful descriptions, show defaults clearly |
| Breaking existing flow | High | Preserve current behavior when no selection made |

### Dependencies

- `ModelSelector` component: Already exists, stable
- `useLlmModels` hook: Already exists, uses react-query
- `useEmbeddingModels` hook: Already exists, uses react-query
- Backend API: Already supports model config

### Testing Strategy

1. Unit test: Model value parsing
2. Integration test: Create tenant with model config
3. Integration test: Create workspace with model config
4. E2E test: Full flow in browser
