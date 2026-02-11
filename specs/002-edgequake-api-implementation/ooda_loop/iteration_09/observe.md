# IMPL-09 Observe — Costs/Lineage/Chunks/Provenance Type Accuracy

## Observations

### SDK State Before This Iteration

- 243 unit tests, 62 E2E tests passing (IMPL-08)
- `costs.ts` types had completely wrong shapes (SDK: `{total_cost, currency, period, breakdown}`, Rust: `{total_input_tokens, total_output_tokens, total_cost_usd, formatted_cost, operations}`)
- Lineage/chunk/provenance types were in `health.ts` with oversimplified shapes
- SDK missing: `WorkspaceCostSummaryResponse`, `OperationBreakdown`, `ModelPricingResponse`, `EstimateCostRequest/Response`, all lineage rich types

### Rust API Analysis

- **costs_types.rs** (369 lines): Rich cost tracking with per-operation breakdowns, workspace summaries, budget info with `is_over_budget`, model pricing, cost estimation
- **lineage_types.rs** (528 lines): Entity lineage with description versions, document graph lineage with extraction stats, chunk details with char_range/entities/relationships/extraction_metadata, entity provenance with sources/related_entities

### Type Mismatches Found

1. `CostSummary` → completely different fields
2. `CostHistory` → wrong data point shape
3. `BudgetStatus` → wrong field names, missing `is_over_budget`
4. SDK missing: pricing, estimation, workspace cost endpoints
5. `EntityLineage` → missing `entity_type`, `source_count`, `description_versions`, `line_ranges`
6. `DocumentLineage` → missing `chunk_count`, `extraction_stats`, `is_shared`
7. `ChunkDetail` → missing `index`, `char_range`, `relationships`, `extraction_metadata`
8. `EntityProvenance` → missing `entity_type`, `sources`, `total_extraction_count`, `related_entities`
