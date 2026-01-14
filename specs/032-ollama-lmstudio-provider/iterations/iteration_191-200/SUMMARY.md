# OODA Iterations 191-200: Query Response Lineage Display

## Objective
Verify and document the query response lineage display (showing which provider/model was used).

## Status: ✅ Already Implemented

The query response lineage display was already implemented during earlier OODA iterations.

## Implementation Details

### 1. Message Interface (query-interface.tsx:106-110)
```typescript
interface Message {
  // ... other fields
  /** LLM provider used (lineage tracking). @implements SPEC-032 */
  llmProvider?: string;
  /** LLM model used (lineage tracking). @implements SPEC-032 */
  llmModel?: string;
}
```

### 2. Server Message Conversion (query-interface.tsx:496-498)
```typescript
// SPEC-032: LLM provider/model lineage tracking
llmProvider: msg.llm_provider ?? undefined,
llmModel: msg.llm_model ?? undefined,
```

### 3. Streaming Chunk Capture (query-interface.tsx:672-674)
```typescript
case 'done':
  // SPEC-032: Capture LLM provider/model for lineage tracking
  llmProvider = chunk.llm_provider;
  llmModel = chunk.llm_model;
```

### 4. UI Display (chat-message.tsx:230-253)
The `MetadataBar` component renders a styled badge showing:
- Provider name (e.g., "ollama", "openai")
- Model name (truncated at `:`, e.g., "gemma3:12b" → "gemma3")
- Brain icon for visual identification
- Tooltip with full provider and model details

### 5. Visual Design
- Badge: Blue-themed with secondary variant
- Color scheme: `bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-300`
- Icon: Brain (from lucide-react)
- Tooltip: Shows full provider and model on hover

## Backend Support
The backend sends lineage information in:
1. **Streaming `done` chunks**: `llm_provider` and `llm_model` fields
2. **Saved messages**: Persisted in the message metadata

## E2E Tests Added
- Verified API returns provider/model metadata
- Verified streaming chunks include lineage information

## Next Steps
- OODA 201-210: Workspace settings page enhancements
