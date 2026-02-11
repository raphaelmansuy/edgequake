# IMPL-09 Act — Results & Validation

## Changes Implemented

### Type Rewrites

- **`src/types/costs.ts`**: 14 interfaces matching Rust `costs_types.rs` — CostSummaryResponse, OperationCostResponse, WorkspaceCostSummaryResponse, OperationBreakdown, BudgetInfo, CostHistoryPoint, CostHistoryResponse, ModelPricingResponse, AvailablePricingResponse, EstimateCostRequest/Response, CostHistoryQuery, UpdateBudgetRequest. Legacy aliases (CostSummary, CostHistory, BudgetStatus) for backward compat.
- **`src/types/lineage.ts`** (NEW): 20 interfaces matching Rust `lineage_types.rs` — EntityLineageResponse, SourceDocumentInfo, LineRangeInfo, DescriptionVersionResponse, DocumentGraphLineageResponse, EntitySummaryResponse, RelationshipSummaryResponse, ExtractionStatsResponse, ChunkDetailResponse, CharRange, ExtractedEntityInfo, ExtractedRelationshipInfo, ExtractionMetadataInfo, EntityProvenanceResponse, EntitySourceInfo, ChunkSourceInfo, RelatedEntityInfo. Legacy aliases for backward compat.

### Resource Updates

- **`src/resources/costs.ts`**: Added `pricing()`, `estimate()`, `workspaceSummary()`. Updated `history()` to accept `CostHistoryQuery`. Return types now match Rust.
- **`src/resources/lineage.ts`**: Import from `lineage.ts`, return `EntityLineageResponse`/`DocumentGraphLineageResponse`
- **`src/resources/chunks.ts`**: Import from `lineage.ts`, return `ChunkDetailResponse`
- **`src/resources/provenance.ts`**: Import from `lineage.ts`, return `EntityProvenanceResponse`
- **`src/types/health.ts`**: Lineage/chunk/provenance types moved out, re-exported for backward compat
- **`src/types/index.ts`**: Added `lineage.ts` to barrel exports

### Test Updates

- Unit tests: 247 pass (130 resource tests, up from 126 — 4 new cost tests)
- E2E tests: 62 pass (unchanged count, all green)
- Total: 309 tests (247 passed, 62 skipped or 62 E2E passed)

## Validation

| Metric     | Before      | After       |
| ---------- | ----------- | ----------- |
| Unit tests | 243 pass    | 247 pass    |
| E2E tests  | 62 pass     | 62 pass     |
| Type check | Clean       | Clean       |
| Build      | ESM+CJS+DTS | ESM+CJS+DTS |
| DTS size   | 72.71 KB    | 83.97 KB    |

## Commit

```
IMPL-09: Costs/lineage/chunks/provenance type accuracy (34 new interfaces)
```
