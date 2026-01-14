# Orient - Iteration 139

## Context Analysis

**Item 3**: Query page provider selection with lineage storage and display

### Data Flow

```
User selects model → Query request → Backend uses selected provider →
Response includes lineage → Frontend displays provider/model badge
```

### Component Integration

1. **Query Input** (`query-interface.tsx`):

   - `ProviderModelSelector` component for model selection
   - Selection passed to query API

2. **API Request** (chat.ts):

   - `llm_provider` and `llm_model` fields in request
   - Same fields returned in response for lineage

3. **Backend Handler** (chat.rs):

   - Creates LLM provider based on request parameters
   - Falls back to workspace config if not specified
   - Falls back to server default if no workspace
   - Stores `llm_provider` and `llm_model` in response

4. **Message Display** (chat-message.tsx):
   - Displays provider/model in badge format
   - Shows alongside token usage information
   - Format: `ollama/gemma3:12b`

### Key Code Locations

| Component   | File                        | Key Lines |
| ----------- | --------------------------- | --------- |
| Selector    | provider-model-selector.tsx | 93-98     |
| Query Input | query-interface.tsx         | 922-928   |
| Backend     | chat.rs                     | 537-548   |
| Display     | chat-message.tsx            | 234-295   |

### Provider Selection Priority

```
1. User selection (from ProviderModelSelector)
   ↓
2. Workspace configuration (llm_provider, llm_model)
   ↓
3. Server default (from models.toml)
```

## Assessment

**Item 3 (Query Provider Selection with Lineage): VERIFIED COMPLETE**

All requirements met:

- ✅ Provider/model selector on query page
- ✅ Selection is traced and stored
- ✅ Lineage displayed next to token usage
- ✅ Format: `provider/model` (e.g., `ollama/gemma3:12b`)
