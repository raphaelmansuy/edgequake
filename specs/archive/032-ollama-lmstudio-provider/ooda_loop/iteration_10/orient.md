# Iteration 10: Orient

**Date:** 2025-01-30
**Focus:** WebUI LLM Model Selector for Workspace Creation

## Analysis

### Component Design Pattern

Following the existing `EmbeddingModelSelector` pattern:

1. **Selection Interface** - Type-safe selection object
2. **Hook Integration** - Uses `useAvailableProviders()`
3. **Provider Icons** - Visual distinction (Cloud, Cpu, Brain)
4. **Full ID Format** - Combines provider + model into `provider/model`

### Required Changes

1. **Create LLMModelSelector Component**

   - Mirror structure of `embedding-model-selector.tsx`
   - Add `LLMSelection` interface
   - Add `formatFullId()` and `parseFullId()` utilities
   - Explain purpose: "Used for document ingestion, entity extraction, and summarization"

2. **Integrate into Workspace Dialog**
   - Add imports for both model selectors
   - Add state for `selectedLLM` and `selectedEmbedding`
   - Update mutation to pass LLM/embedding config
   - Reset selections on successful creation

### Component Hierarchy

```
TenantWorkspaceSelector
├── Tenant Selection
├── Workspace Selection
└── Create Workspace Dialog
    ├── Name Input
    ├── Description Input
    ├── LLMModelSelector          ← NEW
    └── EmbeddingModelSelector    ← ADD
```

## Key Insight

The workspace LLM is **distinct** from the query LLM:

- **Workspace LLM**: Used during document ingestion (entity extraction, summarization)
- **Query LLM**: Used during query execution (answer generation)

This separation allows workspaces to use cost-effective local models for ingestion while using cloud models for query answering.
