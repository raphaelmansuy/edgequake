# OODA-26 Observe: Scheduled Metrics Background Task

## Gap Identified

The metrics infrastructure supports three trigger types:
- **Event**: ✅ Implemented (upload/delete triggers)
- **Scheduled**: ❌ Not implemented (hourly background task)
- **Manual**: ❌ Not implemented (user-triggered endpoint)

## Current State

- workspace_metrics_history table ready
- record_metrics_snapshot() function working
- Event triggers integrated in handlers

## Need

A background task that periodically (hourly) records metrics snapshots
for all active workspaces. This enables:

1. Trend analysis even without document changes
2. Baseline metrics for comparison
3. Detection of external data changes

## Design Considerations

1. Must not block API startup
2. Should be configurable (interval, enabled/disabled)
3. Must handle large numbers of workspaces efficiently
4. Needs graceful shutdown
