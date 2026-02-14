# OODA-18: Observe — Java SDK Lineage Models Gap

## Current State

- Java SDK has 123 unit tests, all passing
- pom.xml targets Java 21 (LTS)
- Existing lineage models in OperationModels.java: ChunkDetail, ProvenanceRecord, LineageNode, LineageEdge, LineageGraph (5 classes)
- **Missing**: EntityLineageResponse, DocumentFullLineageResponse, ChunkLineageResponse, EntityProvenanceResponse, and all supporting sub-types
- No LineageService class exists — no service to call `/api/v1/lineage/*` endpoints
- TypeScript SDK has full lineage types (323 lines in types/lineage.ts) as reference

## Backend Endpoints to Cover

```
GET /api/v1/lineage/entities/{name}     → EntityLineageResponse
GET /api/v1/lineage/documents/{id}      → DocumentGraphLineageResponse
GET /api/v1/documents/{id}/lineage      → DocumentFullLineageResponse
GET /api/v1/documents/{id}/lineage/export → binary (JSON/CSV)
GET /api/v1/chunks/{id}                 → ChunkDetailResponse
GET /api/v1/chunks/{id}/lineage         → ChunkLineageResponse
GET /api/v1/entities/{id}/provenance    → EntityProvenanceResponse
```

## Gap Analysis

- 7 lineage endpoints → 0 covered in Java SDK
- ~15 model classes needed (matching TypeScript SDK types/lineage.ts)
- 1 service class needed (LineageService.java)
