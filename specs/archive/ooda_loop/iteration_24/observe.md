# Iteration 24: Observe

## Mission Reference

Re-read: `/specs/001-improve-ingestion-process.md`

**Objective C: Rebuild Operations Visibility**

Required for Rebuild Embeddings:

1. Phase 1: Clear existing embeddings (show count cleared)
2. Phase 2: Re-embed all chunks (show chunk-level progress)
3. Total chunks to re-embed
4. Progress bar with chunk count
5. ETA based on embedding rate

## Current State

### Existing Components

1. **RebuildEmbeddingsButton** (`workspace/rebuild-embeddings-button.tsx`)
   - Shows confirmation dialog with impact preview
   - Opens PipelineStatusDialog for progress
   - Documents to process count: ✅
   - Chunks to process count: ✅ (from API response)

2. **PipelineStatusDialog** (`documents/pipeline-status-dialog.tsx`)
   - Shows document-level progress: ✅
   - Shows ETA: ✅
   - Shows task statistics: ✅
   - Activity log: ✅
   - Cancel button: ✅

3. **ChunkProgressCard** (`pipeline/pipeline-monitor.tsx`)
   - Real-time chunk progress via WebSocket: ✅
   - Per-chunk timing: ✅
   - Token counts: ✅
   - Cost estimates: ✅

### Gap Analysis

The ChunkProgressCard has all the chunk-level visibility we need, but it's only in PipelineMonitor.

For rebuild operations, users see the PipelineStatusDialog which lacks chunk-level progress.

**Solution**: Add chunk progress section to PipelineStatusDialog

## Design

Add to PipelineStatusDialog:

1. Import and use useChunkProgress hook
2. Add collapsible section showing active chunk progress
3. This gives rebuild operations chunk-level visibility

```
┌────────────────────────────────────────────────────────────────┐
│ Pipeline Status                                     ● Active   │
├────────────────────────────────────────────────────────────────┤
│ Document Progress: [████████░░░░░░░░] 8/25 (32%)              │
│ ETA: ~5 min                                                    │
├────────────────────────────────────────────────────────────────┤
│ CHUNK PROGRESS (Live)                                          │
│ • doc-123: Chunk 18/32 (56%) - ETA 45s                        │
│   Current: "Section 3.2: Methodology..."                       │
├────────────────────────────────────────────────────────────────┤
│ [Pending: 12] [Processing: 3] [Completed: 5] [Failed: 0]      │
└────────────────────────────────────────────────────────────────┘
```
