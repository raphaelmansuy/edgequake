# OODA-21 Observe: Integrate Metrics Snapshot Recording

## Mission Context

OODA-20 created the `record_metrics_snapshot` function. Now we need to call it at the right places:

1. **After document upload completes** - track growth
2. **After document deletion completes** - track reduction

## Integration Points

### 1. delete_document Handler (line ~1920)

After successful deletion, before returning:
```rust
// Record metrics snapshot for trend analysis
// OODA-21: Best-effort - log error but don't fail deletion
if let Ok(workspace_id) = uuid::Uuid::parse_str(&workspace_id_for_storage) {
    if let Err(e) = state.workspace_service
        .record_metrics_snapshot(workspace_id, MetricsTriggerType::Event)
        .await
    {
        tracing::warn!(error = %e, "Failed to record deletion metrics snapshot");
    }
}
```

### 2. upload_document Handler (find the upload endpoint)

Similarly, after successful upload processing.

## Considerations

- **Best-effort**: Metrics recording failure should not fail the main operation
- **Workspace ID**: Need to parse workspace ID string to UUID
- **Storage Mode**: In-memory mode will just log zeros (stub implementation)
