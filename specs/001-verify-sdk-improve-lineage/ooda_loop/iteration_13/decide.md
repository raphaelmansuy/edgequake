# OODA-13 Decide: Kotlin SDK Lineage Test Plan

## Decision
Add 23 lineage/metadata tests to `sdks/kotlin/src/test/kotlin/io/edgequake/sdk/UnitTest.kt`

## Test Categories
1. Entity source_id, metadata, timestamps (3 tests)
2. CreateEntityRequest with metadata (2 tests)
3. GraphNode/Edge provenance properties (3 tests)
4. Document lineage fields (2 tests)
5. BulkDeleteResponse, TaskInfo, ProviderStatus fields (3 tests)
6. EntityService.merge with string params (1 test)
7. JSON roundtrip for lineage models (4 tests)
8. EntityStatistics fields (2 tests)
9. SourceReference, ChatSourceReference (3 tests)

## Risk Mitigation
- Read actual Kotlin data class definitions before writing tests
- Match field names exactly to avoid compilation errors
