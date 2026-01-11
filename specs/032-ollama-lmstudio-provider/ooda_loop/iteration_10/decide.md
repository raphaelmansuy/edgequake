# Iteration 10: Decide

**Date:** 2025-01-30
**Focus:** WebUI LLM Model Selector for Workspace Creation

## Decision

Create `LLMModelSelector` component and integrate it into the workspace creation dialog.

## Implementation Plan

### 1. Create LLMModelSelector Component

**File:** `edgequake_webui/src/components/workspace/llm-model-selector.tsx`

```tsx
export interface LLMSelection {
  model: string;      // e.g., "gemma3:12b"
  provider: string;   // e.g., "ollama"
  fullId: string;     // e.g., "ollama/gemma3:12b"
}

export function LLMModelSelector({
  value,
  onChange,
}: {
  value?: LLMSelection;
  onChange: (selection: LLMSelection | undefined) => void;
}) { ... }
```

### 2. Integrate into Workspace Dialog

**File:** `edgequake_webui/src/components/shared/tenant-workspace-selector.tsx`

Add:
- Import LLMModelSelector and EmbeddingModelSelector
- State: `selectedLLM`, `selectedEmbedding`
- Mutation update: pass LLM/embedding config
- Dialog section: model selectors below description

### 3. Type Definitions

**LLMSelection fields:**
- `model`: Raw model name (e.g., "gemma3:12b")
- `provider`: Provider name (e.g., "ollama")
- `fullId`: Combined format (e.g., "ollama/gemma3:12b")

## Acceptance Criteria

- [ ] LLMModelSelector component created (252 lines)
- [ ] Workspace dialog shows model selectors
- [ ] Mutation passes LLM/embedding config
- [ ] TypeScript compilation passes
- [ ] UI displays provider icons correctly
