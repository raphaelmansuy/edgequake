# OODA-26 Act: Swift SDK Lineage Implementation

## Changes Made

### New Files
- `sdks/swift/Sources/EdgeQuakeSDK/LineageModels.swift` — 19 structs (~230 lines)
  - EntityLineageResponse, SourceDocumentInfo, LineRangeInfo, DescriptionVersionResponse
  - DocumentGraphLineageResponse, EntitySummaryResponse, RelationshipSummaryResponse, ExtractionStatsResponse
  - ChunkDetailResponse, CharRange, ExtractedEntityInfo, ExtractedRelationshipInfo, ExtractionMetadataInfo
  - EntityProvenanceResponse, EntitySourceInfo, ChunkSourceInfo, RelatedEntityInfo
  - DocumentFullLineageResponse, ChunkLineageResponse

- `sdks/swift/Sources/EdgeQuakeSDK/LineageService.swift` — 7 methods (~70 lines)
  - entityLineage(name:), documentLineage(id:), documentFullLineage(id:)
  - exportLineage(id:format:), chunkDetail(id:), chunkLineage(id:), entityProvenance(id:)

### Modified Files
- `sdks/swift/Sources/EdgeQuakeSDK/EdgeQuakeClient.swift` — added `lineage: LineageService` (16→17 services)
- `sdks/swift/Sources/EdgeQuakeSDK/Services.swift` — added convenience methods:
  - ChatService: complete(request:), getConversation(id:), listConversations(), bulkDeleteConversations(ids:), listFolders()
  - QueryService: query(request:)
  - DocumentService: uploadText(request:)
  - EntityService: get(id:), create(request:), delete(id:)
  - ModelService: providerHealth(name:), status()
- `sdks/swift/Tests/EdgeQuakeSDKTests/LineageTest.swift` — added 19 LineageServiceTest tests
- `sdks/swift/Tests/EdgeQuakeSDKTests/UnitTest.swift` — updated testInitializesAllServices (14→17 assertions)

### Bug Fixes
- Fixed CreateEntityLineageTest.testRequestEncoding: added `.convertFromSnakeCase` to JSONDecoder

## Test Results
- 129 unit tests passing (0 non-E2E failures)
- 21 E2E tests failing (expected — no live backend)
- Total: 150 tests executed

## Commit
- SHA: (pending)
- Message: `OODA-26: Swift SDK lineage — 19 models, 7 service methods, 19 tests (129 passing)`
