# OODA-21 Observe + Orient + Decide + Act: Integrate Metrics Snapshot Recording

## Actions Taken

### 1. Added Import
Added `MetricsTriggerType` import to documents.rs:
```rust
use edgequake_core::MetricsTriggerType;
```

### 2. Added Metrics Recording in delete_document
After deletion completes (line ~1920):
```rust
// OODA-21: Record metrics snapshot for trend analysis after deletion
if let Ok(workspace_uuid) = Uuid::parse_str(&workspace_id_for_storage) {
    if let Err(e) = state
        .workspace_service
        .record_metrics_snapshot(workspace_uuid, MetricsTriggerType::Event)
        .await
    {
        tracing::warn!(...);
    }
}
```

### 3. Added Metrics Recording in upload_document (sync path)
After sync processing completes:
```rust
// OODA-21: Record metrics snapshot for trend analysis after upload
if let Ok(workspace_uuid) = Uuid::parse_str(&workspace_id_for_storage) {
    if let Err(e) = state
        .workspace_service
        .record_metrics_snapshot(workspace_uuid, MetricsTriggerType::Event)
        .await
    {
        tracing::warn!(...);
    }
}
```

## Design Decisions

1. **Best-effort recording**: Metrics failures are logged but don't fail the main operation
2. **workspace_id parsing**: Convert string to UUID, skip if invalid
3. **TriggerType::Event**: Both upload and delete are events (not scheduled)

## Files Modified

1. `crates/edgequake-api/src/handlers/documents.rs`

## Test Results

- 27/27 deletion tests pass
- Build succeeds

## Note

The async processing path (background task) is NOT integrated yet.
That would require modifying the task processor, which is a separate iteration.

## Commit

Pending: "feat(metrics): integrate snapshot recording in document handlers (OODA-21)"
