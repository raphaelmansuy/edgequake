# OODA-17: Pipeline Robustness Tests

## Commit: e388519f

12 tests covering pipeline-related endpoints.

| Test | Endpoint | What it verifies |
|------|----------|-----------------|
| test_health_check_structure | /health | Status, version, components fields |
| test_health_shows_provider | /health | Mock provider name |
| test_pipeline_status | /api/v1/pipeline/status | Pipeline state object |
| test_queue_metrics | /api/v1/pipeline/queue-metrics | Queue data |
| test_cost_estimation | /api/v1/pipeline/costs/estimate | Cost calculation |
| test_model_pricing | /api/v1/pipeline/costs/pricing | Pricing data |
| test_cost_summary | /api/v1/costs/summary | Workspace cost summary |
| test_cost_history | /api/v1/costs/history | Historical cost data |
| test_budget_status | /api/v1/costs/budget | Budget tracking |
| test_provider_status | /api/v1/settings/provider/status | Provider config |
| test_document_status_after_upload | Upload + detail | Status transitions |
| test_list_shows_upload_status | Upload x3 + list | Document count + status_counts |

All pass in 0.02s.
