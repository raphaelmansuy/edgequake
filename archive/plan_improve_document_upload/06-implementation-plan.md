# Implementation Plan

## Overview

This document provides a phased implementation approach for the document upload and processing improvements. The plan is designed for incremental delivery with each phase providing immediate value.

## Phase Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    Implementation Phases                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  Phase 1: API Quick Wins (1-2 days)                              │
│    ├── Status counts in document list API                        │
│    ├── Content summary in document response                      │
│    └── Enhanced error messages in task response                  │
│                                                                   │
│  Phase 2: Track ID System (2-3 days)                             │
│    ├── Track ID generation and storage                           │
│    ├── Track status endpoint                                     │
│    └── Frontend batch grouping                                   │
│                                                                   │
│  Phase 3: Pipeline Messages (3-4 days)                           │
│    ├── Pipeline state management                                 │
│    ├── Message logging in worker                                 │
│    ├── Enhanced pipeline status endpoint                         │
│    └── Frontend pipeline dialog update                           │
│                                                                   │
│  Phase 4: Polish & Extras (1-2 days)                             │
│    ├── Cancel confirmation dialog                                │
│    ├── Duplicate detection                                       │
│    └── Position controls for dialogs                             │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

## Phase 1: API Quick Wins

**Estimated Time:** 1-2 days

### 1.1 Status Counts in Document List

**File:** `edgequake/crates/edgequake-api/src/handlers/documents.rs`

```rust
// Update ListDocumentsResponse
#[derive(Serialize)]
pub struct ListDocumentsResponse {
    pub documents: Vec<DocumentSummary>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub status_counts: StatusCounts,  // ADD
}

#[derive(Serialize, Default)]
pub struct StatusCounts {
    pub pending: usize,
    pub processing: usize,
    pub completed: usize,
    pub failed: usize,
}

// Update list_documents handler
pub async fn list_documents(
    State(state): State<AppState>,
    Query(params): Query<ListDocumentsParams>,
) -> Result<Json<ListDocumentsResponse>, ApiError> {
    // ... existing pagination logic

    // Get all documents for status counts (or use a separate query)
    let all_docs = state.edgequake.list_documents(None, None).await?;

    let status_counts = StatusCounts {
        pending: all_docs.iter().filter(|d| d.status == Some("pending".into())).count(),
        processing: all_docs.iter().filter(|d| d.status == Some("processing".into())).count(),
        completed: all_docs.iter().filter(|d| d.status.is_none() || d.status == Some("completed".into())).count(),
        failed: all_docs.iter().filter(|d| d.status == Some("failed".into())).count(),
    };

    Ok(Json(ListDocumentsResponse {
        documents,
        total,
        page,
        page_size,
        status_counts,
    }))
}
```

### 1.2 Content Summary in Document Response

**File:** `edgequake/crates/edgequake-api/src/handlers/documents.rs`

```rust
// Update DocumentSummary
#[derive(Serialize)]
pub struct DocumentSummary {
    pub id: String,
    pub title: Option<String>,
    pub file_name: Option<String>,
    pub content_summary: Option<String>,  // ADD: First 200 chars
    pub content_length: Option<usize>,    // ADD: Total length
    pub chunk_count: usize,
    pub entity_count: Option<usize>,
    pub status: Option<String>,
    pub error_message: Option<String>,    // ADD: If failed
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}
```

**Storage Update:** When storing document metadata, also store content summary:

```rust
// In upload_document
let content_summary = if content.len() > 200 {
    format!("{}...", &content[..200])
} else {
    content.clone()
};

metadata.insert("content_summary", content_summary);
metadata.insert("content_length", content.len().to_string());
```

### 1.3 Enhanced Error Messages

**File:** `edgequake/crates/edgequake-tasks/src/types.rs`

```rust
// Add TaskError struct
#[derive(Serialize, Deserialize, Clone)]
pub struct TaskError {
    pub message: String,
    pub step: String,      // "chunking", "embedding", "extraction", "indexing"
    pub reason: String,    // Specific reason
    pub suggestion: String, // How to fix
    pub retryable: bool,
}

// Update Task
pub struct Task {
    // ... existing fields
    pub error: Option<TaskError>,  // ADD (in addition to error_message)
}

impl Task {
    pub fn mark_failed_with_details(
        &mut self,
        step: &str,
        message: &str,
        reason: &str,
        suggestion: &str,
        retryable: bool,
    ) {
        self.status = TaskStatus::Failed;
        self.error = Some(TaskError {
            message: message.to_string(),
            step: step.to_string(),
            reason: reason.to_string(),
            suggestion: suggestion.to_string(),
            retryable,
        });
        self.error_message = Some(message.to_string()); // Keep backward compat
        self.updated_at = Utc::now();
    }
}
```

### 1.4 Frontend Updates

**File:** `edgequake_webui/src/types/index.ts`

```typescript
// Update Document type
export interface Document {
  // ... existing fields
  content_summary?: string;
  content_length?: number;
  error_message?: string;
}

// Update PaginatedResponse
export interface PaginatedResponse<T> {
  items: T[];
  total: number;
  page: number;
  page_size: number;
  has_more: boolean;
  status_counts?: StatusCounts; // ADD
}

export interface StatusCounts {
  pending: number;
  processing: number;
  completed: number;
  failed: number;
}
```

**File:** `edgequake_webui/src/components/documents/document-manager.tsx`

```typescript
// Use API status counts instead of client-side calculation
const statusCounts = data?.status_counts || {
  pending: 0,
  processing: 0,
  completed: 0,
  failed: 0,
};

// Update DocStatus type to include 'all'
const allCount =
  statusCounts.pending +
  statusCounts.processing +
  statusCounts.completed +
  statusCounts.failed;

<DocumentFilters
  statusCounts={{
    all: allCount,
    ...statusCounts,
  }}
  // ...
/>;
```

---

## Phase 2: Track ID System

**Estimated Time:** 2-3 days

### 2.1 Track ID Generation

**File:** `edgequake/crates/edgequake-api/src/handlers/documents.rs`

```rust
#[derive(Deserialize)]
pub struct UploadDocumentRequest {
    pub content: String,
    pub title: Option<String>,
    pub metadata: Option<Value>,
    pub async_processing: Option<bool>,
    pub track_id: Option<String>,  // ADD: Client can provide
}

#[derive(Serialize)]
pub struct UploadDocumentResponse {
    pub document_id: String,
    pub status: String,
    pub task_id: Option<String>,
    pub track_id: String,  // ADD: Always return
    // ... other fields
}

pub async fn upload_document(
    State(state): State<AppState>,
    Json(request): Json<UploadDocumentRequest>,
) -> Result<Json<UploadDocumentResponse>, ApiError> {
    // Generate or use provided track_id
    let track_id = request.track_id.unwrap_or_else(|| {
        format!("upload_{}_{}",
            Utc::now().format("%Y%m%d_%H%M%S"),
            Uuid::new_v4().to_string().split('-').next().unwrap()
        )
    });

    // Store track_id in document metadata
    let mut metadata = request.metadata.unwrap_or_default();
    metadata["track_id"] = Value::String(track_id.clone());

    // ... rest of upload logic

    Ok(Json(UploadDocumentResponse {
        document_id,
        status: "pending".to_string(),
        task_id: Some(task_id),
        track_id,
        // ...
    }))
}
```

### 2.2 Track Status Endpoint

**File:** `edgequake/crates/edgequake-api/src/handlers/documents.rs`

```rust
#[derive(Serialize)]
pub struct TrackStatusResponse {
    pub track_id: String,
    pub created_at: String,
    pub documents: Vec<DocumentSummary>,
    pub total_count: usize,
    pub status_summary: StatusCounts,
}

pub async fn get_track_status(
    State(state): State<AppState>,
    Path(track_id): Path<String>,
) -> Result<Json<TrackStatusResponse>, ApiError> {
    // Get all documents with this track_id
    let all_docs = state.edgequake.list_documents(None, None).await?;
    let track_docs: Vec<_> = all_docs
        .into_iter()
        .filter(|d| {
            d.metadata
                .as_ref()
                .and_then(|m| m.get("track_id"))
                .and_then(|v| v.as_str())
                == Some(&track_id)
        })
        .collect();

    let status_summary = StatusCounts {
        pending: track_docs.iter().filter(|d| d.status == Some("pending".into())).count(),
        processing: track_docs.iter().filter(|d| d.status == Some("processing".into())).count(),
        completed: track_docs.iter().filter(|d| d.status.is_none() || d.status == Some("completed".into())).count(),
        failed: track_docs.iter().filter(|d| d.status == Some("failed".into())).count(),
    };

    let created_at = track_docs
        .iter()
        .filter_map(|d| d.created_at.as_ref())
        .min()
        .cloned()
        .unwrap_or_default();

    Ok(Json(TrackStatusResponse {
        track_id,
        created_at,
        documents: track_docs.iter().map(to_summary).collect(),
        total_count: track_docs.len(),
        status_summary,
    }))
}
```

### 2.3 Route Registration

**File:** `edgequake/crates/edgequake-api/src/router.rs`

```rust
// Add new route
.route("/documents/track/:track_id", get(handlers::documents::get_track_status))
```

### 2.4 Frontend Track Status

**File:** `edgequake_webui/src/lib/api/edgequake.ts`

```typescript
export interface TrackStatusResponse {
  track_id: string;
  created_at: string;
  documents: Document[];
  total_count: number;
  status_summary: StatusCounts;
}

export async function getTrackStatus(
  trackId: string
): Promise<TrackStatusResponse> {
  return api.get<TrackStatusResponse>(`/documents/track/${trackId}`);
}
```

**File:** `edgequake_webui/src/components/documents/document-manager.tsx`

```typescript
// Enhanced upload handler with track_id
const handleFilesUpload = useCallback(async (files: File[]) => {
  const trackId = `upload_${Date.now()}_${Math.random()
    .toString(36)
    .slice(2, 8)}`;

  // ... upload files with track_id

  for (const file of files) {
    await uploadDocument({
      content: await file.text(),
      title: file.name,
      track_id: trackId, // All files share same track
      async_processing: true,
    });
  }

  // Show batch progress
  setActiveTrackId(trackId);
}, []);

// Poll track status when active
const { data: trackStatus } = useQuery({
  queryKey: ["track-status", activeTrackId],
  queryFn: () => getTrackStatus(activeTrackId!),
  refetchInterval: activeTrackId ? 2000 : false,
  enabled: !!activeTrackId,
});

// Render batch progress
{
  trackStatus && (
    <BatchProgressCard
      trackId={trackStatus.track_id}
      summary={trackStatus.status_summary}
      total={trackStatus.total_count}
      onClose={() => setActiveTrackId(null)}
    />
  );
}
```

---

## Phase 3: Pipeline Messages

**Estimated Time:** 3-4 days

### 3.1 Pipeline State Management

**File:** `edgequake/crates/edgequake-tasks/src/pipeline_state.rs` (NEW)

```rust
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct PipelineMessage {
    pub timestamp: String,
    pub level: String,  // "info", "warn", "error"
    pub message: String,
}

#[derive(Clone)]
pub struct PipelineState {
    inner: Arc<RwLock<PipelineStateInner>>,
}

struct PipelineStateInner {
    is_busy: bool,
    job_name: Option<String>,
    job_start: Option<DateTime<Utc>>,
    total_documents: u32,
    processed_documents: u32,
    current_batch: u32,
    total_batches: u32,
    messages: Vec<PipelineMessage>,
    cancellation_requested: bool,
    max_messages: usize,
}

impl PipelineState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(PipelineStateInner {
                is_busy: false,
                job_name: None,
                job_start: None,
                total_documents: 0,
                processed_documents: 0,
                current_batch: 0,
                total_batches: 0,
                messages: Vec::new(),
                cancellation_requested: false,
                max_messages: 100,
            }))
        }
    }

    pub async fn start_job(&self, name: String, total_docs: u32, batches: u32) {
        let mut inner = self.inner.write().await;
        inner.is_busy = true;
        inner.job_name = Some(name.clone());
        inner.job_start = Some(Utc::now());
        inner.total_documents = total_docs;
        inner.processed_documents = 0;
        inner.current_batch = 0;
        inner.total_batches = batches;
        inner.cancellation_requested = false;
        self.log_internal(&mut inner, "info", format!("Starting: {}", name));
    }

    pub async fn log(&self, level: &str, message: String) {
        let mut inner = self.inner.write().await;
        self.log_internal(&mut inner, level, message);
    }

    fn log_internal(&self, inner: &mut PipelineStateInner, level: &str, message: String) {
        inner.messages.push(PipelineMessage {
            timestamp: Utc::now().to_rfc3339(),
            level: level.to_string(),
            message,
        });

        // Keep last N messages
        if inner.messages.len() > inner.max_messages {
            inner.messages.remove(0);
        }
    }

    pub async fn advance_batch(&self) {
        let mut inner = self.inner.write().await;
        inner.current_batch += 1;
        let msg = format!("Batch {}/{}", inner.current_batch, inner.total_batches);
        self.log_internal(&mut inner, "info", msg);
    }

    pub async fn document_processed(&self, doc_id: &str, entities: usize) {
        let mut inner = self.inner.write().await;
        inner.processed_documents += 1;
        let msg = format!(
            "✓ {} ({} entities) - {}/{}",
            doc_id, entities, inner.processed_documents, inner.total_documents
        );
        self.log_internal(&mut inner, "info", msg);
    }

    pub async fn finish_job(&self) {
        let mut inner = self.inner.write().await;
        let msg = format!("Complete: {} documents", inner.processed_documents);
        self.log_internal(&mut inner, "info", msg);
        inner.is_busy = false;
        inner.job_name = None;
    }

    pub async fn request_cancellation(&self) {
        let mut inner = self.inner.write().await;
        inner.cancellation_requested = true;
        self.log_internal(&mut inner, "warn", "Cancellation requested".to_string());
    }

    pub async fn is_cancellation_requested(&self) -> bool {
        self.inner.read().await.cancellation_requested
    }

    pub async fn get_status(&self) -> PipelineStatusSnapshot {
        let inner = self.inner.read().await;
        PipelineStatusSnapshot {
            is_busy: inner.is_busy,
            job_name: inner.job_name.clone(),
            job_start: inner.job_start.map(|d| d.to_rfc3339()),
            total_documents: inner.total_documents,
            processed_documents: inner.processed_documents,
            current_batch: inner.current_batch,
            total_batches: inner.total_batches,
            latest_message: inner.messages.last().map(|m| m.message.clone()),
            history_messages: inner.messages.clone(),
            cancellation_requested: inner.cancellation_requested,
        }
    }
}

#[derive(Serialize)]
pub struct PipelineStatusSnapshot {
    pub is_busy: bool,
    pub job_name: Option<String>,
    pub job_start: Option<String>,
    pub total_documents: u32,
    pub processed_documents: u32,
    pub current_batch: u32,
    pub total_batches: u32,
    pub latest_message: Option<String>,
    pub history_messages: Vec<PipelineMessage>,
    pub cancellation_requested: bool,
}
```

### 3.2 Worker Pool Integration

**File:** `edgequake/crates/edgequake-tasks/src/worker.rs`

```rust
pub struct WorkerPool {
    // ... existing fields
    pipeline_state: PipelineState,  // ADD
}

impl WorkerPool {
    pub fn new(/* ... */, pipeline_state: PipelineState) -> Self {
        // ...
    }

    pub async fn process_pending_tasks(&self) -> Result<()> {
        let pending = self.task_store.get_pending_tasks().await?;
        if pending.is_empty() {
            return Ok(());
        }

        let batch_size = 4;
        let total_batches = (pending.len() + batch_size - 1) / batch_size;

        self.pipeline_state.start_job(
            format!("Processing {} documents", pending.len()),
            pending.len() as u32,
            total_batches as u32,
        ).await;

        for batch in pending.chunks(batch_size) {
            // Check for cancellation
            if self.pipeline_state.is_cancellation_requested().await {
                self.pipeline_state.log("warn", "Processing cancelled".to_string()).await;
                break;
            }

            self.pipeline_state.advance_batch().await;

            for task in batch {
                self.pipeline_state.log(
                    "info",
                    format!("Extracting entities from {}...", task.track_id),
                ).await;

                match self.process_single_task(task).await {
                    Ok(result) => {
                        let entity_count = result.entity_count.unwrap_or(0);
                        self.pipeline_state.document_processed(
                            &task.track_id,
                            entity_count,
                        ).await;
                    }
                    Err(e) => {
                        self.pipeline_state.log(
                            "error",
                            format!("Failed {}: {}", task.track_id, e),
                        ).await;
                    }
                }
            }
        }

        self.pipeline_state.finish_job().await;
        Ok(())
    }
}
```

### 3.3 Enhanced Pipeline Status Endpoint

**File:** `edgequake/crates/edgequake-api/src/handlers/pipeline.rs` (NEW)

```rust
use crate::state::AppState;
use axum::{extract::State, Json};
use edgequake_tasks::pipeline_state::PipelineStatusSnapshot;

#[derive(Serialize)]
pub struct PipelineStatusResponse {
    // From pipeline state
    #[serde(flatten)]
    pub pipeline: PipelineStatusSnapshot,

    // From task statistics
    pub pending_tasks: usize,
    pub processing_tasks: usize,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
}

pub async fn get_pipeline_status(
    State(state): State<AppState>,
) -> Result<Json<PipelineStatusResponse>, ApiError> {
    let pipeline = state.pipeline_state.get_status().await;
    let stats = state.task_store.get_statistics().await?;

    Ok(Json(PipelineStatusResponse {
        pipeline,
        pending_tasks: stats.pending,
        processing_tasks: stats.processing,
        completed_tasks: stats.indexed,
        failed_tasks: stats.failed,
    }))
}

pub async fn cancel_pipeline(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.pipeline_state.request_cancellation().await;
    Ok(Json(json!({ "status": "cancellation_requested" })))
}
```

### 3.4 Route Registration

```rust
.route("/pipeline/status", get(handlers::pipeline::get_pipeline_status))
.route("/pipeline/cancel", post(handlers::pipeline::cancel_pipeline))
```

### 3.5 Frontend Pipeline Dialog

See detailed implementation in `05-proposed-improvements.md` - Phase 3 Frontend section.

---

## Phase 4: Polish & Extras

**Estimated Time:** 1-2 days

### 4.1 Cancel Confirmation Dialog

```tsx
// CancelConfirmDialog component
function CancelConfirmDialog({
  open,
  onOpenChange,
  onConfirm,
  processedCount,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onConfirm: () => void;
  processedCount: number;
}) {
  const { t } = useTranslation();

  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>
            {t("pipeline.cancelConfirmTitle")}
          </AlertDialogTitle>
          <AlertDialogDescription>
            {t("pipeline.cancelConfirmDesc", { count: processedCount })}
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>{t("common.keepProcessing")}</AlertDialogCancel>
          <AlertDialogAction onClick={onConfirm} className="bg-destructive">
            {t("common.yesCancel")}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
```

### 4.2 Duplicate Detection

**Backend:**

```rust
// In upload_document handler
async fn check_duplicate(
    state: &AppState,
    content: &str,
) -> Option<String> {
    // Simple hash-based duplicate detection
    let hash = sha256::digest(content);

    let docs = state.edgequake.list_documents(None, None).await.ok()?;
    docs.into_iter()
        .find(|d| {
            d.metadata
                .as_ref()
                .and_then(|m| m.get("content_hash"))
                .and_then(|v| v.as_str())
                == Some(&hash)
        })
        .map(|d| d.id)
}

// Use in upload
if let Some(existing_id) = check_duplicate(&state, &content).await {
    return Ok(Json(UploadDocumentResponse {
        document_id: existing_id.clone(),
        status: "duplicated".to_string(),
        duplicate_of: Some(existing_id),
        // ...
    }));
}

// Store hash with document
metadata["content_hash"] = Value::String(sha256::digest(&content));
```

**Frontend:**

```typescript
// Handle duplicate response
if (response.status === "duplicated") {
  toast.warning(
    t("documents.upload.duplicate", {
      name: file.name,
      existing: response.duplicate_of,
    })
  );
  // Skip or show option to replace
}
```

### 4.3 Position Controls

See implementation in `05-proposed-improvements.md` - Position control section.

---

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pipeline_state_logging() {
        let state = PipelineState::new();

        state.start_job("Test".to_string(), 10, 3).await;
        state.log("info", "Test message".to_string()).await;

        let snapshot = state.get_status().await;
        assert!(snapshot.is_busy);
        assert_eq!(snapshot.total_documents, 10);
        assert_eq!(snapshot.history_messages.len(), 2);
    }

    #[tokio::test]
    async fn test_status_counts() {
        // Test status count calculation
    }

    #[tokio::test]
    async fn test_track_id_grouping() {
        // Test documents grouped by track_id
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_upload_with_track_id() {
    let client = TestClient::new().await;

    let track_id = "test_track_123";

    // Upload 3 documents with same track_id
    for i in 0..3 {
        client.upload_document(&json!({
            "content": format!("Document {}", i),
            "title": format!("Doc {}", i),
            "track_id": track_id,
            "async_processing": true,
        })).await.unwrap();
    }

    // Get track status
    let status = client.get_track_status(track_id).await.unwrap();
    assert_eq!(status.total_count, 3);
}
```

### E2E Tests (Playwright)

```typescript
test("should show pipeline progress during upload", async ({ page }) => {
  await page.goto("/documents");

  // Upload multiple files
  const fileChooserPromise = page.waitForEvent("filechooser");
  await page.click('[data-testid="upload-zone"]');
  const fileChooser = await fileChooserPromise;
  await fileChooser.setFiles(["test1.txt", "test2.txt", "test3.txt"]);

  // Verify pipeline dialog shows
  await expect(page.locator('[data-testid="pipeline-dialog"]')).toBeVisible();

  // Verify progress updates
  await expect(page.locator('[data-testid="batch-progress"]')).toContainText(
    "Batch"
  );

  // Verify messages appear
  await expect(page.locator('[data-testid="pipeline-messages"]')).toContainText(
    "Processing"
  );
});
```

---

## Rollout Plan

1. **Phase 1**: Deploy to staging, verify status counts work correctly
2. **Phase 2**: Deploy track_id system, verify batch grouping
3. **Phase 3**: Deploy pipeline messages, verify real-time updates
4. **Phase 4**: Deploy polish features, full E2E testing

Each phase should be:

- Deployed independently
- Tested in staging for 24-48 hours
- Monitored for performance impact
- Rolled back if issues found

---

## Summary

| Phase     | Deliverable                            | Estimated Time | Value    |
| --------- | -------------------------------------- | -------------- | -------- |
| 1         | Status counts, content summary, errors | 1-2 days       | High     |
| 2         | Track ID system, batch grouping        | 2-3 days       | High     |
| 3         | Pipeline messages, real-time updates   | 3-4 days       | Critical |
| 4         | Polish features                        | 1-2 days       | Medium   |
| **Total** |                                        | **7-11 days**  |          |

**Ready to implement!**

---

**Back to:** [Index](./00-index.md)
