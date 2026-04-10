# OODA-41 Act: Cost tracking + ingestion types tests

## Changes
- `progress/cost.rs`: +10 tests — ModelPricing zero/input-only/output-only, OperationCost new/accumulate, CostBreakdown new/add_operation/formatted_cost/zero
- `ingestion_types.rs`: +14 tests — SourceType Display, UnifiedStage default/is_terminal/is_active/index/display_name/roundtrip/to_pipeline_none/text_eq_markdown, StageStatus default, StageProgress new/skipped, IngestionError not_recoverable, IngestionProgress pdf_stages/fail

## Test count: 1505 → 1529 (+24)
