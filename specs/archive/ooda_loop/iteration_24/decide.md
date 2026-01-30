# Iteration 24: Decide

## Action Plan

### Step 1: Import useChunkProgress hook

File: `edgequake_webui/src/components/documents/pipeline-status-dialog.tsx`

### Step 2: Add ChunkProgressSection component

- Display active chunk progress
- Show chunk index, total, percentage
- Show current chunk preview
- Show ETA per document

### Step 3: Integrate into PipelineStatusDialog

- Add section between document progress and statistics
- Only show when there's active chunk progress

### Step 4: Validate

- TypeScript compilation

## Priority

- HIGH: Core Objective C requirement
- No backend changes needed
