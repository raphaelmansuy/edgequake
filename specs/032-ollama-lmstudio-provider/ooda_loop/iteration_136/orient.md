# Iteration 136 – Orient

## Analysis

### Embedding Model Change Flow

From previous OODA loops and code search:

#### 1. Workspace Settings Page ([workspace/page.tsx](edgequake_webui/src/app/(dashboard)/workspace/page.tsx))
- **Embedding Model Selector**: Lines 544-556 (EmbeddingModelSelector)
- **Change Detection**: Lines 258-261 (embeddingModelChanged)
- **Warning Display**: Line 552 ("requires rebuilding all document embeddings")
- **Post-Save Rebuild Prompt**: Lines 648-660

#### 2. Rebuild Embeddings Button ([rebuild-embeddings-button.tsx](edgequake_webui/src/components/workspace/rebuild-embeddings-button.tsx))
- Triggers `/workspaces/{id}/rebuild-embeddings` API
- Shows progress in pipeline dialog

#### 3. Backend Handler (OODA 123)
- **Location**: [workspaces.rs#L814-1140](edgequake/crates/edgequake-api/src/handlers/workspaces.rs#L814)
- Clears existing vectors
- Updates workspace embedding config
- Queues documents for re-embedding
- Returns compatibility_warning if needed

### Data Flow

```
User changes embedding model → Save → Warning shown → 
User clicks "Rebuild Embeddings" → 
API clears vectors → Queues documents → 
Pipeline re-embeds with new model
```

## Conclusion

**Item 20 (Embedding model change at workspace level): VERIFIED COMPLETE**

- UI allows embedding model selection
- Change triggers rebuild prompt
- API handles vector clearing and reprocessing
