# OODA Loop Iteration 57 - Decide

## Decision Date
2025-01-27

## Decisions Made

### D1: Add Model Selection to Tenant Creation Dialog

**Decision**: Modify `tenant-guard.tsx` to include model selection dropdowns.

**Implementation Plan**:
1. Import ModelSelector component
2. Add state: `selectedLlmModel`, `selectedEmbeddingModel`
3. Add two ModelSelector instances (type="llm", type="embedding")
4. Update `createTenant` API call to include model config

### D2: Add Model Selection to Workspace Creation Dialog (tenant-workspace-selector)

**Decision**: Modify `tenant-workspace-selector.tsx` to include model selection.

**Implementation Plan**:
1. Import ModelSelector component
2. Add state for LLM and embedding model selection
3. Add ModelSelector to workspace creation dialog
4. Update `createWorkspace` API call

### D3: Update API Client Types

**Decision**: Verify and update `edgequake.ts` API types to include model fields.

**Files to Check**:
- `edgequake_webui/src/lib/api/edgequake.ts`
- `edgequake_webui/src/types/api.ts` (if exists)

### D4: Make Model Selection Optional with Defaults

**Decision**: Model selection is optional. If not selected, use server defaults.

**Rationale**: 
- Maintains backward compatibility
- Reduces friction for quick workspace creation
- Server already has sensible defaults configured

### D5: Show Currently Selected Model Info

**Decision**: Display the selected model's provider and name clearly in the dialog.

**UI Pattern**:
```
LLM Model
┌────────────────────────────────────┐
│ openai:gpt-4o-mini                 │ ←── Shows full model path
│ OpenAI · Vision · 128K context     │ ←── Shows capabilities
└────────────────────────────────────┘

Embedding Model  
┌────────────────────────────────────┐
│ openai:text-embedding-3-small      │
│ OpenAI · 1536 dimensions           │
└────────────────────────────────────┘
```

## Acceptance Criteria

- [ ] Tenant creation dialog shows LLM model selector
- [ ] Tenant creation dialog shows embedding model selector
- [ ] Workspace creation dialog shows LLM model selector
- [ ] Workspace creation dialog shows embedding model selector
- [ ] Model selection is optional (defaults used if not selected)
- [ ] API calls include model configuration when provided
- [ ] No regression in existing functionality
