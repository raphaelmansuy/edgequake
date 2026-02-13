# Observation - Iteration 28

## Mission Re-read
Re-read mission file. Focus: T8 "Documentation is complete and accurate" — OpenAPI completeness.

## Files Examined
- `openapi.rs` (372 lines) — All lineage endpoints registered in paths, but DTO schemas missing from `components(schemas())`

## Current State
- Lineage endpoints all have `#[utoipa::path]` annotations
- 15 lineage DTO types missing from OpenAPI schema registry
- `ExportParams` missing `ToSchema` derive
