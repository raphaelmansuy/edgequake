# OODA-20: Act — Java SDK Lineage Tests (34 new)

## Execution

- Added 34 lineage tests to UnitTest.java
- Fixed exportLineage return type: String → Map<String,Object> (HttpHelper always deserializes JSON)
- Tests: 157 pass (was 123, +34)
- Commit: see below

## Tests Added

### LineageService endpoint tests (9):

- lineageEntityLineageEndpoint (full response validation)
- lineageDocumentLineageEndpoint (entities, relationships, extraction_stats)
- lineageDocumentFullLineage (metadata, lineage maps)
- lineageExportJson, lineageExportCsv, lineageExportDefaultFormat
- lineageChunkDetail (charRange, entities, relationships, extractionMetadata)
- lineageChunkLineage (position info, entity_count, entity_names)
- lineageEntityProvenance (sources, chunks, related_entities)

### LineageModels field tests (20):

- entityLineageResponseFields, sourceDocumentInfoFields, lineRangeInfoFields
- descriptionVersionFields, documentGraphLineageFields, entitySummaryResponseFields
- extractionStatsFields, chunkDetailResponseFields, charRangeFields
- extractedEntityInfoFields, extractedRelationshipInfoFields, extractionMetadataInfoFields
- entityProvenanceResponseFields, entitySourceInfoFields, chunkSourceInfoFields
- relatedEntityInfoFields, documentFullLineageFields, chunkLineageResponseFields

### Error + edge case tests (5):

- lineageServiceError (404), lineageServiceServerError (500)
- entityLineageEmptySourceDocuments, chunkLineageNullOptionalFields
- entityProvenanceMultipleSources, documentGraphLineageNoEntities
- lineageEntityNameUrlEncoded
