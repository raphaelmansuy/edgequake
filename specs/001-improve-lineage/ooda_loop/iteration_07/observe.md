# Observation - Iteration 07

## Mission Re-read
Re-read `specs/001-improve-lineage.md` (683 lines) — mission deliverable #3: Optimized API Endpoints.

## Files Examined

- `edgequake/crates/edgequake-api/src/handlers/lineage.rs` (704 lines) — Existing endpoints: `get_chunk_detail`, `get_entity_provenance`, `get_entity_lineage`, `get_document_lineage`
- `edgequake/crates/edgequake-api/src/handlers/lineage_types.rs` (535 lines) — `ChunkDetailResponse` struct lacked `start_line`, `end_line`
- `edgequake/crates/edgequake-api/src/routes.rs` (467→475 lines) — Route registration for all API endpoints
- `edgequake/crates/edgequake-api/src/openapi.rs` (368 lines) — OpenAPI/utoipa path registration

## Current Gaps

1. `ChunkDetailResponse` missing `start_line` and `end_line` fields despite OODA-05 storing them in KV
2. `get_chunk_detail` reads `chunk_index` from KV but OODA-05 stores it as `index`
3. No endpoint for complete lineage tree (`GET /documents/:id/lineage`) — required by F5
4. No endpoint for single-call metadata (`GET /documents/:id/metadata`) — required by F5
5. Both missing from OpenAPI spec — required by T8

## Tests Run

- `cargo test --workspace --lib` → 1698 passed, 0 clippy warnings
- Known flaky: `lmstudio::tests::test_from_env_custom` (passes in isolation)
