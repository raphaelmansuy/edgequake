# OODA-40 Act: Tenant + MetricsTriggerType tests

## Changes
- `tenant.rs`: +15 tests — TenantPlan defaults/limits/Display/FromStr, Tenant::new defaults, builder chain (with_plan/description/llm_config/embedding_config/vision_config)
- `metrics.rs`: +6 tests — MetricsTriggerType as_str/Display/parse roundtrip/case-insensitive/unknown, MetricsSnapshot construction

## Test count: 1485 → 1505 (+20)
