# Background Task Processing System

**Specification Version:** 1.0  
**Target Release:** EdgeQuake v1.1.0  
**Priority:** HIGH  
**Status:** Planning

---

## Overview

Implement a robust background task processing system to enable asynchronous document processing, improving API responsiveness and supporting long-running operations.

### Goals

1. **Non-blocking API:** Upload/processing requests return immediately with track_id
2. **Status Tracking:** Clients can poll task status via track_id
3. **Reliability:** Tasks survive server restarts (optional Redis backend)
4. **Scalability:** Support for distributed task processing
5. **Observability:** Task metrics and monitoring

### Non-Goals

- Complex workflow orchestration (use external tools like Temporal)
- Scheduled/cron jobs (use external scheduler)
- Priority queues (all tasks FIFO for v1.1)

---

## Architecture

### Components

```
┌─────────────┐         ┌──────────────┐         ┌─────────────┐
│   API       │         │  Task Queue  │         │   Worker    │
│  Handler    │────────▶│   (Channel)  │────────▶│    Pool     │
└─────────────┘         └──────────────┘         └─────────────┘
      │                                                  │
      │ track_id                                        │
      ▼                                                  ▼
┌─────────────┐                                  ┌─────────────┐
│  Response   │                                  │   Storage   │
│  (201)      │                                  │   (Tasks)   │
└─────────────┘                                  └─────────────┘
```

### Task Lifecycle

```
┌─────────┐
│ Created │ ◄─── API handler creates task record
└────┬────┘
     │
     ▼
┌─────────┐
│ Pending │ ◄─── Task added to queue
└────┬────┘
     │
     ▼
┌────────────┐
│ Processing │ ◄─── Worker picks up task
└─────┬──────┘
      │
      ├─── Success ───▶ ┌─────────┐
      │                 │ Indexed │
      │                 └─────────┘
      │
      └─── Failure ───▶ ┌─────────┐
                        │ Failed  │
                        └─────────┘
```

---

## Data Model

### Task Table Schema

```sql
CREATE TABLE tasks (
    -- Identity
    track_id VARCHAR(50) PRIMARY KEY,        -- Format: {type}-{uuid}
    task_type VARCHAR(20) NOT NULL,          -- upload, insert, scan, reindex

    -- Status
    status VARCHAR(20) NOT NULL,             -- pending, processing, indexed, failed

    -- Timestamps
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    started_at TIMESTAMPTZ,                  -- When processing began
    completed_at TIMESTAMPTZ,                -- When finished (success or failure)

    -- Error handling
    error_message TEXT,
    retry_count INTEGER DEFAULT 0,
    max_retries INTEGER DEFAULT 3,

    -- Payload
    task_data JSONB NOT NULL,                -- Task-specific data

    -- Metadata
    metadata JSONB,                          -- User-defined metadata

    -- Indexes
    CONSTRAINT valid_status CHECK (status IN ('pending', 'processing', 'indexed', 'failed', 'cancelled'))
);

CREATE INDEX idx_tasks_status ON tasks(status, created_at);
CREATE INDEX idx_tasks_type ON tasks(task_type);
CREATE INDEX idx_tasks_created ON tasks(created_at DESC);
```

### Task Types & Payloads

#### DocumentUploadTask

```json
{
  "type": "upload",
  "data": {
    "file_path": "/tmp/doc123.pdf",
    "content_type": "application/pdf",
    "workspace_id": "default",
    "metadata": {
      "title": "Research Paper",
      "author": "John Doe"
    }
  }
}
```

#### TextInsertTask

```json
{
  "type": "insert",
  "data": {
    "text": "This is the content to index...",
    "file_source": "api_request",
    "workspace_id": "default",
    "metadata": {
      "tags": ["important", "research"]
    }
  }
}
```

#### DirectoryScanTask

```json
{
  "type": "scan",
  "data": {
    "directory_path": "/data/input",
    "recursive": true,
    "file_pattern": "*.pdf",
    "workspace_id": "default"
  }
}
```

#### ReindexTask

```json
{
  "type": "reindex",
  "data": {
    "document_ids": ["doc-123", "doc-456"],
    "workspace_id": "default",
    "reason": "failed_initial_indexing"
  }
}
```

---

## API Endpoints

### 1. Create Task (Internal - used by document endpoints)

This is not directly exposed but used internally by document upload/insert endpoints.

```rust
pub struct TaskService {
    queue: TaskQueue,
    storage: Arc<dyn TaskStorage>,
}

impl TaskService {
    pub async fn create_task(
        &self,
        task_type: TaskType,
        task_data: serde_json::Value,
        metadata: Option<serde_json::Value>,
    ) -> Result<String, Error> {
        let track_id = generate_track_id(&task_type);

        let task = Task {
            track_id: track_id.clone(),
            task_type,
            status: TaskStatus::Pending,
            created_at: Utc::now(),
            task_data,
            metadata,
            ..Default::default()
        };

        // Persist task
        self.storage.create_task(&task).await?;

        // Enqueue for processing
        self.queue.send(task).await?;

        Ok(track_id)
    }
}
```

### 2. Get Task Status

```rust
GET /api/v1/tasks/{track_id}
```

**Request:**

```http
GET /api/v1/tasks/upload-a1b2c3d4-e5f6-4789-a0b1-c2d3e4f5g6h7 HTTP/1.1
Host: localhost:8080
```

**Response (200 OK):**

```json
{
  "track_id": "upload-a1b2c3d4-e5f6-4789-a0b1-c2d3e4f5g6h7",
  "task_type": "upload",
  "status": "processing",
  "created_at": "2025-12-22T18:00:00Z",
  "updated_at": "2025-12-22T18:00:05Z",
  "started_at": "2025-12-22T18:00:02Z",
  "completed_at": null,
  "progress": {
    "current_step": "extracting_entities",
    "total_steps": 4,
    "percent_complete": 50
  },
  "metadata": {
    "document_id": "doc-xyz789",
    "file_name": "research_paper.pdf",
    "file_size_bytes": 524288
  }
}
```

**Response (200 OK - Completed):**

```json
{
  "track_id": "upload-a1b2c3d4-e5f6-4789-a0b1-c2d3e4f5g6h7",
  "task_type": "upload",
  "status": "indexed",
  "created_at": "2025-12-22T18:00:00Z",
  "updated_at": "2025-12-22T18:00:35Z",
  "started_at": "2025-12-22T18:00:02Z",
  "completed_at": "2025-12-22T18:00:35Z",
  "result": {
    "document_id": "doc-xyz789",
    "chunk_count": 42,
    "entity_count": 87,
    "relationship_count": 134
  }
}
```

**Response (200 OK - Failed):**

```json
{
  "track_id": "upload-a1b2c3d4-e5f6-4789-a0b1-c2d3e4f5g6h7",
  "task_type": "upload",
  "status": "failed",
  "created_at": "2025-12-22T18:00:00Z",
  "updated_at": "2025-12-22T18:00:10Z",
  "started_at": "2025-12-22T18:00:02Z",
  "completed_at": "2025-12-22T18:00:10Z",
  "error_message": "Failed to extract text from PDF: Corrupted file",
  "retry_count": 3,
  "max_retries": 3
}
```

**Response (404 Not Found):**

```json
{
  "error": "task_not_found",
  "message": "No task found with track_id: invalid-track-id"
}
```

### 3. List Tasks

```rust
GET /api/v1/tasks
```

**Query Parameters:**

- `status`: Filter by status (pending, processing, indexed, failed)
- `task_type`: Filter by type (upload, insert, scan, reindex)
- `page`: Page number (default: 1)
- `page_size`: Items per page (default: 20, max: 100)
- `sort`: Sort field (created_at, updated_at)
- `order`: Sort order (asc, desc, default: desc)

**Request:**

```http
GET /api/v1/tasks?status=failed&page=1&page_size=20 HTTP/1.1
Host: localhost:8080
```

**Response (200 OK):**

```json
{
  "tasks": [
    {
      "track_id": "upload-abc123...",
      "task_type": "upload",
      "status": "failed",
      "created_at": "2025-12-22T17:30:00Z",
      "error_message": "LLM API timeout"
    },
    {
      "track_id": "insert-def456...",
      "task_type": "insert",
      "status": "failed",
      "created_at": "2025-12-22T17:25:00Z",
      "error_message": "Invalid UTF-8 encoding"
    }
  ],
  "pagination": {
    "total": 42,
    "page": 1,
    "page_size": 20,
    "total_pages": 3
  },
  "statistics": {
    "pending": 5,
    "processing": 2,
    "indexed": 1234,
    "failed": 42
  }
}
```

### 4. Cancel Task

```rust
POST /api/v1/tasks/{track_id}/cancel
```

**Request:**

```http
POST /api/v1/tasks/upload-a1b2c3d4.../cancel HTTP/1.1
Host: localhost:8080
```

**Response (200 OK):**

```json
{
  "track_id": "upload-a1b2c3d4...",
  "status": "cancelled",
  "message": "Task cancelled successfully"
}
```

**Response (409 Conflict):**

```json
{
  "error": "cannot_cancel",
  "message": "Cannot cancel task in status: indexed"
}
```

### 5. Retry Failed Task

```rust
POST /api/v1/tasks/{track_id}/retry
```

**Request:**

```http
POST /api/v1/tasks/upload-a1b2c3d4.../retry HTTP/1.1
Host: localhost:8080
```

**Response (200 OK):**

```json
{
  "track_id": "upload-a1b2c3d4...",
  "status": "pending",
  "message": "Task queued for retry",
  "retry_count": 1
}
```

---

## Implementation

### Task Queue

```rust
// crates/edgequake-tasks/src/queue.rs

use tokio::sync::mpsc;
use async_trait::async_trait;

#[async_trait]
pub trait TaskQueue: Send + Sync {
    async fn send(&self, task: Task) -> Result<(), Error>;
    async fn receive(&self) -> Result<Task, Error>;
    async fn size(&self) -> usize;
}

/// In-memory task queue using tokio channels
pub struct ChannelTaskQueue {
    sender: mpsc::Sender<Task>,
    receiver: Arc<Mutex<mpsc::Receiver<Task>>>,
}

impl ChannelTaskQueue {
    pub fn new(capacity: usize) -> Self {
        let (sender, receiver) = mpsc::channel(capacity);
        Self {
            sender,
            receiver: Arc::new(Mutex::new(receiver)),
        }
    }
}

#[async_trait]
impl TaskQueue for ChannelTaskQueue {
    async fn send(&self, task: Task) -> Result<(), Error> {
        self.sender
            .send(task)
            .await
            .map_err(|e| Error::QueueFull(e.to_string()))
    }

    async fn receive(&self) -> Result<Task, Error> {
        let mut rx = self.receiver.lock().await;
        rx.recv()
            .await
            .ok_or(Error::QueueClosed)
    }

    async fn size(&self) -> usize {
        // Channel size estimation
        0 // mpsc::Sender doesn't expose size
    }
}
```

### Redis Task Queue (Optional)

```rust
// crates/edgequake-tasks/src/redis_queue.rs

pub struct RedisTaskQueue {
    client: redis::Client,
    queue_name: String,
}

impl RedisTaskQueue {
    pub fn new(redis_url: &str, queue_name: String) -> Result<Self, Error> {
        let client = redis::Client::open(redis_url)?;
        Ok(Self { client, queue_name })
    }
}

#[async_trait]
impl TaskQueue for RedisTaskQueue {
    async fn send(&self, task: Task) -> Result<(), Error> {
        let mut conn = self.client.get_async_connection().await?;
        let task_json = serde_json::to_string(&task)?;

        redis::cmd("RPUSH")
            .arg(&self.queue_name)
            .arg(&task_json)
            .query_async(&mut conn)
            .await?;

        Ok(())
    }

    async fn receive(&self) -> Result<Task, Error> {
        let mut conn = self.client.get_async_connection().await?;

        // BLPOP with 1 second timeout
        let result: Option<(String, String)> = redis::cmd("BLPOP")
            .arg(&self.queue_name)
            .arg(1)
            .query_async(&mut conn)
            .await?;

        match result {
            Some((_, task_json)) => {
                let task: Task = serde_json::from_str(&task_json)?;
                Ok(task)
            }
            None => Err(Error::QueueEmpty),
        }
    }

    async fn size(&self) -> usize {
        let mut conn = self.client.get_async_connection().await.unwrap();
        redis::cmd("LLEN")
            .arg(&self.queue_name)
            .query_async(&mut conn)
            .await
            .unwrap_or(0)
    }
}
```

### Worker Pool

```rust
// crates/edgequake-tasks/src/worker.rs

pub struct WorkerPool {
    workers: Vec<JoinHandle<()>>,
    shutdown_tx: broadcast::Sender<()>,
}

impl WorkerPool {
    pub fn new(
        num_workers: usize,
        queue: Arc<dyn TaskQueue>,
        storage: Arc<dyn TaskStorage>,
        pipeline: Arc<Pipeline>,
    ) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        let mut workers = Vec::new();

        for id in 0..num_workers {
            let worker = Worker::new(
                id,
                Arc::clone(&queue),
                Arc::clone(&storage),
                Arc::clone(&pipeline),
                shutdown_tx.subscribe(),
            );

            let handle = tokio::spawn(async move {
                worker.run().await;
            });

            workers.push(handle);
        }

        Self {
            workers,
            shutdown_tx,
        }
    }

    pub async fn shutdown(self) {
        // Signal shutdown
        let _ = self.shutdown_tx.send(());

        // Wait for all workers to finish
        for handle in self.workers {
            let _ = handle.await;
        }
    }
}

struct Worker {
    id: usize,
    queue: Arc<dyn TaskQueue>,
    storage: Arc<dyn TaskStorage>,
    pipeline: Arc<Pipeline>,
    shutdown_rx: broadcast::Receiver<()>,
}

impl Worker {
    async fn run(mut self) {
        tracing::info!("Worker {} started", self.id);

        loop {
            tokio::select! {
                _ = self.shutdown_rx.recv() => {
                    tracing::info!("Worker {} shutting down", self.id);
                    break;
                }
                task_result = self.queue.receive() => {
                    match task_result {
                        Ok(task) => {
                            self.process_task(task).await;
                        }
                        Err(Error::QueueEmpty) => {
                            tokio::time::sleep(Duration::from_millis(100)).await;
                        }
                        Err(e) => {
                            tracing::error!("Worker {} queue error: {}", self.id, e);
                            tokio::time::sleep(Duration::from_secs(1)).await;
                        }
                    }
                }
            }
        }
    }

    async fn process_task(&self, mut task: Task) {
        tracing::info!("Worker {} processing task {}", self.id, task.track_id);

        // Update status to processing
        task.status = TaskStatus::Processing;
        task.started_at = Some(Utc::now());
        let _ = self.storage.update_task(&task).await;

        // Execute task
        let result = match task.task_type {
            TaskType::Upload => self.process_upload(&task).await,
            TaskType::Insert => self.process_insert(&task).await,
            TaskType::Scan => self.process_scan(&task).await,
            TaskType::Reindex => self.process_reindex(&task).await,
        };

        // Update final status
        match result {
            Ok(result_data) => {
                task.status = TaskStatus::Indexed;
                task.completed_at = Some(Utc::now());
                task.result = Some(result_data);
            }
            Err(e) => {
                task.status = TaskStatus::Failed;
                task.completed_at = Some(Utc::now());
                task.error_message = Some(e.to_string());

                tracing::error!(
                    "Worker {} task {} failed: {}",
                    self.id,
                    task.track_id,
                    e
                );
            }
        }

        task.updated_at = Utc::now();
        let _ = self.storage.update_task(&task).await;
    }

    async fn process_upload(&self, task: &Task) -> Result<serde_json::Value, Error> {
        // Extract file_path from task_data
        let data: UploadTaskData = serde_json::from_value(task.task_data.clone())?;

        // Read file content
        let content = tokio::fs::read_to_string(&data.file_path).await?;

        // Process through pipeline
        let result = self.pipeline.process(&data.document_id, &content).await?;

        // Return result data
        Ok(serde_json::json!({
            "document_id": data.document_id,
            "chunk_count": result.stats.chunk_count,
            "entity_count": result.stats.entity_count,
            "relationship_count": result.stats.relationship_count,
        }))
    }

    // Similar implementations for insert, scan, reindex...
}
```

### Track ID Generation

```rust
// crates/edgequake-tasks/src/track_id.rs

use uuid::Uuid;

pub fn generate_track_id(task_type: &TaskType) -> String {
    let prefix = match task_type {
        TaskType::Upload => "upload",
        TaskType::Insert => "insert",
        TaskType::Scan => "scan",
        TaskType::Reindex => "reindex",
    };

    let uuid = Uuid::new_v4();
    format!("{}-{}", prefix, uuid)
}

// Example: "upload-a1b2c3d4-e5f6-4789-a0b1-c2d3e4f5g6h7"
```

---

## Modified Document Upload Endpoint

```rust
// crates/edgequake-api/src/handlers/documents.rs

#[utoipa::path(
    post,
    path = "/api/v1/documents",
    tag = "Documents",
    request_body = UploadDocumentRequest,
    responses(
        (status = 202, description = "Document accepted for processing", body = UploadDocumentResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn upload_document(
    State(state): State<AppState>,
    Json(request): Json<UploadDocumentRequest>,
) -> ApiResult<(StatusCode, Json<UploadDocumentResponse>)> {
    // Validate
    if request.content.trim().is_empty() {
        return Err(ApiError::ValidationError("Content cannot be empty".to_string()));
    }

    // Generate document ID
    let document_id = Uuid::new_v4().to_string();

    // Create background task
    let task_data = serde_json::json!({
        "document_id": document_id,
        "content": request.content,
        "title": request.title,
        "metadata": request.metadata,
    });

    let track_id = state.task_service
        .create_task(TaskType::Insert, task_data, None)
        .await?;

    // Return immediately with track_id
    Ok((
        StatusCode::ACCEPTED,
        Json(UploadDocumentResponse {
            document_id,
            track_id,
            status: "accepted".to_string(),
            message: "Document accepted for processing. Use track_id to check status.".to_string(),
        }),
    ))
}
```

---

## Configuration

```toml
# config/default.toml

[tasks]
# Worker pool size
num_workers = 4

# Task queue capacity (for in-memory queue)
queue_capacity = 1000

# Task retention (days)
completed_task_retention_days = 7
failed_task_retention_days = 30

# Retry configuration
default_max_retries = 3
retry_backoff_seconds = [1, 5, 15]  # Exponential backoff

# Queue backend
queue_backend = "channel"  # Options: "channel", "redis"
redis_url = "redis://localhost:6379"
redis_queue_name = "edgequake:tasks"
```

---

## Monitoring & Metrics

### Prometheus Metrics

```rust
use prometheus::{IntCounter, IntGauge, Histogram};

lazy_static! {
    // Task counters
    static ref TASKS_CREATED: IntCounter = register_int_counter!(
        "edgequake_tasks_created_total",
        "Total number of tasks created"
    ).unwrap();

    static ref TASKS_COMPLETED: IntCounter = register_int_counter!(
        "edgequake_tasks_completed_total",
        "Total number of tasks completed successfully"
    ).unwrap();

    static ref TASKS_FAILED: IntCounter = register_int_counter!(
        "edgequake_tasks_failed_total",
        "Total number of tasks failed"
    ).unwrap();

    // Queue metrics
    static ref QUEUE_SIZE: IntGauge = register_int_gauge!(
        "edgequake_task_queue_size",
        "Current number of tasks in queue"
    ).unwrap();

    // Task duration
    static ref TASK_DURATION: Histogram = register_histogram!(
        "edgequake_task_duration_seconds",
        "Task processing duration in seconds"
    ).unwrap();

    // Worker metrics
    static ref ACTIVE_WORKERS: IntGauge = register_int_gauge!(
        "edgequake_active_workers",
        "Number of active worker threads"
    ).unwrap();
}
```

### Tracing

```rust
use tracing::{info, error, instrument};

#[instrument(skip(self, task), fields(track_id = %task.track_id, task_type = ?task.task_type))]
async fn process_task(&self, task: Task) -> Result<(), Error> {
    info!("Starting task processing");

    let start = Instant::now();
    let result = self.execute_task(&task).await;
    let duration = start.elapsed();

    match result {
        Ok(_) => {
            info!(duration_ms = duration.as_millis(), "Task completed successfully");
            TASKS_COMPLETED.inc();
        }
        Err(e) => {
            error!(error = %e, duration_ms = duration.as_millis(), "Task failed");
            TASKS_FAILED.inc();
        }
    }

    TASK_DURATION.observe(duration.as_secs_f64());

    result
}
```

---

## Testing

### Unit Tests

```rust
#[tokio::test]
async fn test_task_creation() {
    let queue = Arc::new(ChannelTaskQueue::new(100));
    let storage = Arc::new(MockTaskStorage::new());
    let service = TaskService::new(queue, storage);

    let track_id = service
        .create_task(
            TaskType::Insert,
            serde_json::json!({"text": "test"}),
            None,
        )
        .await
        .unwrap();

    assert!(track_id.starts_with("insert-"));
}

#[tokio::test]
async fn test_worker_processes_task() {
    let queue = Arc::new(ChannelTaskQueue::new(100));
    let storage = Arc::new(MockTaskStorage::new());
    let pipeline = Arc::new(MockPipeline::new());

    let pool = WorkerPool::new(1, queue.clone(), storage.clone(), pipeline);

    // Create task
    let task = Task {
        track_id: "test-123".to_string(),
        task_type: TaskType::Insert,
        status: TaskStatus::Pending,
        task_data: serde_json::json!({"text": "test content"}),
        ..Default::default()
    };

    storage.create_task(&task).await.unwrap();
    queue.send(task).await.unwrap();

    // Wait for processing
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Verify task completed
    let updated_task = storage.get_task("test-123").await.unwrap();
    assert_eq!(updated_task.status, TaskStatus::Indexed);

    pool.shutdown().await;
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_end_to_end_document_upload() {
    let app = test_app().await;

    // Upload document
    let response = app
        .post("/api/v1/documents")
        .json(&json!({
            "content": "Test document content",
            "title": "Test Doc"
        }))
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::ACCEPTED);

    let body: UploadDocumentResponse = response.json().await;
    assert!(body.track_id.starts_with("insert-"));

    // Poll for completion
    for _ in 0..10 {
        tokio::time::sleep(Duration::from_millis(500)).await;

        let status_response = app
            .get(&format!("/api/v1/tasks/{}", body.track_id))
            .send()
            .await;

        let status: TaskStatusResponse = status_response.json().await;

        if status.status == "indexed" {
            assert!(status.result.is_some());
            return;
        }
    }

    panic!("Task did not complete in time");
}
```

---

## Migration Guide

### From v1.0 (Synchronous) to v1.1 (Async)

**Old Code (v1.0):**

```rust
let response = client
    .post("/api/v1/documents")
    .json(&doc)
    .send()
    .await?;

assert_eq!(response.status(), StatusCode::CREATED);
let body: UploadDocumentResponse = response.json().await?;

// Document is fully processed
println!("Entities: {}", body.entity_count);
```

**New Code (v1.1):**

```rust
let response = client
    .post("/api/v1/documents")
    .json(&doc)
    .send()
    .await?;

assert_eq!(response.status(), StatusCode::ACCEPTED);  // Changed from CREATED
let body: UploadDocumentResponse = response.json().await?;

// Document is queued, not yet processed
let track_id = body.track_id;

// Poll for completion
loop {
    let status = client
        .get(&format!("/api/v1/tasks/{}", track_id))
        .send()
        .await?
        .json::<TaskStatusResponse>()
        .await?;

    match status.status.as_str() {
        "indexed" => {
            println!("Entities: {}", status.result.unwrap()["entity_count"]);
            break;
        }
        "failed" => {
            eprintln!("Error: {}", status.error_message.unwrap());
            break;
        }
        _ => {
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }
}
```

---

## Security Considerations

1. **Track ID Privacy:** Track IDs are UUID-based and unpredictable
2. **Task Isolation:** Tasks cannot access other users' data (enforced in v2.0 with tenants)
3. **Resource Limits:** Queue capacity prevents DoS attacks
4. **Input Validation:** Task payloads are validated before processing
5. **Error Messages:** Sanitized error messages prevent information leakage

---

## Performance Targets

| Metric                | Target          | Measurement        |
| --------------------- | --------------- | ------------------ |
| Task Creation Latency | <50ms           | p95                |
| Queue Throughput      | 1000+ tasks/sec | Sustained          |
| Worker CPU Usage      | <80%            | Per-worker average |
| Memory per Task       | <1MB            | Average            |
| Task Storage Size     | <10KB           | Per task record    |

---

## Future Enhancements (Post v1.1)

1. **Priority Queues:** High/medium/low priority tasks
2. **Task Dependencies:** Task A must complete before Task B
3. **Scheduled Tasks:** Run tasks at specific times
4. **Task Webhooks:** Notify external services on completion
5. **Task Chaining:** Automatically trigger follow-up tasks
6. **Dead Letter Queue:** Separate queue for permanently failed tasks

---

**Status:** ✅ Specification Complete - Ready for Implementation  
**Next Steps:** Begin implementation of TaskQueue and Worker Pool
