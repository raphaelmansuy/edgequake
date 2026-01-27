# OODA-20 Orient: Design for Metrics Snapshot

## Pattern Analysis

### 1. Current Stats Pattern

`get_workspace_stats()` returns a point-in-time snapshot. This should be the data source for recording.

### 2. Database Schema (migration 016)

```sql
workspace_metrics_history (
    id, workspace_id, recorded_at, trigger_type,
    document_count, chunk_count, entity_count, relationship_count,
    embedding_count, storage_bytes
)
```

### 3. Trigger Types

- `event`: After document add/delete
- `scheduled`: Hourly background task
- `manual`: Admin request

## Design Decisions

### A. Add New Type: TriggerType Enum

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum MetricsTriggerType {
    Event,
    Scheduled,
    Manual,
}
```

### B. Add New Type: MetricsSnapshot

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub recorded_at: chrono::DateTime<chrono::Utc>,
    pub trigger_type: MetricsTriggerType,
    pub document_count: u64,
    pub chunk_count: u64,
    pub entity_count: u64,
    pub relationship_count: u64,
    pub embedding_count: u64,
    pub storage_bytes: u64,
}
```

### C. Add Trait Method

```rust
async fn record_metrics_snapshot(
    &self,
    workspace_id: Uuid,
    trigger_type: MetricsTriggerType,
) -> Result<MetricsSnapshot>;
```

### D. Integration Points

1. After document upload: Record with trigger_type=Event
2. After document deletion: Record with trigger_type=Event
3. Future: Scheduled task records hourly

## Implementation Strategy

1. Add types to multitenancy.rs
2. Export from types/mod.rs
3. Add trait method to workspace_service.rs
4. Implement in workspace_service_impl.rs (PostgreSQL)
5. Add stub in InMemoryWorkspaceService
