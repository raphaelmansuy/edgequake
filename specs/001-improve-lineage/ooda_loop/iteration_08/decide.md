# Decision - Iteration 08

## Changes to Make

1. **Add `ChunkLineageResponse` DTO** (lineage_types.rs) — lightweight chunk lineage type
2. **Add `get_chunk_lineage` endpoint** (lineage.rs) — `GET /chunks/:id/lineage`
3. **Register route** (routes.rs) — after `/chunks/{chunk_id}` detail route
4. **Register in OpenAPI** (openapi.rs) — for API docs

## Expected Outcome

All 4 endpoints from mission deliverable #3 are now implemented:
- ✅ `GET /api/v1/documents/:id/lineage` (OODA-07)
- ✅ `GET /api/v1/documents/:id/metadata` (OODA-07)
- ✅ `GET /api/v1/chunks/:id/lineage` (OODA-08)
- ✅ `GET /api/v1/entities/:id/provenance` (pre-existing)
