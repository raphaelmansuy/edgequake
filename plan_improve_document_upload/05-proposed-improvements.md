# Proposed Improvements

## Overview

This document outlines the specific improvements to be made to EdgeQuake's document upload and processing pipeline, with detailed API designs and UX specifications.

## Enhanced API Design

### 1. Enhanced Pipeline Status Endpoint

**Endpoint:** `GET /api/v1/pipeline/status`

```rust
#[derive(Serialize)]
pub struct PipelineStatusResponse {
    // Current state
    pub is_busy: bool,
    pub job_name: Option<String>,
    pub job_start: Option<String>,  // ISO timestamp
    
    // Batch progress
    pub total_documents: u32,
    pub processed_documents: u32,
    pub total_batches: u32,
    pub current_batch: u32,
    
    // Task statistics
    pub pending_tasks: u32,
    pub processing_tasks: u32,
    pub completed_tasks: u32,
    pub failed_tasks: u32,
    
    // Real-time messages
    pub latest_message: Option<String>,
    pub history_messages: Vec<PipelineMessage>,
    
    // Control state
    pub cancellation_requested: bool,
}

#[derive(Serialize)]
pub struct PipelineMessage {
    pub timestamp: String,  // ISO timestamp
    pub level: String,      // "info", "warn", "error"
    pub message: String,
}
```

**Example Response:**
```json
{
  "is_busy": true,
  "job_name": "Processing 10 documents",
  "job_start": "2024-01-15T10:30:45Z",
  "total_documents": 10,
  "processed_documents": 4,
  "total_batches": 3,
  "current_batch": 2,
  "pending_tasks": 0,
  "processing_tasks": 2,
  "completed_tasks": 4,
  "failed_tasks": 0,
  "latest_message": "Extracting entities from document_005...",
  "history_messages": [
    { "timestamp": "2024-01-15T10:30:45Z", "level": "info", "message": "Starting batch processing for 10 documents" },
    { "timestamp": "2024-01-15T10:30:46Z", "level": "info", "message": "Batch 1: Processing documents 1-4" },
    { "timestamp": "2024-01-15T10:30:50Z", "level": "info", "message": "Batch 1: Extracted 45 entities" },
    { "timestamp": "2024-01-15T10:30:51Z", "level": "info", "message": "Batch 2: Processing documents 5-8" }
  ],
  "cancellation_requested": false
}
```

### 2. Enhanced Document Upload Response

**Endpoint:** `POST /api/v1/documents`

```rust
#[derive(Deserialize)]
pub struct UploadDocumentRequest {
    pub content: String,
    pub title: Option<String>,
    pub file_path: Option<String>,     // NEW: Original file path
    pub metadata: Option<Value>,
    pub async_processing: Option<bool>,
    pub track_id: Option<String>,      // NEW: Client can provide
}

#[derive(Serialize)]
pub struct UploadDocumentResponse {
    pub document_id: String,
    pub status: String,
    pub task_id: Option<String>,
    pub track_id: String,              // NEW: Always returned
    pub duplicate_of: Option<String>,  // NEW: If duplicate detected
    pub chunk_count: Option<usize>,
    pub entity_count: Option<usize>,
    pub relationship_count: Option<usize>,
}
```

**Example Responses:**

Success:
```json
{
  "document_id": "doc_abc123",
  "status": "pending",
  "task_id": "task_xyz789",
  "track_id": "upload_20240115_103045_batch1",
  "duplicate_of": null,
  "chunk_count": null,
  "entity_count": null
}
```

Duplicate:
```json
{
  "document_id": "doc_abc123",
  "status": "duplicated",
  "task_id": null,
  "track_id": "upload_20240115_103045_batch1",
  "duplicate_of": "doc_existing789",
  "chunk_count": null,
  "entity_count": null
}
```

### 3. Enhanced Document List Response

**Endpoint:** `GET /api/v1/documents`

```rust
#[derive(Serialize)]
pub struct ListDocumentsResponse {
    pub documents: Vec<DocumentSummary>,
    pub pagination: PaginationInfo,
    pub status_counts: StatusCounts,  // NEW
}

#[derive(Serialize)]
pub struct DocumentSummary {
    pub id: String,
    pub title: Option<String>,
    pub file_name: Option<String>,
    pub file_path: Option<String>,        // NEW
    pub content_summary: Option<String>,  // NEW: First 200 chars
    pub content_length: Option<usize>,    // NEW: Total chars
    pub chunk_count: usize,
    pub entity_count: Option<usize>,
    pub status: String,
    pub error_message: Option<String>,    // NEW: If failed
    pub track_id: Option<String>,         // NEW
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Serialize)]
pub struct StatusCounts {
    pub pending: usize,
    pub processing: usize,
    pub completed: usize,
    pub failed: usize,
}
```

**Example Response:**
```json
{
  "documents": [
    {
      "id": "doc_abc123",
      "title": "Machine Learning Research",
      "file_name": "ml_research.pdf",
      "file_path": "/Users/alice/Documents/ml_research.pdf",
      "content_summary": "This paper presents a novel approach to neural network optimization using...",
      "content_length": 15240,
      "chunk_count": 12,
      "entity_count": 45,
      "status": "completed",
      "error_message": null,
      "track_id": "upload_20240115_103045_batch1",
      "created_at": "2024-01-15T10:30:45Z",
      "updated_at": "2024-01-15T10:35:30Z"
    }
  ],
  "pagination": {
    "total": 150,
    "page": 1,
    "page_size": 20,
    "total_pages": 8
  },
  "status_counts": {
    "pending": 10,
    "processing": 5,
    "completed": 130,
    "failed": 5
  }
}
```

### 4. Track Status Endpoint (New)

**Endpoint:** `GET /api/v1/documents/track/:track_id`

```rust
#[derive(Serialize)]
pub struct TrackStatusResponse {
    pub track_id: String,
    pub created_at: String,
    pub documents: Vec<DocumentSummary>,
    pub total_count: usize,
    pub status_summary: StatusCounts,
}
```

**Example Response:**
```json
{
  "track_id": "upload_20240115_103045_batch1",
  "created_at": "2024-01-15T10:30:45Z",
  "documents": [
    { "id": "doc_001", "status": "completed", "title": "Doc 1" },
    { "id": "doc_002", "status": "completed", "title": "Doc 2" },
    { "id": "doc_003", "status": "processing", "title": "Doc 3" },
    { "id": "doc_004", "status": "pending", "title": "Doc 4" },
    { "id": "doc_005", "status": "pending", "title": "Doc 5" }
  ],
  "total_count": 5,
  "status_summary": {
    "pending": 2,
    "processing": 1,
    "completed": 2,
    "failed": 0
  }
}
```

### 5. Enhanced Task Response

**Endpoint:** `GET /api/v1/tasks/:track_id`

```rust
#[derive(Serialize)]
pub struct TaskResponse {
    pub track_id: String,
    pub task_type: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub retry_count: u32,
    pub max_retries: u32,
    
    // Progress
    pub progress: Option<TaskProgress>,
    
    // Enhanced error info
    pub error: Option<TaskError>,  // NEW: Detailed error
    
    // Results
    pub result: Option<Value>,
    pub metadata: Option<Value>,
}

#[derive(Serialize)]
pub struct TaskError {
    pub message: String,
    pub step: String,           // "chunking", "embedding", "extraction", "indexing"
    pub reason: String,         // More specific reason
    pub suggestion: String,     // How to fix
    pub retryable: bool,        // Can this be retried?
}
```

**Example Error:**
```json
{
  "track_id": "task_xyz789",
  "status": "failed",
  "error": {
    "message": "Entity extraction failed",
    "step": "extraction",
    "reason": "OpenAI API rate limit exceeded",
    "suggestion": "Wait 30 seconds and retry, or reduce batch size",
    "retryable": true
  }
}
```

## Pipeline Message System

### Backend Implementation

```rust
// In edgequake-tasks/src/types.rs

pub struct PipelineState {
    pub is_busy: bool,
    pub job_name: Option<String>,
    pub job_start: Option<DateTime<Utc>>,
    pub total_documents: u32,
    pub processed_documents: u32,
    pub current_batch: u32,
    pub total_batches: u32,
    pub messages: Vec<PipelineMessage>,
    pub cancellation_requested: bool,
}

impl PipelineState {
    pub fn log(&mut self, level: &str, message: String) {
        self.messages.push(PipelineMessage {
            timestamp: Utc::now().to_rfc3339(),
            level: level.to_string(),
            message: message.clone(),
        });
        // Keep last 100 messages
        if self.messages.len() > 100 {
            self.messages.remove(0);
        }
    }
    
    pub fn start_job(&mut self, name: String, total_docs: u32, batches: u32) {
        self.is_busy = true;
        self.job_name = Some(name.clone());
        self.job_start = Some(Utc::now());
        self.total_documents = total_docs;
        self.processed_documents = 0;
        self.current_batch = 0;
        self.total_batches = batches;
        self.log("info", format!("Starting job: {}", name));
    }
    
    pub fn advance_batch(&mut self) {
        self.current_batch += 1;
        self.log("info", format!(
            "Processing batch {}/{}", 
            self.current_batch, 
            self.total_batches
        ));
    }
    
    pub fn document_processed(&mut self, doc_id: &str, entity_count: usize) {
        self.processed_documents += 1;
        self.log("info", format!(
            "Processed {} ({} entities) - {}/{} complete",
            doc_id, entity_count,
            self.processed_documents, self.total_documents
        ));
    }
    
    pub fn finish_job(&mut self) {
        self.log("info", format!(
            "Job complete: {} documents processed",
            self.processed_documents
        ));
        self.is_busy = false;
    }
}
```

### Worker Pool Integration

```rust
// In edgequake-tasks/src/worker.rs

impl WorkerPool {
    pub async fn process_batch(&self, tasks: Vec<Task>) -> Result<()> {
        let batch_size = 4;
        let total_batches = (tasks.len() + batch_size - 1) / batch_size;
        
        // Start job
        self.pipeline_state.lock().await.start_job(
            format!("Processing {} documents", tasks.len()),
            tasks.len() as u32,
            total_batches as u32,
        );
        
        for (batch_idx, batch) in tasks.chunks(batch_size).enumerate() {
            self.pipeline_state.lock().await.advance_batch();
            
            for task in batch {
                // Log processing start
                self.pipeline_state.lock().await.log(
                    "info",
                    format!("Extracting entities from {}...", task.track_id),
                );
                
                // Process task
                let result = self.process_single_task(task).await;
                
                match result {
                    Ok(entities) => {
                        self.pipeline_state.lock().await.document_processed(
                            &task.track_id,
                            entities.len(),
                        );
                    }
                    Err(e) => {
                        self.pipeline_state.lock().await.log(
                            "error",
                            format!("Failed {}: {}", task.track_id, e),
                        );
                    }
                }
            }
        }
        
        self.pipeline_state.lock().await.finish_job();
        Ok(())
    }
}
```

## Enhanced Frontend UX

### 1. Improved Pipeline Status Dialog

```tsx
// edgequake_webui/src/components/documents/pipeline-status-dialog.tsx

interface EnhancedPipelineStatus {
  is_busy: boolean;
  job_name?: string;
  job_start?: string;
  total_documents: number;
  processed_documents: number;
  total_batches: number;
  current_batch: number;
  latest_message?: string;
  history_messages: PipelineMessage[];
  cancellation_requested: boolean;
  // Statistics
  pending_tasks: number;
  processing_tasks: number;
  completed_tasks: number;
  failed_tasks: number;
}

interface PipelineMessage {
  timestamp: string;
  level: 'info' | 'warn' | 'error';
  message: string;
}
```

### New Dialog Layout

```
┌────────────────────────────────────────────────────────────────┐
│ Pipeline Status                              [◀] [⬛] [▶] [✕]  │
├────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ ● Processing 10 documents                     4/10 (40%) │   │
│  │   Started: 10:30:45 AM  •  Batch 2/3                     │   │
│  │   ████████████░░░░░░░░░░░░░░░░░░░░                       │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ Statistics                                                │  │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐         │  │
│  │  │Pending  │ │Running  │ │Complete │ │Failed   │         │  │
│  │  │   0     │ │   2     │ │   4     │ │   0     │         │  │
│  │  └─────────┘ └─────────┘ └─────────┘ └─────────┘         │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                 │
│  Messages:                                                      │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ [10:30:45] Starting batch processing for 10 documents   │   │
│  │ [10:30:46] Batch 1: Processing documents 1-4            │   │
│  │ [10:30:50] ✓ Processed doc_001 (12 entities)            │   │
│  │ [10:30:52] ✓ Processed doc_002 (8 entities)             │   │
│  │ [10:30:55] ✓ Processed doc_003 (15 entities)            │   │
│  │ [10:30:58] ✓ Processed doc_004 (10 entities)            │   │
│  │ [10:31:00] Batch 2: Processing documents 5-8            │   │
│  │ [10:31:02] Extracting entities from doc_005...          │   │
│  │                                          ▼ Auto-scroll   │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
│                              [Cancel Pipeline]                  │
└────────────────────────────────────────────────────────────────┘
```

### Implementation

```tsx
export function PipelineStatusDialog({ open, onOpenChange }: Props) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const historyRef = useRef<HTMLDivElement>(null);
  const [isUserScrolled, setIsUserScrolled] = useState(false);
  const [showCancelConfirm, setShowCancelConfirm] = useState(false);
  const [position, setPosition] = useState<'left' | 'center' | 'right'>('center');

  const { data: status } = useQuery({
    queryKey: ['pipeline-status'],
    queryFn: getEnhancedPipelineStatus,
    refetchInterval: open ? 2000 : false,
    enabled: open,
  });

  // Auto-scroll to bottom unless user scrolled up
  useEffect(() => {
    const container = historyRef.current;
    if (!container || isUserScrolled) return;
    container.scrollTop = container.scrollHeight;
  }, [status?.history_messages, isUserScrolled]);

  const progress = status?.total_documents > 0
    ? (status.processed_documents / status.total_documents) * 100
    : 0;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className={positionClasses[position]}>
        <DialogHeader>
          <DialogTitle className="flex items-center justify-between">
            <span className="flex items-center gap-2">
              <Activity className="h-5 w-5" />
              {t('pipeline.title')}
            </span>
            <PositionControls position={position} onChange={setPosition} />
          </DialogTitle>
        </DialogHeader>

        {status?.is_busy && (
          <div className="space-y-4">
            {/* Job Progress */}
            <div className="p-4 border rounded-lg space-y-2">
              <div className="flex items-center justify-between">
                <span className="flex items-center gap-2">
                  <Loader2 className="h-4 w-4 animate-spin" />
                  {status.job_name}
                </span>
                <span className="text-sm text-muted-foreground">
                  {status.processed_documents}/{status.total_documents} ({progress.toFixed(0)}%)
                </span>
              </div>
              <Progress value={progress} />
              <div className="flex items-center justify-between text-xs text-muted-foreground">
                <span>Started: {formatTime(status.job_start)}</span>
                <span>Batch {status.current_batch}/{status.total_batches}</span>
              </div>
            </div>

            {/* Statistics */}
            <div className="grid grid-cols-4 gap-2 text-center">
              <StatBox label="Pending" value={status.pending_tasks} />
              <StatBox label="Running" value={status.processing_tasks} color="yellow" />
              <StatBox label="Complete" value={status.completed_tasks} color="green" />
              <StatBox label="Failed" value={status.failed_tasks} color="red" />
            </div>

            {/* Messages */}
            <div className="space-y-2">
              <label className="text-sm font-medium">Messages:</label>
              <ScrollArea 
                ref={historyRef}
                onScroll={handleScroll}
                className="h-48 rounded-md bg-muted p-3 font-mono text-xs"
              >
                {status.history_messages.map((msg, idx) => (
                  <div key={idx} className={messageColors[msg.level]}>
                    [{formatTime(msg.timestamp)}] {msg.message}
                  </div>
                ))}
              </ScrollArea>
            </div>

            {/* Cancel */}
            <Button
              variant="destructive"
              onClick={() => setShowCancelConfirm(true)}
              disabled={status.cancellation_requested}
              className="w-full"
            >
              {status.cancellation_requested ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  Cancelling...
                </>
              ) : (
                <>
                  <XCircle className="mr-2 h-4 w-4" />
                  Cancel Pipeline
                </>
              )}
            </Button>
          </div>
        )}

        {/* Cancel Confirmation Dialog */}
        <CancelConfirmDialog
          open={showCancelConfirm}
          onOpenChange={setShowCancelConfirm}
          onConfirm={handleCancel}
          processedCount={status?.processed_documents || 0}
        />
      </DialogContent>
    </Dialog>
  );
}
```

### 2. Batch Upload with Track ID

```tsx
// Enhanced upload handler
const handleFilesUpload = useCallback(async (files: File[]) => {
  const trackId = `upload_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
  
  // Show batch in UI
  setCurrentBatch({
    trackId,
    files: files.map(f => ({ name: f.name, status: 'pending' })),
  });
  
  for (const file of files) {
    const response = await uploadDocument({
      content: await file.text(),
      title: file.name,
      track_id: trackId,  // All files share same track
      async_processing: true,
    });
    
    // Handle duplicate
    if (response.status === 'duplicated') {
      toast.warning(`${file.name} already exists`);
    }
  }
  
  // Start polling track status
  setPollingTrackId(trackId);
}, []);

// Poll track status
const { data: trackStatus } = useQuery({
  queryKey: ['track-status', pollingTrackId],
  queryFn: () => getTrackStatus(pollingTrackId),
  refetchInterval: 2000,
  enabled: !!pollingTrackId,
});

// Show batch progress
{trackStatus && (
  <BatchProgress
    trackId={trackStatus.track_id}
    documents={trackStatus.documents}
    summary={trackStatus.status_summary}
  />
)}
```

### 3. Status Counts in Filters

```tsx
// Use server-side counts instead of client-side calculation
const { data } = useQuery({
  queryKey: ['documents', page, pageSize],
  queryFn: () => getDocuments({ page, page_size: pageSize }),
});

// status_counts comes from API now
const statusCounts = data?.status_counts || {
  pending: 0,
  processing: 0,
  completed: 0,
  failed: 0,
};

<DocumentFilters
  statusCounts={statusCounts}  // From API, not calculated
  // ...
/>
```

## Summary

### API Changes
1. **Enhanced Pipeline Status** - Add batch progress, messages
2. **Track ID System** - Group uploaded documents
3. **Status Counts** - Return in list response
4. **Content Summary** - First 200 chars of document
5. **Detailed Errors** - Step, reason, suggestion

### Frontend Changes
1. **Enhanced Pipeline Dialog** - Progress bar, messages, position control
2. **Batch Upload** - Track ID, group progress
3. **Server-Side Counts** - Use API counts for filters
4. **Cancel Confirmation** - Two-step process

---

**Next:** [Implementation Plan](./06-implementation-plan.md)
