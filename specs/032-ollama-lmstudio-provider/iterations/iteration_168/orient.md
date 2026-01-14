# OODA 168: Orient - LLM Model Selection Integration Strategy

## Date: 2026-01-14

## Analysis

The `HeaderTenantSelector` is the primary UI for tenant/workspace management in the application header. It currently lacks LLM model selection for both tenant and workspace creation.

## Integration Strategy

### Pattern to Follow

Use the existing `TenantWorkspaceSelector` implementation as reference:

- Import `LLMModelSelector` component
- Add state for `LLMSelection`
- Include in mutation payload

### UI Layout

```
┌─────────────────────────────────────────┐
│ Create New Tenant                       │
├─────────────────────────────────────────┤
│ Name: [________________]                │
│ Description: [________________]         │
│                                         │
│ ▼ Default AI Configuration              │
│ ┌─────────────────────────────────────┐ │
│ │ LLM Model:                          │ │
│ │ [OpenAI / gpt-4o-mini ▼]           │ │
│ │                                     │ │
│ │ Embedding Model:                    │ │
│ │ [OpenAI / text-embedding-3-small ▼]│ │
│ └─────────────────────────────────────┘ │
│                                         │
│ [Cancel]              [Create]          │
└─────────────────────────────────────────┘
```

### Component Dependencies

```
LLMModelSelector       -> hooks/use-providers -> /api/v1/models/llm
EmbeddingModelSelector -> hooks/use-providers -> /api/v1/models/embedding
```

## Decision Points

1. **Collapsible section**: Use accordion/disclosure for model config to keep dialog clean
2. **Default values**: Pre-select server defaults if available
3. **Validation**: Allow null selections (use server defaults)
4. **Labels**: Use clear i18n labels for accessibility

## Risk Assessment

| Risk            | Mitigation                           |
| --------------- | ------------------------------------ |
| Dialog too tall | Use collapsible sections             |
| API load        | Models are cached via React Query    |
| Breaking change | Optional fields, backward compatible |
