# Iteration 34: Orient

## API Consistency Matrix

| Feature            | Frontend Function         | Backend Route                          | Status |
| ------------------ | ------------------------- | -------------------------------------- | ------ |
| Queue metrics      | getQueueMetrics           | /pipeline/queue-metrics                | ✅     |
| Pipeline status    | getEnhancedPipelineStatus | Computed                               | ✅     |
| Tasks list         | getTasksList              | /tasks/list                            | ✅     |
| Rebuild embeddings | rebuildEmbeddings         | /workspace/:id/rebuild-embeddings      | ✅     |
| Rebuild KG         | rebuildKnowledgeGraph     | /workspace/:id/rebuild-knowledge-graph | ✅     |
| Reprocess all      | reprocessAllDocuments     | /documents/reprocess-all               | ✅     |

## Gap Analysis

No API gaps found. All endpoints required by the pipeline monitoring features exist in the backend.

## Fallback Strategy

The frontend implements graceful degradation:

- `getQueueMetrics` returns default metrics if endpoint fails
- `getEnhancedPipelineStatus` computes status from available data

This ensures the UI remains functional even during backend updates.
