# OODA Iteration 04 - OBSERVE

## Issue

**High Priority Issue #4**: DocumentManager SRP Violation (1826 lines)

From mission spec:

> Split into: DocumentUploadZone, DocumentList, DocumentFilters, DocumentBatchActions, DocumentDetailPanel
> Create: useDocumentWebSocket hook, useStuckDetection hook

## Data Gathered

### File Size Analysis

```bash
$ wc -l document-manager.tsx
1826 document-manager.tsx
```

Target: <300 lines per component (from success criteria)

### Component Structure

| Line Range | Section                | Responsibility                     |
| ---------- | ---------------------- | ---------------------------------- |
| 1-175      | Imports + helpers      | File type icons, text highlighting |
| 176-275    | State declarations     | 15+ useState calls                 |
| 276-328    | WebSocket subscription | Auto-subscribe to processing docs  |
| 329-365    | Stuck detection        | Warn about stale documents         |
| 369-660    | File upload logic      | Upload, progress, error handling   |
| 661-800    | Mutations              | Delete, reprocess, cancel          |
| 784-900    | Dropzone config        | react-dropzone setup               |
| 879-1000   | Selection handlers     | Select all, bulk actions           |
| 1000-1100  | Navigation handlers    | View details, graph, PDF           |
| 1100-1826  | JSX rendering          | Main UI structure                  |

### State Variables (15+)

```typescript
// Selection state
(selectedIds, setSelectedIds);
(selectedDocument, setSelectedDocument);
(previewPanelOpen, setPreviewPanelOpen);

// Dialogs
(viewerDialogOpen, viewerPdfId);
pipelineDialogOpen;

// Filters
(searchQuery, currentPage, pageSize);
(statusFilter, sortField, sortDirection);

// Upload
(uploadingFiles, isUploading);
```

### WebSocket Logic (Lines 276-365)

Three tightly coupled useEffect hooks:

1. **Subscribe to track IDs** (lines 281-305) - subscribes for processing docs
2. **Progress event listener** (lines 308-326) - invalidates query on updates
3. **Stuck detection** (lines 330-365) - warns about stale documents

### Extraction Candidates

| Component/Hook       | Lines | Dependencies                       | Complexity |
| -------------------- | ----- | ---------------------------------- | ---------- |
| useStuckDetection    | ~30   | data.items, isProcessingStatus     | Low        |
| useDocumentWebSocket | ~50   | connected, data.items, queryClient | Medium     |
| DocumentUploadZone   | ~300  | upload logic, dropzone             | High       |
| DocumentFilters      | ~100  | filter state, handlers             | Medium     |
| DocumentBatchActions | ~100  | selection state, mutations         | Medium     |

### Incremental Strategy

Given 12-hour estimate, I'll extract one piece per iteration:

1. **Iteration 04**: Extract `useStuckDetection` hook (simplest, isolated)
2. **Iteration 05**: Extract `useDocumentWebSocket` hook
3. **Iteration 06+**: Continue with components

### useStuckDetection Analysis

**Current code** (lines 330-365):

```typescript
useEffect(() => {
  if (!data?.items) return;

  const checkStuckDocuments = () => {
    const processingDocs = data.items.filter(
      (doc: Document) => doc.track_id && isProcessingStatus(doc.status as any),
    );

    processingDocs.forEach((doc: Document) => {
      // Check if no update in 30 seconds
      // Log warning with document details
    });
  };

  checkStuckDocuments();
  const interval = setInterval(checkStuckDocuments, 30000);
  return () => clearInterval(interval);
}, [data?.items]);
```

**Can be extracted to**:

```typescript
// hooks/use-stuck-detection.ts
export function useStuckDetection(documents: Document[], options?: {
  timeout?: number;
  onStuck?: (doc: Document) => void;
}) { ... }
```

## Key Observations

1. **Multiple responsibilities** - upload, filters, selection, mutations, WebSocket, rendering
2. **State explosion** - 15+ useState calls make component hard to reason about
3. **Coupled effects** - WebSocket effects depend on each other's side effects
4. **Low-hanging fruit** - stuck detection is completely isolated, easy to extract
5. **Incremental approach** - extracting hooks first creates foundation for component splits
