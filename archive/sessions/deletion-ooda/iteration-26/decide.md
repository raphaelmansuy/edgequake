# OODA-26 Decide: Manual Metrics Trigger Endpoint

## Action: Add POST /api/v1/workspaces/{id}/metrics-snapshot

### 1. Endpoint Details

- **Method**: POST
- **Path**: `/api/v1/workspaces/{workspace_id}/metrics-snapshot`
- **Auth**: Required (API key or session)
- **Body**: None required
- **Response**: The created MetricsSnapshotDTO

### 2. Implementation Steps

1. Add handler function in workspaces.rs
2. Add route in routes.rs
3. Add E2E tests

### 3. Handler Logic

```rust
pub async fn trigger_metrics_snapshot(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
) -> ApiResult<Json<MetricsSnapshotDTO>> {
    let snapshot = state
        .workspace_service
        .record_metrics_snapshot(workspace_id, MetricsTriggerType::Manual)
        .await?;

    Ok(Json(MetricsSnapshotDTO::from(snapshot)))
}
```

### 4. Test Cases

1. Successfully creates snapshot with trigger_type=manual
2. Returns correct response format
3. 404 for non-existent workspace
