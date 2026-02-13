# OODA-16: Observe — TypeScript SDK Lineage Tests

## Baseline
- 247 tests passing in `sdks/typescript/tests/unit/resources.test.ts`
- Basic endpoint mapping for lineage/provenance but no rich field validation
- Types: EntityLineageResponse, DocumentGraphLineageResponse, ChunkDetailResponse, EntityProvenanceResponse, DocumentFullLineageResponse, ChunkLineageResponse

## Gap Analysis
- No tests validating source_documents or description_versions in EntityLineageResponse
- No tests for DocumentFullLineageResponse via DocumentsResource.getLineage
- No tests for ChunkLineageResponse via ChunksResource.getLineage
- No type-level interface structural checks
- No tests for CreateEntity with metadata/source_id
