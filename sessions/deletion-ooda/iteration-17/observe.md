# OODA-17 Observe: Historical Metrics Tracking

## Mission Requirement

From specs/033-study-delete-document/003-study-document.md:

> "We want to monitor Documents numbers, Entities numbers, Relationships numbers, Embeddings numbers per workspace and per tenant **over time**."

## Current Metrics Architecture

### Real-time Metrics (OODA-12/13)

- `WorkspaceStats` struct provides point-in-time counts
- SQL queries count rows directly from tables
- No historical storage

### What's Missing

1. **Time-series storage** for metrics
2. **Periodic sampling** mechanism
3. **API endpoints** to query historical data
4. **Aggregation** by time window (hour, day, week)

## PostgreSQL Schema Analysis

### Current Tables (migrations 001-015)

- `documents` - Document metadata
- `chunks` - Document chunks
- `entities` - KG nodes
- `relationships` - KG edges
- `embeddings` - Vector embeddings
- `workspaces` - Tenant/workspace config
- `_sqlx_migrations` - Schema version tracking

### Proposed New Table: `workspace_metrics_history`

```sql
CREATE TABLE IF NOT EXISTS workspace_metrics_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id TEXT NOT NULL,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    -- Counts at point in time
    document_count BIGINT NOT NULL DEFAULT 0,
    chunk_count BIGINT NOT NULL DEFAULT 0,
    entity_count BIGINT NOT NULL DEFAULT 0,
    relationship_count BIGINT NOT NULL DEFAULT 0,
    embedding_count BIGINT NOT NULL DEFAULT 0,
    storage_bytes BIGINT NOT NULL DEFAULT 0,

    -- Indexes for efficient time-series queries
    CONSTRAINT fk_workspace FOREIGN KEY (workspace_id)
        REFERENCES workspaces(id) ON DELETE CASCADE
);

CREATE INDEX idx_metrics_workspace_time
    ON workspace_metrics_history(workspace_id, recorded_at DESC);
```

## Sampling Strategy Options

### Option A: Event-Driven Sampling

Record metrics after each document add/delete operation.

- Pro: Accurate
- Con: High write volume

### Option B: Periodic Sampling

Background task records metrics every N minutes.

- Pro: Controlled write volume
- Con: Less accurate for fast-changing workspaces

### Option C: Hybrid (RECOMMENDED)

- Record after significant events (document add/delete)
- Rate-limit to max 1 sample per minute per workspace
- Background task for hourly snapshots

## API Endpoints for Historical Data

```
GET /api/v1/workspaces/{id}/metrics/history
  ?start=2024-01-01T00:00:00Z
  &end=2024-01-31T23:59:59Z
  &interval=hour  # hour, day, week
```

Response:

```json
{
  "workspace_id": "ws-123",
  "start": "2024-01-01T00:00:00Z",
  "end": "2024-01-31T23:59:59Z",
  "interval": "hour",
  "data_points": [
    {
      "timestamp": "2024-01-01T00:00:00Z",
      "document_count": 10,
      "entity_count": 150,
      "relationship_count": 200,
      "embedding_count": 500
    },
    ...
  ]
}
```

## Implementation Priority

1. Create migration 016_workspace_metrics_history.sql
2. Add MetricsRecorder service
3. Integrate with document add/delete handlers
4. Add history API endpoint
5. Add WebUI dashboard component
