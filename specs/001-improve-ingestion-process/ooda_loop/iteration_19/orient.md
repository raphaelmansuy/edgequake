# OODA Iteration 19: ORIENT

**Date**: 2025-01-27
**Mission Re-Read**: ✅ YES - `/specs/001-improve-ingestion-process.md`

---

## Gap Analysis

### Current vs Required (Objective B)

| Requirement                | Current State                | Gap                  |
| -------------------------- | ---------------------------- | -------------------- |
| Document counts by status  | ✅ TaskStatistics has counts | None                 |
| Queue depth                | ✅ TaskStatistics.pending    | None                 |
| Queue order                | ❌ Not exposed               | Need sorted list     |
| Wait time per document     | ❌ Not calculated            | Need calculation     |
| Average wait time          | ❌ Not calculated            | Need aggregation     |
| Processing rate (docs/min) | ❌ Not tracked               | Need throughput calc |
| Active workers             | ❌ Not tracked               | Need worker state    |
| Rate limiting status       | ❌ Not tracked               | Need error tracking  |

---

## Design Decision: QueueMetrics Struct

### Option A: Extend TaskStatistics

Add fields to existing struct.

- ❌ Changes existing API response shape
- ❌ Mixes concerns (counts vs metrics)

### Option B: New QueueMetrics Struct (CHOSEN)

Create dedicated struct for queue metrics.

- ✅ Backward compatible
- ✅ Clear separation of concerns
- ✅ Can be fetched independently

---

## QueueMetrics Design

````rust
/// Queue-level metrics for workspace processing visibility.
///
/// @implements SPEC-001/Objective-B: Workspace-Level Task Queue Visibility
///
/// WHY: Users need visibility into the task queue to understand:
/// - How many documents are waiting
/// - How long they'll have to wait
/// - What the system throughput is
///
/// ```text
/// ┌────────────────────────────────────────────────────────────────┐
/// │ WORKSPACE: default-workspace                                   │
/// ├────────────────────────────────────────────────────────────────┤
/// │ Documents:  Pending: 12  Processing: 3  Completed: 156        │
/// │             Failed: 2    Cancelled: 0                          │
/// ├────────────────────────────────────────────────────────────────┤
/// │ Throughput: 2.3 docs/min | Avg wait: 1m 42s                   │
/// └────────────────────────────────────────────────────────────────┘
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueMetrics {
    /// Documents waiting to be processed.
    pub pending_count: u64,

    /// Documents currently being processed.
    pub processing_count: u64,

    /// Active concurrent workers.
    pub active_workers: u32,

    /// Maximum concurrent workers allowed.
    pub max_workers: u32,

    /// Worker utilization percentage (0-100).
    pub worker_utilization: u8,

    /// Average wait time in seconds (time from created to started).
    pub avg_wait_time_seconds: f64,

    /// Maximum wait time in queue (oldest pending task).
    pub max_wait_time_seconds: f64,

    /// Documents processed per minute (rolling 5-minute average).
    pub throughput_per_minute: f64,

    /// Estimated time for new document to start processing.
    pub estimated_queue_time_seconds: f64,

    /// Whether rate limiting is currently active.
    pub rate_limited: bool,

    /// Timestamp of this metrics snapshot.
    pub timestamp: DateTime<Utc>,
}
````

---

## Implementation Approach

### Phase 1: Backend Types (This Iteration)

1. Add `QueueMetrics` struct to `storage.rs`
2. Add `get_queue_metrics()` to `TaskStorage` trait
3. Implement for memory storage

### Phase 2: API Endpoint (Next Iteration)

1. Add `/workspaces/{id}/queue-metrics` endpoint
2. Expose via REST API

### Phase 3: Frontend (Later Iteration)

1. Create `QueueMetricsCard` component
2. Integrate into Pipeline Monitor

---

## Calculation Methods

### Average Wait Time

```rust
avg_wait_time = sum(started_at - created_at) / count
// For pending tasks, use current_time - created_at
```

### Throughput

```rust
// Count tasks completed in last 5 minutes
completed_in_window = tasks.filter(|t|
    t.status == Indexed &&
    t.completed_at > now - 5.minutes()
).count();

throughput_per_minute = completed_in_window / 5.0;
```

### Estimated Queue Time

```rust
// Position in queue × average processing time
position = pending_tasks_before_this_one;
estimated = position * avg_processing_time;
```

---

## Next Step

DECIDE → Finalize implementation plan and file changes.
