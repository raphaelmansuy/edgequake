# OODA Iteration 19: DECIDE

**Date**: 2025-01-27
**Mission Re-Read**: ✅ YES - `/specs/001-improve-ingestion-process.md`

---

## Decision: Add QueueMetrics Types and Trait Method

### Action Plan

```
┌────────────────────────────────────────────────────────────────────────┐
│                    ITERATION 19 ACTION PLAN                           │
├────────────────────────────────────────────────────────────────────────┤
│                                                                        │
│  STEP 1: Add QueueMetrics struct to storage.rs                        │
│  ├── Define struct with all required fields                          │
│  ├── Implement Default trait                                          │
│  └── Add documentation with ASCII diagram                             │
│                                                                        │
│  STEP 2: Add get_queue_metrics() to TaskStorage trait                 │
│  ├── Add async method signature                                       │
│  └── Document the method                                               │
│                                                                        │
│  STEP 3: Implement for MemoryTaskStorage                              │
│  ├── Calculate metrics from in-memory tasks                           │
│  └── Handle edge cases (empty queue)                                   │
│                                                                        │
│  STEP 4: Build and test                                               │
│  ├── cargo build -p edgequake-tasks                                   │
│  └── cargo test -p edgequake-tasks                                    │
│                                                                        │
└────────────────────────────────────────────────────────────────────────┘
```

---

## File Changes

### storage.rs Changes

**Location**: After `TaskStatistics` struct (~line 85)

**Add**:

```rust
/// Queue-level metrics for workspace processing visibility.
///
/// @implements SPEC-001/Objective-B: Workspace-Level Task Queue Visibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueMetrics {
    pub pending_count: u64,
    pub processing_count: u64,
    pub active_workers: u32,
    pub max_workers: u32,
    pub worker_utilization: u8,
    pub avg_wait_time_seconds: f64,
    pub max_wait_time_seconds: f64,
    pub throughput_per_minute: f64,
    pub estimated_queue_time_seconds: f64,
    pub rate_limited: bool,
    pub timestamp: DateTime<Utc>,
}
```

**Add to trait**: After `get_statistics()`

```rust
/// Get queue metrics for task queue visibility
///
/// @implements SPEC-001/Objective-B
async fn get_queue_metrics(&self) -> TaskResult<QueueMetrics>;
```

---

## Validation Criteria

1. ✅ `cargo build -p edgequake-tasks` succeeds
2. ✅ `cargo test -p edgequake-tasks` passes
3. ✅ No clippy warnings
4. ✅ QueueMetrics is serializable

---

## Proceed to ACT
