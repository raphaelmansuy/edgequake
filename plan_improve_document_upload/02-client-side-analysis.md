# Client-Side Analysis

## Overview

This document analyzes EdgeQuake's current frontend implementation for document upload and management, focusing on UX patterns, state management, and user feedback mechanisms.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    EdgeQuake WebUI (Next.js)                     │
│                      /edgequake_webui                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │                  Document Manager Page                       │ │
│  │              /components/documents/document-manager.tsx      │ │
│  │                                                              │ │
│  │  ┌─────────────────┐  ┌─────────────────┐  ┌──────────────┐ │ │
│  │  │   Dropzone      │  │  Upload Status  │  │  Filters     │ │ │
│  │  │   Component     │  │  Panel          │  │  Component   │ │ │
│  │  └─────────────────┘  └─────────────────┘  └──────────────┘ │ │
│  │                                                              │ │
│  │  ┌─────────────────────────────────────────────────────────┐│ │
│  │  │                  Documents Table                         ││ │
│  │  │  - Title, Status, Entities, Created                     ││ │
│  │  │  - Actions: Reprocess, Delete                           ││ │
│  │  └─────────────────────────────────────────────────────────┘│ │
│  │                                                              │ │
│  │  ┌─────────────────┐  ┌─────────────────────────────────┐   │ │
│  │  │ Pipeline Status │  │  Pagination Controls            │   │ │
│  │  │ Dialog          │  └─────────────────────────────────┘   │ │
│  │  └─────────────────┘                                        │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                   │
├─────────────────────────────────────────────────────────────────┤
│                        API Layer                                 │
│                   /lib/api/edgequake.ts                         │
│                                                                   │
│  ┌──────────────┐  ┌────────────────┐  ┌─────────────────────┐  │
│  │ uploadDoc()  │  │ getDocuments() │  │ getPipelineStatus() │  │
│  │ deleteDoc()  │  │ getTaskStatus()│  │ cancelPipeline()    │  │
│  └──────────────┘  └────────────────┘  └─────────────────────┘  │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

## Upload UX Flow

### File Drag & Drop

**File:** `edgequake_webui/src/components/documents/document-manager.tsx`

```typescript
// Dropzone configuration
const { getRootProps, getInputProps, isDragActive } = useDropzone({
  onDrop,
  accept: {
    'text/plain': ['.txt'],
    'text/markdown': ['.md'],
    'application/json': ['.json'],
  },
});
```

### Upload Progress State

```typescript
interface UploadingFile {
  file: File;
  progress: number;
  status: 'pending' | 'reading' | 'uploading' | 'extracting' | 'success' | 'error';
  error?: string;
  phase?: string; // Human-readable phase description
}

const [uploadingFiles, setUploadingFiles] = useState<UploadingFile[]>([]);
const [isUploading, setIsUploading] = useState(false);
```

### Upload Progress Phases

```
┌─────────┐     ┌─────────┐     ┌───────────┐     ┌────────────┐     ┌─────────┐
│ Pending │────▶│ Reading │────▶│ Uploading │────▶│ Extracting │────▶│ Success │
│  (0%)   │     │  (10%)  │     │   (40%)   │     │   (80%)    │     │ (100%)  │
└─────────┘     └─────────┘     └───────────┘     └────────────┘     └─────────┘
     │                                                                    │
     └──────────────────────────────────────────────────────────────────▶│
                              (on error: Error state with message)        │
```

### Phase Visual Indicators

```tsx
// Phase Legend Component (shown during upload)
<div className="flex items-center gap-4 text-xs text-muted-foreground">
  <span className="flex items-center gap-1.5">
    <span className="h-2 w-2 rounded-full bg-amber-500" />
    Reading
  </span>
  <span className="text-muted-foreground/50">→</span>
  <span className="flex items-center gap-1.5">
    <span className="h-2 w-2 rounded-full bg-blue-500" />
    Uploading
  </span>
  <span className="text-muted-foreground/50">→</span>
  <span className="flex items-center gap-1.5">
    <span className="h-2 w-2 rounded-full bg-purple-500" />
    Extracting
  </span>
  <span className="text-muted-foreground/50">→</span>
  <span className="flex items-center gap-1.5">
    <span className="h-2 w-2 rounded-full bg-green-500" />
    Done
  </span>
</div>
```

### File Status Icons

| Status | Icon | Color | Animation |
|--------|------|-------|-----------|
| pending | Clock | muted | none |
| reading | FileSearch | amber-500 | pulse |
| uploading | Upload | blue-500 | bounce |
| extracting | Sparkles | purple-500 | pulse |
| success | CheckCircle | green-500 | none |
| error | XCircle | red-500 | none |

### Upload Handler Implementation

```typescript
const handleFilesUpload = useCallback(async (files: File[]) => {
  if (files.length === 0) return;
  
  setIsUploading(true);
  
  // Initialize upload state for all files
  const initialFiles: UploadingFile[] = files.map((file) => ({
    file,
    progress: 0,
    status: 'pending',
    phase: 'Waiting...',
  }));
  setUploadingFiles(initialFiles);

  // Show loading toast
  const toastId = toast.loading(`Uploading ${files.length} file(s)...`, {
    duration: Infinity
  });

  let successCount = 0;
  let errorCount = 0;

  // Process files SEQUENTIALLY for better feedback
  for (let i = 0; i < files.length; i++) {
    const file = files[i];
    
    // Phase 1: Reading file
    setUploadingFiles((prev) =>
      prev.map((f, idx) =>
        idx === i ? { ...f, status: 'reading', progress: 10, phase: 'Reading file...' } : f
      )
    );

    try {
      const text = await file.text();
      
      // Phase 2: Uploading to server
      setUploadingFiles((prev) =>
        prev.map((f, idx) =>
          idx === i ? { ...f, status: 'uploading', progress: 40, phase: 'Uploading...' } : f
        )
      );

      const response = await uploadDocument({ 
        content: text, 
        title: file.name,
        async_processing: true,
      });
      
      // Phase 3: Extraction queued
      setUploadingFiles((prev) =>
        prev.map((f, idx) =>
          idx === i ? { 
            ...f, 
            status: 'extracting', 
            progress: 80, 
            phase: response.task_id 
              ? `Queued (Task: ${response.task_id.slice(0, 8)})`
              : 'Processing...',
          } : f
        )
      );
      
      // Mark as complete
      setUploadingFiles((prev) =>
        prev.map((f, idx) =>
          idx === i ? { ...f, status: 'success', progress: 100, phase: 'Complete!' } : f
        )
      );
      
      successCount++;
    } catch (error) {
      setUploadingFiles((prev) =>
        prev.map((f, idx) =>
          idx === i ? { ...f, status: 'error', progress: 100, error: error.message } : f
        )
      );
      errorCount++;
    }
  }

  // Update toast with final result
  if (errorCount === 0) {
    toast.success(`Successfully uploaded ${successCount} file(s)`, { id: toastId });
  } else if (successCount === 0) {
    toast.error(`All ${errorCount} file(s) failed`, { id: toastId });
  } else {
    toast.warning(`Uploaded ${successCount}, ${errorCount} failed`, { id: toastId });
  }

  // Refresh documents list
  queryClient.invalidateQueries({ queryKey: ['documents'] });
  setIsUploading(false);

  // Clear upload list after delay
  setTimeout(() => setUploadingFiles([]), 3000);
}, [queryClient, t]);
```

## Document List

### Query Configuration

```typescript
const { data, isLoading, isError, error, refetch } = useQuery({
  queryKey: ['documents', currentPage, pageSize, statusFilter],
  queryFn: () => getDocuments({ 
    page: currentPage, 
    page_size: pageSize,
    status: statusFilter === 'all' ? undefined : statusFilter,
  }),
  refetchInterval: 5000, // Poll for status updates every 5 seconds
});
```

### Status Filter Component

**File:** `document-filters.tsx`

```typescript
export type DocStatus = 'all' | 'pending' | 'processing' | 'completed' | 'failed';

interface DocumentFiltersProps {
  status: DocStatus;
  onStatusChange: (status: DocStatus) => void;
  statusCounts: Record<DocStatus, number>;
  // ... sorting props
}
```

### Status Counts Calculation (Client-Side)

```typescript
// Currently calculated client-side, not from API
const statusCounts: Record<DocStatus, number> = {
  all: allDocuments.length,
  pending: allDocuments.filter((d) => d.status === 'pending').length,
  processing: allDocuments.filter((d) => d.status === 'processing').length,
  completed: allDocuments.filter((d) => !d.status || d.status === 'completed').length,
  failed: allDocuments.filter((d) => d.status === 'failed').length,
};
```

**Issue:** This requires loading ALL documents to get accurate counts, which doesn't scale.

### Document Table Columns

| Column | Source | Display |
|--------|--------|---------|
| Title | `doc.title \|\| doc.file_name \|\| doc.id.slice(0,8)` | Text |
| Status | `doc.status \|\| 'completed'` | Badge with icon |
| Entities | `doc.entity_count ?? doc.chunk_count ?? '-'` | Number |
| Created | `doc.created_at` | Relative time |
| Actions | - | Dropdown menu |

### Status Badge Component

```typescript
const statusConfig = {
  pending: { icon: Clock, color: 'bg-yellow-500', label: 'Pending', animate: false },
  processing: { icon: Loader2, color: 'bg-blue-500', label: 'Processing', animate: true },
  completed: { icon: CheckCircle, color: 'bg-green-500', label: 'Completed', animate: false },
  failed: { icon: XCircle, color: 'bg-red-500', label: 'Failed', animate: false },
};

function StatusBadge({ status }: { status: DocumentStatus }) {
  const config = statusConfig[status];
  const Icon = config.icon;

  return (
    <Badge variant="outline" className="gap-1">
      <Icon className={`h-3 w-3 ${config.animate ? 'animate-spin' : ''}`} />
      {config.label}
    </Badge>
  );
}
```

## Pipeline Status Dialog

**File:** `pipeline-status-dialog.tsx`

### Status Query

```typescript
const { data, isLoading } = useQuery({
  queryKey: ['pipeline-status'],
  queryFn: getPipelineStatus,
  refetchInterval: open ? 2000 : false, // Poll every 2s when open
  enabled: open,
});
```

### Dialog Content

```tsx
<Dialog>
  <DialogContent>
    {/* Statistics Grid */}
    <div className="grid grid-cols-2 gap-4">
      <div className="p-2 bg-muted rounded">
        <p>Processing</p>
        <p className="text-xl font-bold">{data.running_tasks}</p>
      </div>
      <div className="p-2 bg-muted rounded">
        <p>Queued</p>
        <p className="text-xl font-bold">{data.queued_tasks}</p>
      </div>
      <div className="p-2 bg-muted rounded">
        <p>Completed</p>
        <p className="text-xl font-bold text-green-600">{data.completed_tasks}</p>
      </div>
      <div className="p-2 bg-muted rounded">
        <p>Failed</p>
        <p className="text-xl font-bold text-red-600">{data.failed_tasks}</p>
      </div>
    </div>

    {/* Recent Tasks */}
    {data.tasks?.length > 0 && (
      <ScrollArea className="h-32 rounded-md border">
        {data.tasks.slice(0, 10).map((task) => (
          <div key={task.track_id} className="flex items-center justify-between">
            <span>{task.track_id.slice(0, 8)}...</span>
            <span className={statusColorClass}>{task.status}</span>
          </div>
        ))}
      </ScrollArea>
    )}

    {/* Cancel Button */}
    <Button variant="destructive" onClick={handleCancel}>
      Cancel Pipeline
    </Button>
  </DialogContent>
</Dialog>
```

### Pipeline Status API Composition

```typescript
// getPipelineStatus() derives status from tasks list
export async function getPipelineStatus(): Promise<PipelineStatus> {
  const result = await getTasksList({ page_size: 50 });
  
  return {
    is_busy: result.statistics.processing > 0,
    running_tasks: result.statistics.processing,
    queued_tasks: result.statistics.pending,
    completed_tasks: result.statistics.indexed,
    failed_tasks: result.statistics.failed,
    tasks: result.tasks,
  };
}
```

## Polling Strategy

| Resource | Interval | Condition |
|----------|----------|-----------|
| Documents | 5000ms | Always |
| Pipeline Status | 2000ms | Dialog open |
| Pipeline Status (header) | 5000ms | Always |

## API Integration

**File:** `edgequake_webui/src/lib/api/edgequake.ts`

### Document API Functions

```typescript
// Upload document (JSON body)
export async function uploadDocument(
  data: UploadDocumentRequest
): Promise<UploadDocumentResponse> {
  return api.post<UploadDocumentResponse>("/documents", data);
}

// Upload file (multipart form)
export async function uploadFile(file: File): Promise<UploadDocumentResponse> {
  const formData = new FormData();
  formData.append("file", file);
  return api.post<UploadDocumentResponse>("/documents/upload", formData);
}

// List documents with pagination
export async function getDocuments(
  params?: PaginationParams & { status?: string }
): Promise<PaginatedResponse<Document>> {
  // ...
}

// Delete single document
export async function deleteDocument(documentId: string): Promise<void>;

// Delete all documents
export async function deleteAllDocuments(): Promise<{ deleted_count: number }>;

// Reprocess document
export async function reprocessDocument(documentId: string): Promise<UploadDocumentResponse>;
```

### Task API Functions

```typescript
// Get task list with statistics
export async function getTasksList(params?: {
  status?: string;
  task_type?: string;
  page?: number;
  page_size?: number;
}): Promise<TaskListResponse>;

// Get single task status
export async function getTaskStatus(taskId: string): Promise<TaskResponse>;

// Cancel single task
export async function cancelTask(taskId: string): Promise<void>;

// Retry failed task
export async function retryTask(taskId: string): Promise<TaskResponse>;
```

## Current UX Strengths

1. **Phase-Based Progress** - Clear visual indication of upload phases
2. **Color-Coded Status** - Consistent color scheme (amber/blue/purple/green)
3. **Animated Icons** - Pulse/bounce animations for active states
4. **Phase Legend** - Helps users understand the workflow
5. **Toast Notifications** - Immediate feedback with duration control
6. **Sequential Upload** - Prevents overwhelming the server

## UX Gaps

### 1. No Real-Time Pipeline Messages
- Only shows task IDs and statuses
- No logs or messages from processing pipeline
- Users can't see what's happening during extraction

### 2. No Batch Progress
- Can't see "Processing document 3 of 10"
- No overall batch completion percentage
- No estimated time remaining

### 3. Status Counts Require Full Data
- `statusCounts` calculated client-side
- Requires loading all documents
- Doesn't scale with large document sets

### 4. No Document Grouping
- Can't see which documents were uploaded together
- No batch/track_id correlation
- Hard to track multi-file uploads

### 5. Limited Error Details
- Only shows generic error messages
- No step-level failure information
- No retry suggestions

## Summary

EdgeQuake's frontend has a good foundation:
- ✅ Phased upload progress with visual indicators
- ✅ Status badges with animations
- ✅ Pipeline status dialog with cancel
- ✅ Polling for status updates

Key improvements needed:
- ❌ Real-time pipeline messages (like LightRAG's `history_messages`)
- ❌ Batch progress (documents/total, batches/total)
- ❌ Server-side status counts
- ❌ Track-based document grouping
- ❌ Richer error details

---

**Next:** [LightRAG Inspiration](./03-lightrag-inspiration.md)
