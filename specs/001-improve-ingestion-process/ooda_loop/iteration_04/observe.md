# Iteration 04 - OBSERVE Phase

## Objective
Enhance UX for rebuild confirmation dialogs with impact previews

## Current State Analysis

### Rebuild Embeddings Button
- **File**: `rebuild-embeddings-button.tsx` (341 lines)
- **Features**:
  - AlertDialog confirmation
  - Warning about clearing embeddings
  - Automatic document reprocessing trigger
  - PipelineStatusDialog for progress
  - Shows current model/dimension in card variant

**Missing**:
- No data-testid attributes for E2E testing
- No estimated time display
- No document/chunk count preview BEFORE confirming

### Rebuild Knowledge Graph Button  
- **File**: `rebuild-knowledge-graph-button.tsx` (407 lines)
- **Features**:
  - AlertDialog confirmation
  - Warning about clearing graph data
  - Option to also rebuild embeddings
  - Automatic reprocessing trigger
  - PipelineStatusDialog for progress
  - Shows skipped document count

**Missing**:
- No data-testid attributes for E2E testing
- No estimated time display
- No document/chunk count preview BEFORE confirming
- Success response shows counts AFTER confirmation, not before

## API Data Available

From the backend handlers:
```rust
// rebuild_embeddings returns:
RebuildEmbeddingsResponse {
    documents_to_process: i64,
    chunks_to_process: i64,
    vectors_cleared: i64,
    compatibility_warning: Option<String>,
}

// rebuild_knowledge_graph returns:
RebuildKnowledgeGraphResponse {
    nodes_cleared: i64,
    edges_cleared: i64,
    vectors_cleared: i64,
    documents_to_process: i64,
    chunks_to_process: i64,
}
```

## Enhancement Opportunity

We could add a **preview endpoint** or use the **list documents** endpoint to show:
- Total document count
- Total chunk count
- Estimated processing time
- Current model configuration

This would require either:
1. A new `/workspaces/{id}/rebuild-preview` endpoint
2. Or fetching document stats before showing the dialog

## Test IDs Needed

For E2E testing:
- `data-testid="rebuild-embeddings-button"`
- `data-testid="rebuild-embeddings-confirm"`
- `data-testid="rebuild-embeddings-cancel"`
- `data-testid="rebuild-kg-button"`
- `data-testid="rebuild-kg-confirm"`
- `data-testid="rebuild-kg-cancel"`
