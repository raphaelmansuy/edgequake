# OODA-40 Observe

## Target files
- `types/multitenancy/tenant.rs` (272 lines, 0 tests) — Tenant struct with builder, TenantPlan enum with limits/Display/FromStr
- `types/multitenancy/metrics.rs` (75 lines, 0 tests) — MetricsTriggerType enum, MetricsSnapshot struct

## Findings
- TenantPlan has 4 variants with 3 limit methods each (12 combinations)
- Tenant::new reads env vars via Workspace::default_llm_config() — tests verify structure, not env
- MetricsTriggerType has parse/as_str/Display — all pure, all testable
