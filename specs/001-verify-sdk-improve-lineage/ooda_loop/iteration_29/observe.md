# OODA-29 Observe: Go SDK Lineage Enhancement

## Current State
- Go SDK: 22 services (including Chunks, Provenance, Lineage already wired)
- LineageService: Only has `ForEntity(ctx, entityName, depth)` → returns `*LineageGraph`
- ChunkService: Only has `Get(ctx, id)` → returns `*ChunkDetail`
- ProvenanceService: Has `ForEntity(ctx, entityID)` → returns `[]ProvenanceRecord`
- Existing types: `LineageGraph`, `LineageNode`, `LineageEdge`, `ChunkDetail`, `ProvenanceRecord`
- Tests: 209 existing tests in edgequake_test.go + edgequake_coverage_test.go
- Pattern: httptest.NewServer mock, typed responses, `*Client.get/post/delNoContent`
- No `getRaw` client method for raw byte downloads

## Lineage Gap (4 of 7 endpoints missing)
| Endpoint | Status |
|----------|--------|
| `GET /api/v1/lineage/entities/{name}` | ✅ LineageService.ForEntity |
| `GET /api/v1/lineage/documents/{id}` | ❌ Missing |
| `GET /api/v1/documents/{id}/lineage` | ❌ Missing |
| `GET /api/v1/documents/{id}/lineage/export` | ❌ Missing |
| `GET /api/v1/chunks/{id}` | ✅ ChunkService.Get |
| `GET /api/v1/chunks/{id}/lineage` | ❌ Missing |
| `GET /api/v1/entities/{id}/provenance` | ✅ ProvenanceService.ForEntity |
