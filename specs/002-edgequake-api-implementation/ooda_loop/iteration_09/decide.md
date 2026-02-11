# IMPL-09 Decide — Implementation Plan

## Changes

1. Rewrite `src/types/costs.ts` matching Rust `costs_types.rs` (14 interfaces)
2. Create `src/types/lineage.ts` matching Rust `lineage_types.rs` (20 interfaces)
3. Update `src/resources/costs.ts` — add `pricing()`, `estimate()`, `workspaceSummary()`; accept `CostHistoryQuery`
4. Update `src/resources/lineage.ts` — import from `lineage.ts` instead of `health.ts`
5. Update `src/resources/chunks.ts` — import from `lineage.ts`
6. Update `src/resources/provenance.ts` — import from `lineage.ts`
7. Clean `src/types/health.ts` — remove lineage/chunk/provenance types, add re-exports
8. Add `lineage.ts` to barrel exports in `src/types/index.ts`
9. Update unit test mocks for all changed resources
10. Legacy aliases for backward compatibility
