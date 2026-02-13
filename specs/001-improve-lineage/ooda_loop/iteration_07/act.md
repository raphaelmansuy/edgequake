# Implementation - Iteration 07

## Changes Made

### 1. ChunkDetailResponse enhanced (lineage_types.rs)
- **Lines**: ~140-142
- **Change**: Added `start_line: Option<usize>` and `end_line: Option<usize>` with `skip_serializing_if = "Option::is_none"`
- **WHY**: OODA-05 stores position data in KV but the API response dropped it at the boundary

### 2. get_chunk_detail fixed (lineage.rs)
- **Lines**: ~260-280
- **Change**: Reads `index` (fallback `chunk_index`) and reads `start_line`/`end_line` from KV data
- **WHY**: Field name mismatch caused chunk index to always be 0 in API responses

### 3. get_document_full_lineage endpoint (lineage.rs)
- **Lines**: ~546-576
- **Change**: New endpoint `GET /api/v1/documents/{document_id}/lineage` — reads `{doc_id}-lineage` KV key (from OODA-06) + metadata, returns combined response
- **WHY**: Satisfies F5 (single API call for complete lineage tree) and T2 (no N+1 queries)

### 4. get_document_metadata endpoint (lineage.rs)
- **Lines**: ~594-612
- **Change**: New endpoint `GET /api/v1/documents/{document_id}/metadata` — reads `{doc_id}-metadata` KV key
- **WHY**: Satisfies F1 (all metadata retrievable) with single O(1) KV lookup

### 5. Route registration (routes.rs)
- **Lines**: ~265-273
- **Change**: Added both routes before catch-all `{document_id}` route
- **WHY**: Axum requires specific routes before parameterized catch-all

### 6. OpenAPI registration (openapi.rs)
- **Lines**: ~112-113
- **Change**: Added `get_document_full_lineage` and `get_document_metadata` to utoipa paths
- **WHY**: Required for auto-generated API docs

### 7. Test fix (lineage_types.rs)
- **Lines**: ~402-414
- **Change**: Added `start_line: Some(1)`, `end_line: Some(5)` to test struct construction
- **WHY**: New required fields in ChunkDetailResponse

## Tests Run

- `cargo test --workspace --lib` → 1698 passed, 0 failed
- `cargo clippy -p edgequake-api` → 0 warnings

## Success Criteria Addressed

- **F5**: Single API call retrieves complete document lineage tree ✅
- **F1**: All document metadata retrievable via API ✅
- **T2**: No N+1 queries — both endpoints do single KV lookups ✅
- **T8**: Documentation via utoipa OpenAPI annotations ✅
- **Q5**: API follows REST best practices (`/documents/:id/lineage`) ✅
