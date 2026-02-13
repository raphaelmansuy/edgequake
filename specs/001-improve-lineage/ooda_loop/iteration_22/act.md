# Implementation - Iteration 22

## Changes Made

1. **File**: `edgequake-api/src/handlers/lineage.rs`
   - Lines: 27-32 — Updated imports: added `Query`, `header`, `StatusCode`, `IntoResponse` from axum
   - Lines: ~860-990 — Added `ExportParams` struct, `export_document_lineage` handler, `lineage_to_csv()` helper
   - Lines: ~1040-1100 — Added 6 unit tests for CSV export logic

2. **File**: `edgequake-api/src/routes.rs`
   - Lines: ~275-279 — Added route `/documents/{document_id}/lineage/export` → `export_document_lineage`

3. **File**: `edgequake-api/src/openapi.rs`
   - Line: ~114 — Added `handlers::export_document_lineage` to utoipa paths

## Tests Added
- `test_lineage_to_csv_basic` — Verifies 2-chunk CSV generation with header
- `test_lineage_to_csv_empty_chunks` — Verifies header-only output for empty chunks
- `test_lineage_to_csv_no_chunks_key` — Verifies graceful handling of missing chunks key
- `test_lineage_to_csv_escapes_quotes` — Verifies CSV quote escaping (double-quote)
- `test_export_params_default_format` — Verifies default format is "json"
- `test_export_params_csv_format` — Verifies CSV format parsing

## Verification
- `cargo build -p edgequake-api`: ✅ Compiles cleanly
- `cargo test -p edgequake-api --lib`: ✅ 450 passed, 0 failed
- `cargo test -p edgequake-api --lib lineage`: ✅ 32 passed (was 26, +6 new)
