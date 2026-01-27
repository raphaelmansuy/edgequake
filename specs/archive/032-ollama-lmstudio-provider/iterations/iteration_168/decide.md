# OODA 168: Decide - Implementation Plan

## Date: 2026-01-14

## Decision

Enhance `HeaderTenantSelector` to include LLM model selection for both tenant and workspace creation.

## Changes Required

### 1. Add Imports

```tsx
import {
  LLMModelSelector,
  type LLMSelection,
} from "@/components/workspace/llm-model-selector";
```

### 2. Add State Variables

```tsx
// Tenant default LLM configuration
const [tenantDefaultLLM, setTenantDefaultLLM] = useState<
  LLMSelection | undefined
>(undefined);
// Workspace LLM configuration
const [workspaceLLM, setWorkspaceLLM] = useState<LLMSelection | undefined>(
  undefined
);
```

### 3. Update Tenant Creation Dialog

Add LLM model selector after description field.

### 4. Update Workspace Creation Dialog

Add LLM model selector alongside existing embedding selector.

### 5. Update Mutation Payloads

Include LLM configuration in both tenant and workspace creation mutations.

## Files to Modify

- [header-tenant-selector.tsx](edgequake_webui/src/components/layout/header-tenant-selector.tsx)

## Acceptance Criteria

- [ ] Tenant creation dialog shows LLM model selector
- [ ] Tenant creation dialog shows embedding model selector
- [ ] Workspace creation dialog shows LLM model selector
- [ ] Both mutations include model configuration
- [ ] UI compiles without errors
- [ ] No regression in existing functionality
