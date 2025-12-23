# LightRAG Inspiration

## Overview

This document extracts key patterns and features from LightRAG's mature document processing implementation that can inspire EdgeQuake's improvements.

## LightRAG Document Status Model

### Document Status Enum

**File:** `lightrag/api/routers/document_routes.py`

```python
class DocStatus(str, Enum):
    """Document processing status"""
    PENDING = "pending"
    PROCESSING = "processing"
    PREPROCESSED = "preprocessed"  # Additional intermediate state
    PROCESSED = "processed"
    FAILED = "failed"
```

**Key Insight:** LightRAG has a `PREPROCESSED` state for documents that have been chunked but not yet fully indexed. This provides more granular progress visibility.

### Document Status Response

```python
class DocStatusResponse(BaseModel):
    id: str                                  # Document identifier
    content_summary: str                     # Summary/preview of content
    content_length: int                      # Character count
    status: DocStatus                        # Current status
    created_at: str                          # ISO timestamp
    updated_at: str                          # ISO timestamp
    track_id: Optional[str]                  # Links to upload batch
    chunks_count: Optional[int]              # Number of chunks
    error_msg: Optional[str]                 # Detailed error if failed
    metadata: Optional[dict]                 # Additional metadata
    file_path: str                           # Original file path
```

**EdgeQuake Gaps:**

- Missing `content_summary` (preview of document)
- Missing `track_id` (upload batch grouping)
- Missing `error_msg` (detailed error info)
- Missing `file_path` (original location)

## Pipeline Status Response

### The Key Innovation

**File:** `lightrag/api/routers/document_routes.py`

```python
class PipelineStatusResponse(BaseModel):
    """Response model for pipeline status"""

    # Scan status
    autoscanned: bool = False              # Has auto-scan started?

    # Processing status
    busy: bool = False                     # Is pipeline currently busy?
    job_name: str = "Default Job"          # Current job description
    job_start: Optional[str] = None        # When job started (ISO)

    # Batch progress
    docs: int = 0                          # Total documents to process
    batchs: int = 0                        # Total batches
    cur_batch: int = 0                     # Current batch being processed

    # Request status
    request_pending: bool = False          # Is there a pending request?
    cancellation_requested: bool = False   # Has cancellation been requested?

    # Real-time messages
    latest_message: str = ""               # Most recent log message
    history_messages: Optional[List[str]] = None  # Full message history

    # Namespace update status
    update_status: Optional[dict] = None   # Per-namespace update flags
```

**This is the critical feature EdgeQuake lacks!**

### Visual Representation

```
┌───────────────────────────────────────────────────────────────────┐
│                    Pipeline Status Dialog                          │
├───────────────────────────────────────────────────────────────────┤
│                                                                     │
│  Status: ● Busy                    Request Pending: ○              │
│                                                                     │
│  ┌───────────────────────────────────────────────────────────────┐ │
│  │ Job: indexing files                                            │ │
│  │ Start: 2024-01-15 10:30:45                                     │ │
│  │ Progress: Batch 3/5 (45 documents)                             │ │
│  └───────────────────────────────────────────────────────────────┘ │
│                                                                     │
│  Pipeline Messages:                                                 │
│  ┌───────────────────────────────────────────────────────────────┐ │
│  │ [10:30:45] Starting document processing...                     │ │
│  │ [10:30:46] Batch 1: Processing 10 documents                    │ │
│  │ [10:30:52] Batch 1: Extracted 45 entities                      │ │
│  │ [10:30:55] Batch 2: Processing 10 documents                    │ │
│  │ [10:31:02] Batch 2: Extracted 38 entities                      │ │
│  │ [10:31:05] Batch 3: Processing 10 documents                    │ │
│  │ [10:31:08] Batch 3: Extracting entities...  ← latest_message  │ │
│  └───────────────────────────────────────────────────────────────┘ │
│                                                                     │
│                            [Cancel Pipeline]                        │
└───────────────────────────────────────────────────────────────────┘
```

## Track-Based Document Grouping

### Track Status Response

```python
class TrackStatusResponse(BaseModel):
    """Response model for tracking document processing status by track_id"""

    track_id: str                           # The tracking identifier
    documents: List[DocStatusResponse]      # Documents in this batch
    total_count: int                        # Total documents in batch
    status_summary: Dict[str, int]          # Count by status
```

### Example Usage

```json
{
  "track_id": "upload_20250115_103045_abc123",
  "documents": [
    {
      "id": "doc_001",
      "content_summary": "Research paper on machine learning...",
      "status": "processed",
      "track_id": "upload_20250115_103045_abc123"
    },
    {
      "id": "doc_002",
      "content_summary": "Analysis of neural networks...",
      "status": "processing",
      "track_id": "upload_20250115_103045_abc123"
    }
  ],
  "total_count": 2,
  "status_summary": {
    "processed": 1,
    "processing": 1
  }
}
```

**Key Insight:** All documents uploaded together share a `track_id`, making it easy to:

- Show "Uploaded 5 documents" as a group
- Track batch completion
- Retry failed documents from same batch

## Paginated Documents Response

### Status Counts in Response

```python
class PaginatedDocsResponse(BaseModel):
    """Response model for paginated document queries"""

    documents: List[DocStatusResponse]       # Current page items
    pagination: PaginationInfo               # Page info
    status_counts: Dict[str, int]            # Count by status (ALL documents)
```

### Example Response

```json
{
  "documents": [
    /* current page items */
  ],
  "pagination": {
    "page": 1,
    "page_size": 50,
    "total_count": 150,
    "total_pages": 3,
    "has_next": true,
    "has_prev": false
  },
  "status_counts": {
    "pending": 10,
    "processing": 5,
    "preprocessed": 3,
    "processed": 127,
    "failed": 5
  }
}
```

**Key Insight:** Status counts are calculated server-side for ALL documents, not just the current page. This enables accurate filter badges without loading all data.

## Upload/Insert Response

### Tracking ID in Response

```python
class InsertResponse(BaseModel):
    """Response model for document insertion operations"""

    status: Literal["success", "duplicated", "partial_success", "failure"]
    message: str                             # Human-readable result
    track_id: str                            # For monitoring progress
```

**Key Insight:** Every upload returns a `track_id` that can be used to:

1. Poll for batch completion
2. Group documents in the UI
3. Show batch-level progress

## Frontend Pipeline Status Component

**File:** `lightrag_webui/src/components/documents/PipelineStatusDialog.tsx`

### Key UI Features

```tsx
// Auto-scroll to latest message
const historyRef = useRef<HTMLDivElement>(null);

useEffect(() => {
  const container = historyRef.current;
  if (!container || isUserScrolled) return;
  container.scrollTop = container.scrollHeight;
}, [status?.history_messages, isUserScrolled]);

// Detect user scroll (to pause auto-scroll)
const handleScroll = () => {
  const container = historyRef.current;
  if (!container) return;

  const isAtBottom =
    Math.abs(
      container.scrollHeight - container.scrollTop - container.clientHeight
    ) < 1;

  setIsUserScrolled(!isAtBottom);
};
```

### Batch Progress Display

```tsx
<span>
  {t("pipelineStatus.progress")}:
  {status ? `${status.cur_batch}/${status.batchs} batches` : "-"}
</span>
```

### Position Control (Nice Touch!)

```tsx
// User can move dialog to left/center/right
<div className="flex items-center gap-2">
  <Button onClick={() => setPosition("left")}>
    <AlignLeft className="h-4 w-4" />
  </Button>
  <Button onClick={() => setPosition("center")}>
    <AlignCenter className="h-4 w-4" />
  </Button>
  <Button onClick={() => setPosition("right")}>
    <AlignRight className="h-4 w-4" />
  </Button>
</div>
```

**Why this matters:** Users may want to view the graph or documents while watching pipeline progress.

### Cancellation Flow

```tsx
// Two-step cancellation (confirmation required)
const [showCancelConfirm, setShowCancelConfirm] = useState(false);

// Cancel button shows different states
<Button
  variant="destructive"
  disabled={!canCancel}
  onClick={() => setShowCancelConfirm(true)}
  title={
    status?.cancellation_requested
      ? 'Cancellation in progress...'
      : 'Cancel pipeline'
  }
>
  Cancel
</Button>

// Confirmation dialog
<Dialog open={showCancelConfirm}>
  <DialogHeader>
    <DialogTitle>Cancel Pipeline?</DialogTitle>
    <DialogDescription>
      This will stop processing. Already processed documents will be kept.
    </DialogDescription>
  </DialogHeader>
  <div className="flex justify-end gap-3">
    <Button variant="outline" onClick={() => setShowCancelConfirm(false)}>
      Keep Processing
    </Button>
    <Button variant="destructive" onClick={handleConfirmCancel}>
      Yes, Cancel
    </Button>
  </div>
</Dialog>
```

## Frontend Upload Component

**File:** `lightrag_webui/src/components/documents/UploadDocumentsDialog.tsx`

### Sequential Upload with Progress

```tsx
// Create collator for proper file sorting (handles Chinese, etc.)
const collator = new Intl.Collator(["zh-CN", "en"], {
  sensitivity: "accent",
  numeric: true, // "File 10" comes after "File 2"
});
const sortedFiles = [...files].sort((a, b) => collator.compare(a.name, b.name));

// Upload sequentially
for (const file of sortedFiles) {
  setProgresses((prev) => ({ ...prev, [file.name]: 0 }));

  const result = await uploadDocument(file, (percent) => {
    setProgresses((prev) => ({ ...prev, [file.name]: percent }));
  });

  if (result.status === "duplicated") {
    setFileErrors((prev) => ({ ...prev, [file.name]: "Duplicate file" }));
  }
}
```

### Duplicate Detection

```tsx
if (result.status === "duplicated") {
  uploadErrors[file.name] = t("fileUploader.duplicateFile");
}
```

**Key Insight:** LightRAG detects and reports duplicate uploads, preventing wasted processing.

## API TypeScript Types

**File:** `lightrag_webui/src/api/lightrag.ts`

```typescript
export type PipelineStatusResponse = {
  autoscanned: boolean;
  busy: boolean;
  job_name: string;
  job_start?: string;
  docs: number;
  batchs: number;
  cur_batch: number;
  request_pending: boolean;
  cancellation_requested?: boolean;
  latest_message: string;
  history_messages?: string[];
  update_status?: Record<string, any>;
};

export type DocStatusResponse = {
  id: string;
  content_summary: string;
  content_length: number;
  status: DocStatus;
  created_at: string;
  updated_at: string;
  track_id?: string;
  chunks_count?: number;
  error_msg?: string;
  metadata?: Record<string, any>;
  file_path: string;
};

export type TrackStatusResponse = {
  track_id: string;
  documents: DocStatusResponse[];
  total_count: number;
  status_summary: Record<string, number>;
};

export type PaginatedDocsResponse = {
  documents: DocStatusResponse[];
  pagination: PaginationInfo;
  status_counts: Record<string, number>;
};
```

## Key Patterns to Adopt

### 1. Pipeline Status with Batch Progress

```
Current: "5 tasks processing"
Better:  "Processing batch 3/5 (25 documents), 15 entities extracted"
```

### 2. History Messages

```
Current: [No messages]
Better:
  [10:30:45] Starting batch 1...
  [10:30:48] Extracted 12 entities from document_001
  [10:30:52] Batch 1 complete
  [10:30:53] Starting batch 2...
```

### 3. Track-Based Grouping

```
Current: Documents listed individually
Better:
  Upload Batch (5 documents) - 3 processed, 2 pending
    ├── doc_001.txt ✓
    ├── doc_002.txt ✓
    ├── doc_003.txt ✓
    ├── doc_004.txt ⋯ Processing
    └── doc_005.txt ○ Pending
```

### 4. Status Counts in API

```
Current: Client counts from loaded documents
Better:
  API: { status_counts: { pending: 10, processing: 5, completed: 130, failed: 5 } }
  Client: Just display the counts
```

### 5. Detailed Error Messages

```
Current: "Processing failed"
Better:  "Entity extraction failed: Rate limit exceeded. Retry in 30 seconds."
```

## Summary

LightRAG's key innovations:

1. **Batch Progress** - `batchs`, `cur_batch` for overall progress
2. **History Messages** - Real-time log of pipeline activities
3. **Track ID** - Groups related documents together
4. **Status Counts** - Server-side counts for accurate filtering
5. **Content Summary** - Preview of document content
6. **Detailed Errors** - `error_msg` with actionable information
7. **Cancellation Flow** - Two-step confirmation with status feedback

---

**Next:** [Gap Analysis](./04-gap-analysis.md)
