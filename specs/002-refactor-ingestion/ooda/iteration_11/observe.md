# OODA-11: Observe

## Target: ProcessingStatusSummary Component Extraction

### Current Location
- **File**: `edgequake_webui/src/components/documents/document-manager.tsx`
- **Lines**: ~1127-1175 (48 lines)

### Code Analysis

The processing status summary shows:
1. **Spinner + Message**: "Processing N document(s)" or "N document(s) queued"
2. **Stage Details**: Shows stage messages for processing documents
3. **Queue Info**: Shows queued count when also processing
4. **Completed Count**: Shows completed task count
5. **Click CTA**: Opens pipeline dialog for details

### Dependencies Identified
- `Loader2, Clock, CheckCircle` icons
- `pipelineStatus` object from pipeline status query
- `documents` array to filter processing documents
- `isProcessingStatus` utility function
- `setPipelineDialogOpen` setter
- Translation `t()` function

### Props Required
```typescript
interface ProcessingStatusSummaryProps {
  pipelineStatus: PipelineStatus;
  documents: Document[];
  onOpenDetails: () => void;
}
```

### Estimated Savings
- **Lines to extract**: ~48 lines
- **Expected reduction**: ~40 lines (after accounting for component usage)
