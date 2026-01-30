# Iteration 24: Orient

## Gap Analysis

### Objective C Requirements vs Current State

| Requirement                   | Current                                   | Gap                    |
| ----------------------------- | ----------------------------------------- | ---------------------- |
| Show cleared count            | API returns `chunks_to_process`           | ✅ Shown in toast      |
| Chunk-level progress          | ChunkProgressCard only in PipelineMonitor | ❌ Need in dialog      |
| Progress bar with chunk count | Document-level only                       | ❌ Need chunk-level    |
| ETA based on rate             | Document ETA exists                       | ⚠️ Could add chunk ETA |

## Implementation Plan

### Option 1: Add Chunk Progress to PipelineStatusDialog

- Import useChunkProgress hook
- Add section to show active chunk progress
- Reuse existing formatting logic

### Option 2: Create RebuildProgressDialog

- Specialized dialog for rebuild operations
- Multi-phase display
- More complex, higher effort

**Decision**: Option 1 - Enhance PipelineStatusDialog

- Lower effort
- Consistent UI
- Benefits all operations, not just rebuilds

## Changes Required

1. Import useChunkProgress in pipeline-status-dialog.tsx
2. Add ChunkProgressSection component
3. Display when hasActiveProgress is true
4. Show chunk progress, ETA, token counts

## Effort

- Medium: ~60 lines of code
- No backend changes
- Pure frontend enhancement
