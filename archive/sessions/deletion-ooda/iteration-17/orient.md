# OODA-17 Orient: Historical Metrics Analysis

## Architecture Decision

### Chosen Approach: Hybrid Sampling

```
┌─────────────────────────────────────────────────────────────┐
│                    Metrics Recording Flow                     │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  Document Add/Delete Event                                   │
│          │                                                   │
│          ▼                                                   │
│  ┌─────────────────┐                                        │
│  │ Rate Limiter    │  Max 1 sample per workspace/minute     │
│  │ (in-memory)     │                                        │
│  └────────┬────────┘                                        │
│           │ Allow?                                          │
│           ▼                                                 │
│  ┌─────────────────┐                                        │
│  │ Metrics Query   │  SELECT counts from tables             │
│  └────────┬────────┘                                        │
│           │                                                 │
│           ▼                                                 │
│  ┌─────────────────┐                                        │
│  │ Insert History  │  INSERT INTO workspace_metrics_history │
│  └─────────────────┘                                        │
│                                                               │
└─────────────────────────────────────────────────────────────┘
```

## First Principles Analysis

### Why Historical Metrics?

1. **Trend Analysis**: Identify growth patterns, anomalies
2. **Capacity Planning**: Predict storage needs
3. **Debugging**: Correlate issues with changes over time
4. **Billing**: Usage-based pricing potential

### Why Rate-Limit?

- Without: 100 document uploads = 100 metric records
- With: 100 document uploads = ~1-5 metric records
- Reduces storage overhead by 95%+

### Why Hourly Snapshots?

- Ensures at least 24 data points per day
- Even if no events, trends are captured
- Background task, no user-facing latency

## Database Schema Design

### Table: workspace_metrics_history

| Column             | Type        | Purpose                |
| ------------------ | ----------- | ---------------------- |
| id                 | UUID        | Primary key            |
| workspace_id       | TEXT        | FK to workspaces       |
| recorded_at        | TIMESTAMPTZ | Sample timestamp       |
| document_count     | BIGINT      | Documents at time      |
| chunk_count        | BIGINT      | Chunks at time         |
| entity_count       | BIGINT      | KG nodes at time       |
| relationship_count | BIGINT      | KG edges at time       |
| embedding_count    | BIGINT      | Embeddings at time     |
| storage_bytes      | BIGINT      | Total storage          |
| trigger_type       | TEXT        | 'event' or 'scheduled' |

### Indexes

- `(workspace_id, recorded_at DESC)` - Time-series queries
- `(recorded_at)` - Cleanup of old records

### Retention Policy

- Keep hourly data for 7 days
- Keep daily aggregates for 90 days
- Keep monthly aggregates forever

## Implementation Plan

### Phase 1: Foundation (This iteration)

1. Create migration 016_workspace_metrics_history.sql
2. Add insert function for metrics recording
3. Hook into document add/delete handlers

### Phase 2: Background Task (Next iteration)

1. Hourly snapshot task
2. Aggregation/cleanup task

### Phase 3: API & UI (Future)

1. History query endpoint
2. WebUI dashboard

## Risk Assessment

| Risk              | Impact            | Mitigation       |
| ----------------- | ----------------- | ---------------- |
| High write volume | DB overload       | Rate limiting    |
| Storage growth    | Disk full         | Retention policy |
| Query latency     | Slow UI           | Async recording  |
| Missing data      | Incomplete trends | Hourly snapshots |

## Memory Provider Consideration

Memory provider: Skip historical recording (no persistence anyway).
PostgreSQL provider: Full metrics recording.
