# OODA-18: Act — Java SDK LineageModels.java

## Execution

- Created `sdks/java/src/main/java/io/edgequake/sdk/models/LineageModels.java` (19 model classes, 205 lines)
- All 123 existing tests pass (0 failures, 0 errors)
- Build with JDK 23 (target 21) successful

## Model Classes Added

1. EntityLineageResponse — entity_name, source_documents, description_versions
2. SourceDocumentInfo — document_id, chunk_ids, line_ranges
3. LineRangeInfo — start_line, end_line
4. DescriptionVersionResponse — version, description, source_chunk_id, created_at
5. DocumentGraphLineageResponse — entities, relationships, extraction_stats
6. EntitySummaryResponse — name, entity_type, source_chunks, is_shared
7. RelationshipSummaryResponse — source, target, keywords, source_chunks
8. ExtractionStatsResponse — totals, processing_time_ms
9. ChunkDetailResponse — full chunk with extraction_metadata
10. CharRange — start, end
11. ExtractedEntityInfo — id, name, entity_type, description
12. ExtractedRelationshipInfo — source_name, target_name, relation_type
13. ExtractionMetadataInfo — model, gleaning_iterations, duration_ms, tokens
14. EntityProvenanceResponse — sources, related_entities, total_extraction_count
15. EntitySourceInfo — document_id, chunks, first_extracted_at
16. ChunkSourceInfo — chunk_id, start_line, end_line, source_text
17. RelatedEntityInfo — entity_id, entity_name, relationship_type, shared_documents
18. DocumentFullLineageResponse — document_id, metadata, lineage
19. ChunkLineageResponse — chunk_id, position info, entity_count, entity_names

## Test Results

```
Tests run: 123, Failures: 0, Errors: 0, Skipped: 0
BUILD SUCCESS
```
