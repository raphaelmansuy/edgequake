# OODA-24 Act: C# SDK Lineage Models + Service

## Changes Made

### New Files

- `sdks/csharp/src/EdgeQuakeSDK/LineageModels.cs` — 19 model classes (~210 lines)
  - EntityLineageResponse, SourceDocumentInfo, LineRangeInfo, DescriptionVersionResponse
  - DocumentGraphLineageResponse, EntitySummaryResponse, RelationshipSummaryResponse, ExtractionStatsResponse
  - ChunkDetailResponse, CharRange, ExtractedEntityInfo, ExtractedRelationshipInfo, ExtractionMetadataInfo
  - EntityProvenanceResponse, EntitySourceInfo, ChunkSourceInfo, RelatedEntityInfo
  - DocumentFullLineageResponse, ChunkLineageResponse

- `sdks/csharp/src/EdgeQuakeSDK/LineageService.cs` — 7 async methods (~60 lines)
  - `EntityLineageAsync(entityName)` → EntityLineageResponse
  - `DocumentLineageAsync(documentId)` → DocumentGraphLineageResponse
  - `DocumentFullLineageAsync(documentId)` → DocumentFullLineageResponse
  - `ExportLineageAsync(documentId, format)` → JsonElement (via GetRawAsync)
  - `ChunkDetailAsync(chunkId)` → ChunkDetailResponse
  - `ChunkLineageAsync(chunkId)` → ChunkLineageResponse
  - `EntityProvenanceAsync(entityId)` → EntityProvenanceResponse

### Modified Files

- `sdks/csharp/src/EdgeQuakeSDK/EdgeQuakeClient.cs` — Added `Lineage` property (16→17 services)
- `sdks/csharp/tests/EdgeQuakeSDK.Tests/LineageTest.cs` — Updated service count 16→17

## Test Evidence

```
dotnet test --filter "FullyQualifiedName~LineageTest"
Passed! - Failed: 0, Passed: 39, Total: 39

dotnet test --filter "FullyQualifiedName~UnitTest"
Passed! - Failed: 0, Passed: 79, Total: 79

Total: 118 unit tests passing, 0 failures
```

## Commit

SHA: (pending)
