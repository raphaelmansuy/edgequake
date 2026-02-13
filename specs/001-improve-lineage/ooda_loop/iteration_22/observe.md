# Observation - Iteration 22

## Mission Re-read
Re-read complete mission file (687 lines). Focus: Deliverable #4 "Export Capability: Download complete lineage as JSON/CSV".

## Files Examined
- `edgequake/crates/edgequake-api/src/handlers/lineage.rs` (lines 25-855) — existing lineage handlers, imports, KV lookup pattern
- `edgequake/crates/edgequake-api/src/routes.rs` (lines 260-290) — route registration pattern, `{document_id}` catch-all ordering
- `edgequake/crates/edgequake-api/src/openapi.rs` (lines 108-120) — utoipa path registration

## Current State
- Lineage data is persisted in KV storage under `{document_id}-lineage` key
- `get_document_full_lineage` returns JSON response but as API response (not downloadable file)
- No export endpoint exists for downloading lineage as a file
- No CSV format support for lineage data
- All existing lineage endpoints return `Json<T>` with standard content-type

## Tests Run
- `cargo test -p edgequake-api --lib` → 450 passed, 0 failed
- `cargo test -p edgequake-api --lib lineage` → 26 tests passed (pre-change baseline)
