# OODA 69 - Observe: Query Interface Provider Selection

## Mission Alignment Check
✅ Focus 3: "On Query --> Ensure I can chose the current LLM Provider -> Ensure it used, traced and stored in the generated message"

## Current State

### ProviderModelSelector Component
- Located at: `edgequake_webui/src/components/query/provider-model-selector.tsx`
- Implements SPEC-032 provider selection in query interface
- Referenced in `query-interface.tsx` at line 920-921

### Query Interface Integration
- `QueryInterface` component includes `ProviderModelSelector`
- Selector is positioned in the input area for easy access
- Supports workspace-level model defaults

## E2E Coverage Gap

### Current Tests (spec032-provider-integration.spec.ts)
- ✅ API provider/model structure
- ✅ Default configuration validation
- ✅ Streaming capability
- ❌ **No UI-level provider selection test**
- ❌ **No test for provider choice persistence**
- ❌ **No test for query submission with selected provider**

## Observation

The query interface has `ProviderModelSelector` component that allows users to:
1. View available providers
2. Select LLM model for query
3. Persist selection in local state

However, no E2E test validates:
1. Provider selector is visible on query page
2. Provider dropdown shows available models
3. Selection changes are reflected in the UI
