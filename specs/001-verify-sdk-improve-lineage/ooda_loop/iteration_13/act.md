# OODA-13 Act: Kotlin SDK Lineage Tests

## Actions Taken
1. Added 23 lineage/metadata tests to UnitTest.kt
2. Fixed 7 field name mismatches:
   - Document: no `entityCount`/`fileName` → used `title`/`chunkCount`
   - EntityExistsResponse: only `entityId`/`exists`
   - EntityService.merge: takes `(source, target)` strings
   - ProviderStatus: map-based, not `currentProvider`
   - BulkDeleteResponse: `deleted` not `deletedCount`
   - TaskInfo: `id` not `trackId`
   - GraphEdge: `label` not `edgeType`
3. All 122 tests pass

## Commit
- Hash: `bbff393e`
- Message: `feat(kotlin-sdk): add 23 lineage/metadata tests (122 total)`
