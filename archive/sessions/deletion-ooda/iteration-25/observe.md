# OODA-25 Observe: Metrics Infrastructure Documentation

## Documentation Gap

We've built a comprehensive metrics infrastructure (OODA 17-23) but haven't
documented it for users and developers.

## What Needs Documentation

### 1. Architecture Overview

- workspace_metrics_history table (migration 016)
- MetricsTriggerType enum (Event, Scheduled, Manual)
- MetricsSnapshot struct

### 2. API Reference

- GET /api/v1/workspaces/{id}/metrics-history
- Query parameters (limit, offset)
- Response format

### 3. Integration Points

- Automatic recording after document upload
- Automatic recording after document deletion
- Future: Scheduled recording (hourly)

### 4. Usage Examples

- Trend analysis queries
- Debugging scenarios

## Target: Create docs/metrics-infrastructure.md
