# Observe - Iteration 141

## Focus: Rebuild Document Extraction + Embedding Works (Item 5)

Verifying SPEC-032 requirement:
- **Item 5**: Rebuild document extraction + embedding works, processing information displayed like first-time processing

## Investigation

### Rebuild Embeddings Button

**File**: `edgequake_webui/src/components/workspace/rebuild-embeddings-button.tsx`
- Lines: 337
- Annotations:
  - `@implements SPEC-032: Vector database rebuild on embedding model change`
  - `@implements SPEC-032 Focus Area 5: Rebuild with progress display`

### Rebuild Flow

1. **Step 1 - Clear Embeddings** (lines 129-161):
   - `rebuildEmbeddings(selectedWorkspaceId, { force: true })`
   - Shows compatibility warning if chunk size exceeds model context (REQ-25)
   - Triggers reprocessing automatically

2. **Step 2 - Queue Documents** (lines 85-126):
   - `reprocessAllDocuments(selectedWorkspaceId, { include_completed: true })`
   - Opens `PipelineStatusDialog` to show progress

### Pipeline Status Dialog

**File**: `edgequake_webui/src/components/documents/pipeline-status-dialog.tsx`
- Lines: 326
- Features:
  - Real-time polling every 2 seconds
  - Progress bar with percentage
  - Message history with timestamps
  - Status badges (processing, completed, error)
  - Cancel button with confirmation

### Processing Display

The dialog shows:
- Total documents / processed count
- Progress percentage
- Individual document status
- Processing messages with timestamps
- Entity/chunk counts when available

## Findings

Item 5 is fully implemented:
- ✅ Rebuild embeddings triggers full reprocessing
- ✅ PipelineStatusDialog shows real-time progress
- ✅ Same UI as first-time processing
- ✅ Progress messages with timestamps
- ✅ Compatibility warnings displayed
