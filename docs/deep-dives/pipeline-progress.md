# Deep Dive: Pipeline Progress Tracking

> **Real-Time Monitoring of Document Ingestion**

EdgeQuake provides comprehensive progress tracking for document ingestion, enabling real-time UI updates, error handling, and ETA estimation.

---

## Overview

The progress tracking system monitors each stage of the ingestion pipeline:

```
┌─────────────────────────────────────────────────────────────────┐
│                    PIPELINE PROGRESS FLOW                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Document Upload                                                │
│       │                                                         │
│       ▼                                                         │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │ 1. Preprocessing    ████████████████░░░░  80%  [Running]    ││
│  │    Parsing PDF, extracting text...                          ││
│  └─────────────────────────────────────────────────────────────┘│
│       │                                                         │
│       ▼                                                         │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │ 2. Chunking         ░░░░░░░░░░░░░░░░░░░░   0%  [Pending]    ││
│  │    Waiting...                                               ││
│  └─────────────────────────────────────────────────────────────┘│
│       │                                                         │
│       ▼                                                         │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │ 3. Extracting       ░░░░░░░░░░░░░░░░░░░░   0%  [Pending]    ││
│  │    Waiting...                                               ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                 │
│  Overall: ████░░░░░░░░░░░░░░░░  20%   ETA: ~45 seconds          │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Why Progress Tracking?

| Purpose             | Benefit                                |
| ------------------- | -------------------------------------- |
| **User Experience** | Visual feedback during long operations |
| **Error Recovery**  | Know where failures occurred           |
| **Debugging**       | Detailed logs per stage                |
| **Optimization**    | Identify slow stages                   |
| **SLA Monitoring**  | Track processing times                 |

---

## Core Data Structures

### IngestionStatus

Overall job status:

```rust
/// Overall ingestion status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IngestionStatus {
    /// Waiting to start
    Pending,
    /// Currently processing
    Running,
    /// Successfully completed
    Completed,
    /// Failed with errors
    Failed,
    /// Cancelled by user
    Cancelled,
}
```

### PipelineStage

The 9 stages of document processing:

```rust
/// Pipeline processing stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineStage {
    Preprocessing,  // Validation, parsing
    Chunking,       // Text segmentation
    Extracting,     // Entity/relationship extraction
    Gleaning,       // Re-extraction for completeness
    Merging,        // Graph integration
    Summarizing,    // Description generation
    Embedding,      // Vector generation
    Storing,        // Database persistence
    Finalizing,     // Cleanup and completion
}
```

### StageProgress

Progress within a single stage:

```rust
/// Progress for a single pipeline stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageProgress {
    /// The stage
    pub stage: PipelineStage,

    /// Current status
    pub status: StageStatus,

    /// Total items to process
    pub total_items: usize,

    /// Items completed
    pub completed_items: usize,

    /// Completion percentage (0-100)
    pub completion_percentage: f32,

    /// When stage started
    pub started_at: Option<DateTime<Utc>>,

    /// When stage completed
    pub completed_at: Option<DateTime<Utc>>,
}
```

### IngestionProgress

Complete job snapshot:

```rust
/// Complete ingestion progress snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionProgress {
    /// Job identifier
    pub job_id: String,

    /// Document identifier
    pub document_id: String,

    /// Overall status
    pub status: IngestionStatus,

    /// Current stage
    pub current_stage: PipelineStage,

    /// Progress for each stage
    pub stages: Vec<StageProgress>,

    /// Overall completion percentage
    pub completion_percentage: f32,

    /// Estimated time remaining (seconds)
    pub eta_seconds: Option<u64>,

    /// Latest status message
    pub latest_message: String,

    /// Message history
    pub history_messages: Vec<ProgressMessage>,

    /// Errors encountered
    pub errors: Vec<IngestionError>,

    /// Timestamps
    pub started_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}
```

---

## The Progress Tracker

Thread-safe wrapper for concurrent updates:

```rust
/// Thread-safe progress tracker.
pub struct ProgressTracker {
    inner: Arc<RwLock<IngestionProgress>>,
}

impl ProgressTracker {
    /// Create a new progress tracker
    pub fn new(job_id: String, document_id: String) -> Self;

    /// Start the job
    pub async fn start(&self);

    /// Set current stage and item count
    pub async fn set_stage(&self, stage: PipelineStage, total_items: usize);

    /// Update stage progress
    pub async fn update_stage(&self, stage: PipelineStage, completed: usize);

    /// Complete a stage
    pub async fn complete_stage(&self, stage: PipelineStage);

    /// Skip a stage
    pub async fn skip_stage(&self, stage: PipelineStage);

    /// Add a message
    pub async fn add_message(&self, message: String, level: MessageLevel);

    /// Add an error
    pub async fn add_error(&self, error: IngestionError);

    /// Complete the job
    pub async fn complete(&self);

    /// Fail the job
    pub async fn fail(&self, error: IngestionError);

    /// Get current snapshot
    pub async fn snapshot(&self) -> IngestionProgress;
}
```

---

## Usage

### Basic Progress Tracking

```rust
use edgequake_pipeline::progress::{ProgressTracker, PipelineStage, MessageLevel};

// Create tracker for a job
let tracker = ProgressTracker::new("job-123", "doc-456");

// Start the job
tracker.start().await;

// Set current stage
tracker.set_stage(PipelineStage::Chunking, 10).await;
tracker.add_message("Chunking document into 10 segments", MessageLevel::Info).await;

// Update progress as chunks are processed
for i in 0..10 {
    process_chunk(i).await;
    tracker.update_stage(PipelineStage::Chunking, i + 1).await;
}

// Complete the stage
tracker.complete_stage(PipelineStage::Chunking).await;

// Move to next stage
tracker.set_stage(PipelineStage::Extracting, 10).await;
```

### Error Handling

```rust
use edgequake_pipeline::progress::{IngestionError, PipelineStage};

// Create an error
let error = IngestionError::new(
    "E001",
    "LLM rate limit exceeded",
    PipelineStage::Extracting,
)
.with_details("429 Too Many Requests")
.with_item_id("chunk-7")
.recoverable();

// Add to tracker
tracker.add_error(error).await;

// Or fail the entire job
tracker.fail(error).await;
```

### Get Progress Snapshot

```rust
// Get current state for API response
let progress = tracker.snapshot().await;

println!("Job: {}", progress.job_id);
println!("Status: {:?}", progress.status);
println!("Stage: {:?}", progress.current_stage);
println!("Overall: {:.1}%", progress.completion_percentage);

// Check individual stages
for stage in &progress.stages {
    println!("{}: {:?} ({:.1}%)",
             stage.stage.name(),
             stage.status,
             stage.completion_percentage);
}
```

---

## Message Levels

```rust
/// Message severity level.
pub enum MessageLevel {
    Debug,    // Verbose debugging
    Info,     // Normal progress
    Warning,  // Non-fatal issues
    Error,    // Errors (job may continue)
}
```

**Examples:**

```rust
// Info: Normal progress
tracker.add_message("Processing chunk 5 of 10", MessageLevel::Info).await;

// Warning: Non-critical issue
tracker.add_message("Duplicate entity detected, merging", MessageLevel::Warning).await;

// Error: Problem but continuing
tracker.add_message("Failed to extract from chunk 7, skipping", MessageLevel::Error).await;
```

---

## Completion Percentage Calculation

Overall progress is calculated across all stages:

```rust
impl IngestionProgress {
    pub fn calculate_completion(&mut self) {
        let total_stages = self.stages.len() as f32;

        let completed: f32 = self.stages.iter().map(|s| {
            match s.status {
                StageStatus::Completed | StageStatus::Skipped => 1.0,
                StageStatus::Running => s.completion_percentage / 100.0,
                _ => 0.0,
            }
        }).sum();

        self.completion_percentage = (completed / total_stages) * 100.0;
    }
}
```

**Example:**

- 3 stages completed (3.0)
- 1 stage at 50% (0.5)
- 5 stages pending (0.0)
- Total: (3.5 / 9) × 100 = 38.9%

---

## API Integration

### Progress Streaming (SSE)

```bash
# Subscribe to progress updates
curl -N "http://localhost:8080/api/v1/rag/progress/job-123/stream"

# Receives events:
data: {"job_id":"job-123","status":"Running","completion_percentage":25.5}

data: {"job_id":"job-123","status":"Running","completion_percentage":50.0}

data: {"job_id":"job-123","status":"Completed","completion_percentage":100.0}
```

### Polling Endpoint

```bash
# Get current progress
curl "http://localhost:8080/api/v1/rag/progress/job-123"

{
  "job_id": "job-123",
  "status": "Running",
  "current_stage": "Extracting",
  "completion_percentage": 45.2,
  "eta_seconds": 30,
  "latest_message": "Extracting entities from chunk 5 of 10",
  "stages": [
    {"stage": "Preprocessing", "status": "Completed", "completion_percentage": 100.0},
    {"stage": "Chunking", "status": "Completed", "completion_percentage": 100.0},
    {"stage": "Extracting", "status": "Running", "completion_percentage": 50.0},
    ...
  ],
  "errors": []
}
```

---

## Frontend Integration

### React Progress Component

```tsx
function IngestionProgress({ jobId }: { jobId: string }) {
  const [progress, setProgress] = useState<Progress | null>(null);

  useEffect(() => {
    // SSE subscription
    const eventSource = new EventSource(`/api/v1/rag/progress/${jobId}/stream`);

    eventSource.onmessage = (event) => {
      setProgress(JSON.parse(event.data));
    };

    return () => eventSource.close();
  }, [jobId]);

  if (!progress) return <Loading />;

  return (
    <div className="progress-container">
      <h3>Processing: {progress.document_id}</h3>

      {/* Overall progress bar */}
      <ProgressBar value={progress.completion_percentage} max={100} />

      {/* Stage breakdown */}
      {progress.stages.map((stage) => (
        <StageRow key={stage.stage} stage={stage} />
      ))}

      {/* Latest message */}
      <p className="status-message">{progress.latest_message}</p>

      {/* ETA */}
      {progress.eta_seconds && (
        <p>ETA: {formatDuration(progress.eta_seconds)}</p>
      )}
    </div>
  );
}
```

---

## Best Practices

1. **Update Frequently** - Call `update_stage()` after each item
2. **Meaningful Messages** - Include counts ("Processing 5 of 10")
3. **Handle Skipped Stages** - Mark as skipped, not just skip
4. **Log Errors** - Even for recoverable errors
5. **Clean Up** - Always call `complete()` or `fail()`

---

## Performance Considerations

### RwLock Contention

```rust
// Good: Few writes, many reads
tracker.update_stage(stage, completed).await;  // Brief write lock

let snapshot = tracker.snapshot().await;  // Read lock (concurrent OK)
```

### Message History Size

```rust
// Consider trimming old messages for long jobs
if progress.history_messages.len() > 100 {
    progress.history_messages.drain(0..50);
}
```

---

## Troubleshooting

### Progress Not Updating

**Check:**

1. Tracker is shared across tasks (use `Arc<ProgressTracker>`)
2. `update_stage()` is called after each item
3. No deadlocks on the RwLock

### Missing Stages

**Cause:** Stage skipped without notification

**Solution:**

```rust
// Always mark skipped stages
if !needs_gleaning {
    tracker.skip_stage(PipelineStage::Gleaning).await;
}
```

### Incorrect Completion %

**Cause:** Total items set incorrectly

**Solution:**

```rust
// Set correct total before starting stage
let chunk_count = chunker.estimate_chunks(&content);
tracker.set_stage(PipelineStage::Chunking, chunk_count).await;
```

---

## See Also

- [Cost Tracking](./cost-tracking.md) - LLM cost monitoring
- [Operations: Monitoring](../operations/monitoring.md) - Production observability
- [REST API](../api-reference/rest-api.md) - Progress endpoints
