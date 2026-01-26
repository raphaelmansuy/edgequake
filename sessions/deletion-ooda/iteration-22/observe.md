# OODA-22 Observe: Metrics History API Endpoint

## Mission Context

OODA-17 created the `workspace_metrics_history` table.
OODA-20 added `record_metrics_snapshot()` function.
OODA-21 integrated recording into handlers.

Now we need an API endpoint to query the history.

## Requirements

### 1. New Trait Method
```rust
async fn get_metrics_history(
    &self,
    workspace_id: Uuid,
    limit: usize,
    offset: usize,
) -> Result<Vec<MetricsSnapshot>>;
```

### 2. New API Endpoint
```
GET /api/v1/workspaces/{workspace_id}/metrics-history
Query params:
  - limit (default: 100)
  - offset (default: 0)
  - trigger_type (optional filter)
```

### 3. Response DTO
```rust
#[derive(Serialize)]
pub struct MetricsHistoryResponse {
    pub snapshots: Vec<MetricsSnapshotDTO>,
    pub total: usize,
}
```

## Files to Modify

1. `workspace_service.rs` - Add trait method
2. `workspace_service_impl.rs` - PostgreSQL implementation
3. `workspaces.rs` - API handler
