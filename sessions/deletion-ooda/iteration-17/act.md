# OODA-17 Act: Historical Metrics Schema

## Implementation

### Created Migration File
Location: `edgequake/migrations/016_workspace_metrics_history.sql`

### Table Schema

```sql
CREATE TABLE workspace_metrics_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id TEXT NOT NULL,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    trigger_type TEXT NOT NULL DEFAULT 'event',
    document_count BIGINT NOT NULL DEFAULT 0,
    chunk_count BIGINT NOT NULL DEFAULT 0,
    entity_count BIGINT NOT NULL DEFAULT 0,
    relationship_count BIGINT NOT NULL DEFAULT 0,
    embedding_count BIGINT NOT NULL DEFAULT 0,
    storage_bytes BIGINT NOT NULL DEFAULT 0,
    CONSTRAINT fk_metrics_workspace 
        FOREIGN KEY (workspace_id) 
        REFERENCES workspaces(id) 
        ON DELETE CASCADE
);
```

### Indexes Created

| Index | Purpose |
|-------|---------|
| `idx_metrics_workspace_time` | Time-series queries (workspace + time range) |
| `idx_metrics_recorded_at` | Retention policy cleanup |
| `idx_metrics_trigger_type` | Filter by sample type |

### Design Decisions

1. **trigger_type column**: Distinguishes event-driven (`'event'`) from scheduled (`'scheduled'`) samples
2. **CASCADE delete**: Automatically cleans up history when workspace is deleted
3. **Separate table**: Time-series data separate from real-time stats for query optimization

## Test Results

```
test result: ok. 25 passed; 0 failed; 0 ignored
```

All existing tests pass (migration doesn't affect memory-based tests).

## Next Steps (OODA-18+)

1. Add `record_metrics_snapshot()` function to WorkspaceService
2. Integrate with document add/delete handlers
3. Add background hourly snapshot task
4. Add API endpoint for historical query
5. Add WebUI dashboard component
