# OODA-16: Decide — TypeScript SDK Lineage Tests

## Plan
1. Create `sdks/typescript/tests/unit/lineage.test.ts`
2. Add 41 tests across 10 describe blocks
3. Cover: EntityLineageResponse, DocumentGraphLineageResponse, DocumentFullLineageResponse, ChunkLineageResponse, ChunkDetailResponse, EntityProvenanceResponse, CreateEntity, Relationship, type structural checks
4. Run vitest to verify all pass
5. Commit changes
