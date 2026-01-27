# Observe - Iteration 139

## Focus: Query Page Provider Selection with Lineage (Item 3)

Verifying SPEC-032 requirement:

- **Item 3**: Query page allows provider/model selection, with lineage stored and displayed

## Investigation

### Query Interface

**File**: `edgequake_webui/src/components/query/query-interface.tsx`

- Line 83: Imports `ProviderModelSelector`
- Line 922-928: Uses `ProviderModelSelector` component for provider/model selection

### Provider Model Selector

**File**: `edgequake_webui/src/components/query/provider-model-selector.tsx`

- Line 6: `@implements SPEC-032: Ollama/LM Studio provider support - Query interface selector`
- Provides searchable dropdown with provider grouping and capability badges

### Backend Lineage Storage

**File**: `edgequake/crates/edgequake-api/src/handlers/chat.rs`

- Line 537: `// SPEC-032 Item 18, 22: Token metrics and model lineage`
- Line 541-542: `llm_provider: used_provider.clone()`, `llm_model: used_model.clone()`
- Line 546-548: `// SPEC-032: Provider lineage tracking` with provider/model fields

### Frontend Lineage Display

**File**: `edgequake_webui/src/components/query/chat-message.tsx`

- Line 62: `/** LLM model used (lineage tracking). @implements SPEC-032 */`
- Line 234: `{/* SPEC-032: Display LLM provider/model as lineage badge */}`
- Line 254: Displays `{t('query.llmLineage', 'LLM Provider')}: {llmProvider || 'server default'}`
- Line 295: Displays `{llmProvider}/{llmModel}` format

### API Types

**File**: `edgequake_webui/src/lib/api/chat.ts`

- Line 111: `/** LLM model used (lineage tracking). @implements SPEC-032 */`
- Line 143: Same comment for streaming types

## Findings

Item 3 is fully implemented:

- ✅ Query page has provider/model selector (`ProviderModelSelector`)
- ✅ Selection is used during query execution
- ✅ Lineage (provider/model) is stored in backend
- ✅ Lineage is displayed in UI next to token usage
