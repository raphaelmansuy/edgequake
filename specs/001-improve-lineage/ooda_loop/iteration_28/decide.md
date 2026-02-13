# Decision - Iteration 28

## Changes to Make
1. `openapi.rs` — Add 15 lineage DTO schemas to `components(schemas())`
2. `lineage.rs` — Add `ToSchema` + `Serialize` derives to `ExportParams`

## Expected Outcome
- OpenAPI spec includes complete type definitions for all lineage endpoints
- SDK code generators can produce typed lineage clients
