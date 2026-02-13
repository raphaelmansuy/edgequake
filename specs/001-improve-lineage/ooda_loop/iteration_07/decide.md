# Decision - Iteration 07

## Changes to Make

1. **Add `start_line`, `end_line` to `ChunkDetailResponse`** (lineage_types.rs:~140)
   - Optional fields with `skip_serializing_if = "Option::is_none"`
   - Backward compatible

2. **Fix `get_chunk_detail` field reading** (lineage.rs:~260)
   - Read `index` with fallback to `chunk_index`
   - Read `start_line`, `end_line` from KV data

3. **Create `get_document_full_lineage` endpoint** (lineage.rs:~546)
   - `GET /api/v1/documents/{document_id}/lineage`
   - Reads `{doc_id}-lineage` KV key (from OODA-06)
   - Combines lineage + metadata in single response

4. **Create `get_document_metadata` endpoint** (lineage.rs:~594)
   - `GET /api/v1/documents/{document_id}/metadata`
   - Reads `{doc_id}-metadata` KV key
   - Returns raw JSON metadata blob

5. **Register routes** (routes.rs:~265)
   - Before catch-all `{document_id}` route

6. **Register in OpenAPI** (openapi.rs:~112)
   - Add both endpoints to utoipa paths

## Priority

1. Fix ChunkDetailResponse (high impact, low effort) ← fixes data loss at API boundary
2. Create new endpoints (high impact, medium effort) ← satisfies F5
3. Route + OpenAPI registration (required for endpoints to work)
4. Fix test (required for CI)

## Expected Outcome

- `ChunkDetailResponse` includes `start_line`, `end_line` from OODA-05 data
- Single API call at `/documents/:id/lineage` returns complete lineage tree
- Single API call at `/documents/:id/metadata` returns all metadata
- Both endpoints visible in OpenAPI/Swagger docs
