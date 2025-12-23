# Server-Side Analysis

## Overview

This document analyzes EdgeQuake's current server-side implementation for document upload and processing, focusing on the API design, task system, and data models.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                     EdgeQuake API Layer                          │
│  (edgequake-api crate - Axum-based REST API)                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ┌──────────────────┐     ┌──────────────────┐                   │
│  │  documents.rs    │     │    tasks.rs      │                   │
│  │                  │     │                  │                   │
│  │  POST /documents │     │  GET /tasks      │                   │
│  │  GET /documents  │     │  GET /tasks/:id  │                   │
│  │  DELETE /docs/:id│     │  POST /cancel    │                   │
│  └────────┬─────────┘     │  POST /retry     │                   │
│           │               └────────┬─────────┘                   │
│           │                        │                              │
├───────────┼────────────────────────┼──────────────────────────────┤
│           │                        │                              │
│  ┌────────▼────────────────────────▼─────────┐                   │
│  │           edgequake-tasks crate            │                   │
│  │                                            │                   │
│  │  ┌─────────────┐  ┌────────────────────┐  │                   │
│  │  │   Task      │  │    WorkerPool      │  │                   │
│  │  │  - track_id │  │  - num_workers     │  │                   │
│  │  │  - status   │  │  - auto_retry      │  │                   │
│  │  │  - progress │  │  - process_task()  │  │                   │
│  │  └─────────────┘  └────────────────────┘  │                   │
│  └───────────────────────────────────────────┘                   │
│                                                                   │
│  ┌───────────────────────────────────────────┐                   │
│  │         edgequake-core crate              │                   │
│  │         (Pipeline orchestration)          │                   │
│  └───────────────────────────────────────────┘                   │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

## Document Upload Flow

### Endpoint: `POST /documents`

**File:** `edgequake/crates/edgequake-api/src/handlers/documents.rs`

```rust
pub struct UploadDocumentRequest {
    pub content: String,
    pub title: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub async_processing: Option<bool>,  // Default: false
}

pub struct UploadDocumentResponse {
    pub document_id: String,
    pub status: String,
    pub task_id: Option<String>,        // Only present if async_processing=true
    pub chunk_count: Option<usize>,
    pub entity_count: Option<usize>,
    pub relationship_count: Option<usize>,
}
```

### Processing Modes

#### Synchronous Mode (Default)
```
Client Request → Upload → Process → Extract → Index → Response
                                    (blocking)
```
- Returns immediately with full results
- Good for small documents
- No task_id returned

#### Asynchronous Mode (async_processing=true)
```
Client Request → Upload → Create Task → Response
                              ↓
                         WorkerPool
                              ↓
                      Process in Background
```
- Returns immediately with task_id
- Client polls `/tasks/{task_id}` for status
- Scalable for large documents

### Sync vs Async Decision Logic

```rust
// In upload_document handler
if request.async_processing.unwrap_or(false) {
    // Create task with track_id
    let track_id = format!("upload_{}", Uuid::new_v4());
    let task = Task::new(
        track_id.clone(),
        TaskType::Upload,
        json!({
            "document_id": document_id,
            "content": content,
            "title": title,
        }),
    );
    
    // Queue task for background processing
    task_store.add_task(task).await?;
    
    // Return immediately with task_id
    Ok(Json(UploadDocumentResponse {
        document_id,
        status: "pending".to_string(),
        task_id: Some(track_id),
        ..Default::default()
    }))
} else {
    // Process synchronously
    let result = pipeline.process_document(&content, &title).await?;
    Ok(Json(UploadDocumentResponse {
        document_id,
        status: "completed".to_string(),
        task_id: None,
        chunk_count: Some(result.chunk_count),
        entity_count: Some(result.entity_count),
        relationship_count: Some(result.relationship_count),
    }))
}
```

## Task System

### Task Data Model

**File:** `edgequake/crates/edgequake-tasks/src/types.rs`

```rust
pub struct Task {
    pub track_id: String,
    pub task_type: TaskType,
    pub status: TaskStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub payload: serde_json::Value,
    pub result: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub retry_count: u32,
    pub max_retries: u32,
    pub progress: Option<TaskProgress>,
    pub metadata: Option<serde_json::Value>,
}

pub enum TaskStatus {
    Pending,
    Processing,
    Indexed,    // Success
    Failed,
    Cancelled,
}

pub enum TaskType {
    Upload,
    Insert,
    Scan,
    Reindex,
}

pub struct TaskProgress {
    pub current_step: u32,
    pub total_steps: u32,
    pub percent_complete: f32,
}
```

### Task Lifecycle

```
┌─────────┐     ┌────────────┐     ┌─────────┐
│ Pending │────▶│ Processing │────▶│ Indexed │
└─────────┘     └────────────┘     └─────────┘
     │               │                   
     │               ▼                   
     │          ┌─────────┐              
     │          │ Failed  │──(retry)──▶ Pending
     │          └─────────┘              
     │               │                   
     ▼               ▼                   
┌───────────┐                            
│ Cancelled │                            
└───────────┘                            
```

### Task Methods

```rust
impl Task {
    pub fn mark_processing(&mut self) {
        self.status = TaskStatus::Processing;
        self.started_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    pub fn mark_success(&mut self, result: serde_json::Value) {
        self.status = TaskStatus::Indexed;
        self.result = Some(result);
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    pub fn mark_failed(&mut self, error: String) {
        self.status = TaskStatus::Failed;
        self.error_message = Some(error);
        self.updated_at = Utc::now();
    }

    pub fn update_progress(&mut self, current: u32, total: u32) {
        self.progress = Some(TaskProgress {
            current_step: current,
            total_steps: total,
            percent_complete: (current as f32 / total as f32) * 100.0,
        });
        self.updated_at = Utc::now();
    }
}
```

## Tasks API

### Endpoint: `GET /tasks`

**File:** `edgequake/crates/edgequake-api/src/handlers/tasks.rs`

```rust
pub struct TaskListResponse {
    pub tasks: Vec<TaskResponse>,
    pub pagination: PaginationInfo,
    pub statistics: StatisticsInfo,
}

pub struct PaginationInfo {
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

pub struct StatisticsInfo {
    pub pending: usize,
    pub processing: usize,
    pub indexed: usize,
    pub failed: usize,
    pub cancelled: usize,
}
```

### Task Query Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `status` | string | - | Filter by status (pending, processing, indexed, failed, cancelled) |
| `task_type` | string | - | Filter by type (upload, insert, scan, reindex) |
| `page` | int | 1 | Page number |
| `page_size` | int | 20 | Items per page |
| `sort_by` | string | created_at | Sort field |
| `sort_order` | string | desc | Sort direction (asc, desc) |

### Endpoint: `GET /tasks/:track_id`

Returns single task with full details:

```rust
pub struct TaskResponse {
    pub track_id: String,
    pub task_type: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub error_message: Option<String>,
    pub retry_count: u32,
    pub max_retries: u32,
    pub progress: Option<TaskProgress>,
    pub result: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
}
```

## Document List API

### Endpoint: `GET /documents`

```rust
pub struct ListDocumentsResponse {
    pub documents: Vec<DocumentSummary>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
}

pub struct DocumentSummary {
    pub id: String,
    pub title: Option<String>,
    pub file_name: Option<String>,
    pub chunk_count: usize,
    pub entity_count: Option<usize>,
    pub status: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}
```

### Document Metadata Storage

Documents store metadata in KV storage with this structure:

```rust
// Key: doc_meta:{document_id}
// Value: JSON object
{
    "title": "Document Title",
    "file_name": "original_file.txt",
    "status": "completed",
    "created_at": "2024-01-15T10:30:00Z",
    "updated_at": "2024-01-15T10:35:00Z",
    "entity_count": 25,
    "chunk_count": 12
}
```

## Worker Pool

**File:** `edgequake/crates/edgequake-tasks/src/worker.rs`

```rust
pub struct WorkerPool {
    pub workers: Vec<Worker>,
    pub task_store: Arc<TaskStore>,
    pub num_workers: usize,
    pub auto_retry: bool,
    pub retry_delay: Duration,
}

impl WorkerPool {
    pub async fn process_task(&self, task: &mut Task) -> Result<()> {
        task.mark_processing();
        
        match &task.task_type {
            TaskType::Upload => {
                let document_id = task.payload["document_id"].as_str()?;
                let content = task.payload["content"].as_str()?;
                let title = task.payload["title"].as_str();
                
                // Update progress: chunking
                task.update_progress(1, 4);
                let chunks = chunker.chunk(&content)?;
                
                // Update progress: embedding
                task.update_progress(2, 4);
                let embeddings = embedder.embed(&chunks).await?;
                
                // Update progress: entity extraction
                task.update_progress(3, 4);
                let entities = extractor.extract(&content).await?;
                
                // Update progress: indexing
                task.update_progress(4, 4);
                storage.index(document_id, chunks, embeddings, entities).await?;
                
                task.mark_success(json!({
                    "chunk_count": chunks.len(),
                    "entity_count": entities.len(),
                }));
            }
            // ... other task types
        }
        
        Ok(())
    }
}
```

## Current Limitations

### 1. No Batch Progress Tracking
- Tasks track individual progress (current_step/total_steps)
- No global batch progress (documents processed / total documents)
- Missing: `batchs`, `cur_batch` like LightRAG

### 2. No History Messages
- No log of processing activities
- Missing: `history_messages`, `latest_message` like LightRAG

### 3. No Track ID Grouping
- Each task has individual track_id
- No way to group documents uploaded in same batch
- Missing: `TrackStatusResponse` like LightRAG

### 4. Limited Document Metadata
- Missing `content_summary` (first N chars of content)
- Missing `file_path` (original file location)
- Missing `error_msg` in document response

### 5. No Status Counts in List Response
- Client must count statuses manually
- Missing: `status_counts` like LightRAG's `PaginatedDocsResponse`

## Summary

EdgeQuake has a solid foundation with:
- ✅ Async task processing with worker pool
- ✅ Task progress tracking (steps, percent)
- ✅ Task retry and cancellation
- ✅ Task statistics (pending, processing, indexed, failed, cancelled)

Key gaps compared to LightRAG:
- ❌ Batch/job-level progress tracking
- ❌ Real-time history messages
- ❌ Track-based document grouping
- ❌ Status counts in paginated responses
- ❌ Content summary and error details in documents

---

**Next:** [Client-Side Analysis](./02-client-side-analysis.md)
