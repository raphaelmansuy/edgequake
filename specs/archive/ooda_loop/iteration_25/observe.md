# Iteration 25 – OBSERVE

## Mission Context

**Objective C**: Rebuild Operations Visibility

- Required: Multi-phase progress for Knowledge Graph rebuild
- 3 phases: Clear → Re-extract → Re-embed
- Need document + chunk level visibility

## Current State

### Backend Status

The rebuild KG handler (`workspaces.rs:1760+`) currently:

1. Clears graph storage (nodes_cleared, edges_cleared logged)
2. Optionally clears vectors (vectors_cleared logged)
3. Generates track*id: `rebuild_kg*{timestamp}\_{uuid}`
4. Queues documents for reprocessing

The EnhancedPipelineStatusResponse provides:

- job_name: Contains track_id which reveals operation type
- total_documents, processed_documents: Document-level progress
- history_messages: Activity log

### Frontend Status

PipelineStatusDialog now has:

- ChunkProgressSection: Real-time chunk-level progress
- Document progress bar
- Statistics grid (pending/processing/completed/failed)

### Gap Analysis

**Missing UI Elements:**

1. Phase indicator (which of 3 phases we're in)
2. Clear phase summary (counts of cleared entities/relationships)
3. Visual distinction for rebuild vs normal ingestion

### Data Sources

The job_name contains rebuild type:

- `rebuild_kg_*`: Knowledge Graph rebuild
- `rebuild_embed_*`: Embeddings rebuild
- Other: Normal ingestion

## Observations

1. **Phase Detection**: Can infer phase from job_name prefix
2. **Clear Stats**: Not exposed in API - would need backend change
3. **Progressive Disclosure**: Show phases only for rebuild operations
4. **Visual Hierarchy**: Rebuild dialogs should look different from ingestion
