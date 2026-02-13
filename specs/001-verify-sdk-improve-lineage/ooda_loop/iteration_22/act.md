# OODA-22: Act — Kotlin SDK lineage models + service

## Changes
- Created `sdks/kotlin/src/main/kotlin/io/edgequake/sdk/models/LineageModels.kt` (19 data classes, ~180 lines)
- Created `sdks/kotlin/src/main/kotlin/io/edgequake/sdk/resources/LineageService.kt` (7 methods)
- Updated `EdgeQuakeClient.kt` to add `lineage` service accessor

## Models Added
EntityLineageResponse, SourceDocumentInfo, LineRangeInfo, DescriptionVersionResponse,
DocumentGraphLineageResponse, EntitySummaryResponse, RelationshipSummaryResponse,
ExtractionStatsResponse, ChunkDetailResponse, CharRange, ExtractedEntityInfo,
ExtractedRelationshipInfo, ExtractionMetadataInfo, EntityProvenanceResponse,
EntitySourceInfo, ChunkSourceInfo, RelatedEntityInfo, DocumentFullLineageResponse,
ChunkLineageResponse

## Service Methods
1. `entityLineage(name)` → EntityLineageResponse
2. `documentLineage(id)` → DocumentGraphLineageResponse
3. `documentFullLineage(id)` → DocumentFullLineageResponse
4. `exportLineage(id, format)` → Map<String, Any?>
5. `chunkDetail(id)` → ChunkDetailResponse
6. `chunkLineage(id)` → ChunkLineageResponse
7. `entityProvenance(id)` → EntityProvenanceResponse

## Evidence
```
mvn compile -q → BUILD SUCCESS (clean compile, no warnings)
```

## Next
OODA-23: Add lineage tests to Kotlin SDK.
