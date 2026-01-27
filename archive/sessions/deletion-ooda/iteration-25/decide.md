# OODA-25 Decide: Create Metrics Documentation

## Action: Create metrics-infrastructure.md

### Content Plan

1. **Overview**
   - Purpose of metrics system
   - Benefits for debugging and monitoring

2. **Architecture**
   - Database schema (workspace_metrics_history table)
   - Rust types (MetricsTriggerType, MetricsSnapshot)
   - Storage layer integration

3. **API Reference**
   - Endpoint: GET /api/v1/workspaces/{id}/metrics-history
   - Query parameters
   - Response schema
   - Example request/response

4. **Automatic Recording**
   - Document upload triggers Event snapshot
   - Document deletion triggers Event snapshot
   - Best-effort pattern (won't fail main operation)

5. **Use Cases**
   - Track entity/relationship/embedding growth
   - Debug deletion issues
   - Monitor workspace health

6. **Future Roadmap**
   - Scheduled snapshots (hourly background task)
   - Manual trigger endpoint
   - Alerting on thresholds

## File: specs/033-study-delete-document/docs/metrics-infrastructure.md
