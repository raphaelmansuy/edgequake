# OODA-20 Act: Implement record_metrics_snapshot

## Completed Actions

### 1. Added Types to multitenancy.rs

**MetricsTriggerType enum**:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetricsTriggerType {
    Event,
    Scheduled,
    Manual,
}
```

**MetricsSnapshot struct**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub recorded_at: chrono::DateTime<chrono::Utc>,
    pub trigger_type: MetricsTriggerType,
    pub document_count: i64,
    pub chunk_count: i64,
    pub entity_count: i64,
    pub relationship_count: i64,
    pub embedding_count: i64,
    pub storage_bytes: i64,
}
```

### 2. Updated Exports in types/mod.rs

Added `MetricsSnapshot` and `MetricsTriggerType` to exports.

### 3. Updated Exports in lib.rs

Added `MetricsSnapshot` and `MetricsTriggerType` to public exports.

### 4. Extended WorkspaceService Trait

Added trait method:
```rust
async fn record_metrics_snapshot(
    &self,
    workspace_id: Uuid,
    trigger_type: MetricsTriggerType,
) -> Result<MetricsSnapshot>;
```

### 5. Implemented PostgreSQL Version

Uses INSERT ... RETURNING for atomic recording to workspace_metrics_history table (migration 016).

### 6. Added In-Memory Stub

Returns a zero-stat snapshot for testing compatibility.

## Files Modified

1. `crates/edgequake-core/src/types/multitenancy.rs` - Added types
2. `crates/edgequake-core/src/types/mod.rs` - Exports
3. `crates/edgequake-core/src/lib.rs` - Exports
4. `crates/edgequake-core/src/workspace_service.rs` - Trait + InMemory
5. `crates/edgequake-core/src/workspace_service_impl.rs` - PostgreSQL impl

## Test Results

- 24/24 core tests pass
- Build succeeds without warnings

## Commit

Pending: "feat(metrics): add record_metrics_snapshot function (OODA-20)"

## Next Steps

OODA-21: Integrate snapshot recording into document handlers
- Call record_metrics_snapshot after document upload
- Call record_metrics_snapshot after document deletion
