# OODA-26 Act: Manual Metrics Trigger Endpoint

## Actions Completed

### 1. Added Handler (workspaces.rs)

```rust
pub async fn trigger_metrics_snapshot(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
) -> Result<(StatusCode, Json<MetricsSnapshotDTO>), ApiError>
```

- Calls `record_metrics_snapshot` with `MetricsTriggerType::Manual`
- Returns 201 CREATED with MetricsSnapshotDTO

### 2. Added Route (routes.rs)

```
POST /api/v1/workspaces/{workspace_id}/metrics-snapshot
```

### 3. Added E2E Tests (e2e_metrics_history.rs)

| Test | Description |
|------|-------------|
| `test_trigger_metrics_snapshot_creates_snapshot` | Endpoint reachable |
| `test_trigger_metrics_snapshot_response_structure` | Response format |
| `test_trigger_metrics_snapshot_method_not_allowed` | Only POST allowed |

### 4. Test Results

- 8/8 metrics tests pass (5 history + 3 trigger)
- Route correctly registered
- Method restriction enforced

## Outcome

Users can now manually trigger a metrics snapshot for debugging or
external scheduler integration. This completes the three trigger types:

- Event: ✅ Automatic on upload/delete
- Manual: ✅ POST endpoint (this iteration)
- Scheduled: ⏳ Future background task

## Commit: feat(api): add manual metrics snapshot trigger endpoint
