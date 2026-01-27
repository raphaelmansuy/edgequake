# OODA-20 Observe: Record Metrics Snapshot Function

## Mission Context

From specs/033-study-delete-document/003-study-document.md:

> "Ensure metric likes number of Entities, Relationships, Embeddings per document"

OODA-17 created the schema. Now we need the function to record snapshots.

## Current State

### Migration 016 Schema (OODA-17)

```sql
CREATE TABLE workspace_metrics_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id TEXT NOT NULL,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    trigger_type TEXT NOT NULL DEFAULT 'event',
    document_count, chunk_count, entity_count, relationship_count,
    embedding_count, storage_bytes BIGINT
);
```

### Trigger Types

- `event`: Recorded on document add/delete
- `scheduled`: Recorded by background hourly task
- `manual`: Recorded on admin request

## Function Requirements

### 1. `record_metrics_snapshot(workspace_id, trigger_type)`

- Collect current stats using `get_workspace_stats()`
- Insert into workspace_metrics_history
- Return the snapshot ID

### 2. Integration Points

- After document upload completes
- After document deletion completes
- On scheduled cron (hourly)

## Files to Modify

1. `edgequake/crates/edgequake-core/src/workspace_service.rs` - Trait
2. `edgequake/crates/edgequake-core/src/workspace_service_impl.rs` - PostgreSQL impl
3. `edgequake/crates/edgequake-api/src/handlers/documents.rs` - Integration
