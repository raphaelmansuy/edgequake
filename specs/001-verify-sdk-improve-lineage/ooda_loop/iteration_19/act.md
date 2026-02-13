# OODA-19: Act — Java SDK LineageService

## Execution
- Created `sdks/java/src/main/java/io/edgequake/sdk/resources/LineageService.java` (7 methods, 103 lines)
- Updated `sdks/java/src/main/java/io/edgequake/sdk/EdgeQuakeClient.java` (added lineageService)
- Tests: 123 pass, 0 fail
- Commit: see below

## Methods Added
1. `entityLineage(name)` → GET /api/v1/lineage/entities/{name}
2. `documentLineage(id)` → GET /api/v1/lineage/documents/{id}
3. `documentFullLineage(id)` → GET /api/v1/documents/{id}/lineage
4. `exportLineage(id, format)` → GET /api/v1/documents/{id}/lineage/export
5. `chunkDetail(id)` → GET /api/v1/chunks/{id}
6. `chunkLineage(id)` → GET /api/v1/chunks/{id}/lineage
7. `entityProvenance(id)` → GET /api/v1/entities/{id}/provenance
