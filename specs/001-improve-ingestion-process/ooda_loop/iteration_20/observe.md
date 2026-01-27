# Iteration 20: Observe

## Mission Reference

Re-read: `/specs/001-improve-ingestion-process.md`

**Objective B: Workspace-Level Task Queue Visibility**

- Queue depth, order, wait time per document
- Processing rate (docs/min), worker utilization
- Average wait time, ETA for queue

## Current State

### QueueMetrics Struct (from Iteration 19)

Location: `edgequake-tasks/src/storage.rs:124`

```rust
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

### TaskStorage Trait Method

Location: `edgequake-tasks/src/storage.rs:39`

```rust
async fn get_queue_metrics(&self) -> TaskResult<QueueMetrics>;
```

### Implementations

1. **MemoryTaskStorage**: `edgequake-tasks/src/memory.rs:149`
2. **PostgresTaskStorage**: `edgequake-tasks/src/postgres.rs:289`

### API Pattern Analysis

Studied existing endpoints:

1. `/api/v1/pipeline/status` - Returns pipeline status + task stats
2. `/api/v1/tasks` - Lists tasks with statistics
3. `/api/v1/workspaces/{id}/stats` - Workspace statistics

#### Handler Pattern

```rust
pub async fn handler_name(
    State(state): State<AppState>,
) -> ApiResult<Json<ResponseDto>> {
    let result = state.task_storage.method().await
        .map_err(|e| ApiError::Internal(...))?;
    Ok(Json(ResponseDto::from(result)))
}
```

### AppState Access

- `state.task_storage` - SharedTaskStorage (Arc<dyn TaskStorage>)
- Located in: `edgequake-api/src/state.rs:189`

## Files to Modify

1. **pipeline_types.rs** - Add QueueMetricsResponse DTO
2. **pipeline.rs** - Add `get_queue_metrics` handler
3. **routes.rs** - Add route `/api/v1/pipeline/queue-metrics`
4. **mod.rs** - Ensure exports are correct

## Design Decision

Route choice: `/api/v1/pipeline/queue-metrics`

- Consistent with existing `/api/v1/pipeline/status`
- Queue metrics are pipeline-level (not workspace-specific)
- Follows established naming convention
