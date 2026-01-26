# Metrics Infrastructure

> **ITERATION**: 27  
> **STATUS**: Manual Trigger Complete  
> **DATE**: 2025-01-27

## Overview

EdgeQuake's metrics infrastructure provides time-series tracking of workspace
health through automated snapshots. Every document upload and deletion records
a metrics snapshot, enabling trend analysis and debugging.

### Key Benefits

- **Trend Analysis**: Track entity/relationship/embedding growth over time
- **Debugging**: Identify deletion issues by comparing before/after snapshots
- **Monitoring**: Monitor workspace health and detect anomalies
- **Audit Trail**: Historical record of data changes

## Architecture

### Database Schema

Migration 016 creates the `workspace_metrics_history` table:

```sql
CREATE TABLE workspace_metrics_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    trigger_type TEXT NOT NULL CHECK (trigger_type IN ('event', 'scheduled', 'manual')),
    entity_count BIGINT NOT NULL DEFAULT 0,
    relationship_count BIGINT NOT NULL DEFAULT 0,
    embedding_count BIGINT NOT NULL DEFAULT 0,
    document_count BIGINT NOT NULL DEFAULT 0
);

CREATE INDEX idx_workspace_metrics_history_workspace 
    ON workspace_metrics_history(workspace_id);
CREATE INDEX idx_workspace_metrics_history_recorded 
    ON workspace_metrics_history(workspace_id, recorded_at DESC);
```

### Rust Types

#### MetricsTriggerType

```rust
pub enum MetricsTriggerType {
    Event,     // Triggered by upload/delete operations
    Scheduled, // Background task (future)
    Manual,    // User-initiated via POST endpoint
}
```

#### MetricsSnapshot

```rust
pub struct MetricsSnapshot {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub recorded_at: DateTime<Utc>,
    pub trigger_type: MetricsTriggerType,
    pub entity_count: i64,
    pub relationship_count: i64,
    pub embedding_count: i64,
    pub document_count: i64,
}
```

### Storage Layer Integration

The `WorkspaceService` trait provides:

```rust
async fn record_metrics_snapshot(
    &self,
    workspace_id: Uuid,
    trigger_type: MetricsTriggerType,
) -> Result<MetricsSnapshot>;

async fn get_metrics_history(
    &self,
    workspace_id: Uuid,
    limit: i32,
    offset: i32,
) -> Result<Vec<MetricsSnapshot>>;
```

## API Reference

### Get Metrics History

**Endpoint**: `GET /api/v1/workspaces/{workspace_id}/metrics-history`

**Authentication**: Required (API key or session)

**Query Parameters**:

| Parameter | Type | Default | Max | Description |
|-----------|------|---------|-----|-------------|
| `limit` | i32 | 100 | 1000 | Number of snapshots to return |
| `offset` | i32 | 0 | - | Offset for pagination |

**Response**: `200 OK`

```json
{
  "snapshots": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "workspace_id": "550e8400-e29b-41d4-a716-446655440001",
      "recorded_at": "2025-01-27T10:30:00Z",
      "trigger_type": "event",
      "entity_count": 150,
      "relationship_count": 75,
      "embedding_count": 300,
      "document_count": 10
    }
  ],
  "total": 25,
  "limit": 100,
  "offset": 0
}
```

### Example Request

```bash
curl -X GET \
  "http://localhost:3000/api/v1/workspaces/550e8400-e29b-41d4-a716-446655440001/metrics-history?limit=10" \
  -H "Authorization: Bearer YOUR_API_KEY"
```

### Trigger Metrics Snapshot (Manual)

**Endpoint**: `POST /api/v1/workspaces/{workspace_id}/metrics-snapshot`

**Authentication**: Required (API key or session)

**Request Body**: None required

**Response**: `201 Created`

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "recorded_at": "2025-01-27T10:30:00Z",
  "trigger_type": "manual",
  "document_count": 10,
  "chunk_count": 50,
  "entity_count": 150,
  "relationship_count": 75,
  "embedding_count": 300,
  "storage_bytes": 1048576
}
```

### Example Request (Manual Trigger)

```bash
curl -X POST \
  "http://localhost:3000/api/v1/workspaces/550e8400-e29b-41d4-a716-446655440001/metrics-snapshot" \
  -H "Authorization: Bearer YOUR_API_KEY"
```

## Automatic Recording

Metrics snapshots are automatically recorded in two scenarios:

### 1. Document Upload (POST /api/v1/documents)

After a document is successfully uploaded and processed, an `Event` snapshot
is recorded. This captures the state after new entities and relationships
are extracted.

### 2. Document Deletion (DELETE /api/v1/documents/{id})

After a document is successfully deleted (with cascading removal of entities,
relationships, and embeddings), an `Event` snapshot is recorded. This captures
the reduced state.

### Best-Effort Pattern

Metrics recording uses a best-effort pattern:

```rust
if let Err(e) = workspace_service
    .record_metrics_snapshot(workspace_id, MetricsTriggerType::Event)
    .await
{
    tracing::warn!("Failed to record metrics snapshot: {e}");
    // Continue - don't fail the main operation
}
```

This ensures that:
- Metrics failures never fail document operations
- Issues are logged for debugging
- The system remains resilient

## Use Cases

### 1. Track Growth Over Time

Query snapshots to see how a workspace grows:

```sql
SELECT recorded_at, entity_count, relationship_count
FROM workspace_metrics_history
WHERE workspace_id = $1
ORDER BY recorded_at ASC;
```

### 2. Debug Deletion Issues

Compare entity counts before and after deletion:

```sql
SELECT trigger_type, entity_count, relationship_count
FROM workspace_metrics_history
WHERE workspace_id = $1
ORDER BY recorded_at DESC
LIMIT 10;
```

If deletion doesn't reduce counts as expected, there may be orphaned data.

### 3. Monitor Workspace Health

Set up alerts when counts unexpectedly drop or spike:

```sql
-- Alert if entity count drops by more than 50%
WITH recent AS (
    SELECT entity_count, LAG(entity_count) OVER (ORDER BY recorded_at) as prev_count
    FROM workspace_metrics_history
    WHERE workspace_id = $1
    ORDER BY recorded_at DESC
    LIMIT 2
)
SELECT * FROM recent
WHERE entity_count < prev_count * 0.5;
```

## Test Coverage

### E2E Tests (e2e_metrics_history.rs)

| Test | Description |
|------|-------------|
| `test_metrics_history_empty_for_new_workspace` | Returns empty array for new workspace |
| `test_metrics_history_limit_parameter` | Limit parameter works correctly |
| `test_metrics_history_offset_parameter` | Offset parameter works correctly |
| `test_metrics_history_max_limit_enforced` | Limits capped at 1000 |
| `test_metrics_history_pagination_combined` | Limit + offset work together |
| `test_trigger_metrics_snapshot_creates_snapshot` | Manual trigger endpoint works |
| `test_trigger_metrics_snapshot_response_structure` | Response format validation |
| `test_trigger_metrics_snapshot_method_not_allowed` | Only POST allowed |

**Total: 8 tests**

## Future Roadmap

### Scheduled Snapshots (Planned)

Background task to record hourly snapshots:

```rust
// Future implementation
#[async_trait]
pub trait MetricsScheduler {
    async fn record_all_workspaces(&self) -> Result<()>;
}
```

### ~~Manual Trigger Endpoint~~ ✅ IMPLEMENTED (OODA-26)

```
POST /api/v1/workspaces/{id}/metrics-snapshot
```

Allows users to manually trigger a snapshot for debugging or
external scheduler integration.

### Alerting Integration (Planned)

Webhook notifications when metrics exceed thresholds:

```json
{
  "workspace_id": "...",
  "alert_type": "entity_count_spike",
  "current_value": 10000,
  "threshold": 5000
}
```

## Related Documentation

- [Study Summary](./summary.md) - Overall deletion study
- [Migration 016](../../../../edgequake/crates/edgequake-core/migrations/016_workspace_metrics_history.sql) - Schema
- [E2E Tests](../../../../edgequake/crates/edgequake-api/tests/e2e_metrics_history.rs) - Test cases
