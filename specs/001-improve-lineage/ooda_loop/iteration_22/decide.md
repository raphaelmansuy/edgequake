# Decision - Iteration 22

## Changes to Make
1. `edgequake-api/src/handlers/lineage.rs` — Add `ExportParams` struct and `export_document_lineage` handler with JSON/CSV support
2. `edgequake-api/src/handlers/lineage.rs` — Add `lineage_to_csv()` helper that flattens chunks to tabular rows
3. `edgequake-api/src/routes.rs` — Register `/documents/{document_id}/lineage/export` route
4. `edgequake-api/src/openapi.rs` — Add `export_document_lineage` to utoipa paths
5. `edgequake-api/src/handlers/lineage.rs` — Add 6 unit tests for CSV generation and export params

## Priority
1. Export handler + CSV logic (high impact, low effort)
2. Route + OpenAPI registration (required, trivial)
3. Tests (required for validation)

## Expected Outcome
- `GET /api/v1/documents/{id}/lineage/export` returns downloadable JSON file
- `GET /api/v1/documents/{id}/lineage/export?format=csv` returns downloadable CSV file
- Content-Disposition attachment headers trigger browser download
- 6 new unit tests pass
- All 450+ API tests continue to pass
