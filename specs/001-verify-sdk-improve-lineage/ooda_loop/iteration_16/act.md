# OODA-16: Act — TypeScript SDK Lineage Tests

## Execution
- Created `sdks/typescript/tests/unit/lineage.test.ts` with 41 tests
- All 41 new tests pass
- Full suite: 288 tests (was 247, +41)
- Committed as `85d68f11`

## Test Coverage Added
- EntityLineageResponse with source_documents/description_versions (4 tests)
- DocumentGraphLineageResponse with extraction_stats (3 tests)
- DocumentFullLineageResponse via DocumentsResource.getLineage (3 tests)
- DocumentsResource.getMetadata (1 test)
- ChunkLineageResponse via ChunksResource.getLineage (6 tests)
- ChunkDetailResponse with extraction_metadata (4 tests)
- EntityProvenanceResponse with sources/related_entities (4 tests)
- CreateEntity with metadata/source_id (2 tests)
- Relationship create with metadata (1 test)
- Type-level interface structural checks (13 tests)
