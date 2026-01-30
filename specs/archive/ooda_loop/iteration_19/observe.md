# OODA Iteration 19: OBSERVE

**Date**: 2025-01-27
**Mission Re-Read**: ✅ YES - `/specs/001-improve-ingestion-process.md` lines 97-140

---

## Observation Focus: Workspace-Level Task Queue Visibility (Objective B)

### Mission Requirements (Objective B)

From spec lines 97-140:

```
**REQUIRED METRICS (per workspace):**

1. **Document counts by status**:
   - Pending (queued, waiting to start)
   - Processing (actively being ingested)
   - Completed (successfully indexed)
   - Failed (errors during processing)
   - Cancelled (user-cancelled)

2. **Task Queue Visualization**:
   - Queue depth: How many documents waiting
   - Queue order: Which document is next
   - Wait time per document: How long has each been waiting
   - Average wait time: Typical queue delay
   - Processing rate: Documents/minute throughput

3. **Worker Status**:
   - Active workers: How many concurrent extractions
   - Worker utilization: % capacity used
   - Rate limiting: If hitting API limits
```

---

### Current State Analysis

#### 1. TaskStatistics (storage.rs:77-85)

**Current Structure**:

```rust
pub struct TaskStatistics {
    pub pending: u64,
    pub processing: u64,
    pub indexed: u64,
    pub failed: u64,
    pub cancelled: u64,
    pub total: u64,
}
```

**Gap**: Only has counts, missing:

- Average wait time
- Processing rate (throughput)
- Queue order
- Wait time per document

#### 2. TaskStorage Trait (storage.rs:10-26)

**Current Methods**:

```rust
pub trait TaskStorage: Send + Sync {
    async fn create_task(&self, task: &Task) -> TaskResult<()>;
    async fn get_task(&self, track_id: &str) -> TaskResult<Option<Task>>;
    async fn update_task(&self, task: &Task) -> TaskResult<()>;
    async fn delete_task(&self, track_id: &str) -> TaskResult<()>;
    async fn list_tasks(&self, filter: TaskFilter, pagination: Pagination) -> TaskResult<TaskList>;
    async fn get_statistics(&self) -> TaskResult<TaskStatistics>;
}
```

**Gap**: No method for queue-specific metrics like:

- `get_queue_metrics()` → Queue depth, order, wait times
- `get_throughput()` → Processing rate calculation

#### 3. Task Struct (types.rs:63-109)

**Existing Fields Useful for Queue Metrics**:

```rust
pub created_at: DateTime<Utc>,    // ← Can calculate wait time
pub started_at: Option<DateTime<Utc>>,  // ← Processing start
pub completed_at: Option<DateTime<Utc>>,  // ← Processing end
pub status: TaskStatus,
```

**Already Have Data For**:

- Wait time = started_at - created_at
- Processing time = completed_at - started_at

#### 4. Pipeline State (pipeline_state.rs)

**Current State Tracking**:

```rust
struct PipelineStateInner {
    is_busy: bool,
    job_name: Option<String>,
    total_documents: u32,
    processed_documents: u32,
    // ...
}
```

**Missing for Worker Status**:

- Active workers count (concurrent tasks)
- Worker utilization percentage
- Rate limiting indicators

---

## Findings Summary

### Data Already Available (Just Needs Calculation)

| Metric            | Source                           | Calculation             |
| ----------------- | -------------------------------- | ----------------------- |
| Queue depth       | TaskStatistics.pending           | Direct                  |
| Wait time per doc | Task.created_at, Task.started_at | started_at - created_at |
| Avg wait time     | All pending/processing tasks     | Mean of wait times      |
| Processing rate   | Completed tasks + timestamps     | Count in last 60s       |

### Data Requires New Implementation

| Metric             | Required Change                    |
| ------------------ | ---------------------------------- |
| Queue order        | Sort pending tasks by created_at   |
| Worker count       | Track concurrent processing tasks  |
| Worker utilization | concurrent / max_workers           |
| Rate limit status  | Track 429 errors in last N minutes |

---

## Files Requiring Modification

| File                                       | Change Required                                     |
| ------------------------------------------ | --------------------------------------------------- |
| `edgequake-tasks/src/storage.rs`           | Add QueueMetrics struct, get_queue_metrics() method |
| `edgequake-tasks/src/types.rs`             | Add QueueMetrics export if needed                   |
| `edgequake-api/src/handlers/workspaces.rs` | Add queue status endpoint                           |

---

## Next Step

ORIENT → Design QueueMetrics struct and decide on implementation approach.
