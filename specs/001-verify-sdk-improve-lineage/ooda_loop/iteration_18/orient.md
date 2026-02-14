# OODA-18: Orient — Java SDK Lineage Model Design

## Analysis

The Java SDK currently has basic lineage types (LineageNode/Edge/Graph, ProvenanceRecord, ChunkDetail) but lacks the comprehensive response types that match the actual API endpoints. The TypeScript SDK's `types/lineage.ts` (323 lines) is the gold standard reference.

## Design Decision

Create `LineageModels.java` as a new model file (SRP — separate from OperationModels.java) containing:

1. **EntityLineageResponse** — source_documents, description_versions, source_count
2. **SourceDocumentInfo** — document_id, chunk_ids, line_ranges
3. **LineRangeInfo** — start_line, end_line
4. **DescriptionVersionResponse** — version, description, source_chunk_id, created_at
5. **DocumentGraphLineageResponse** — entities, relationships, extraction_stats
6. **EntitySummaryResponse** — name, entity_type, source_chunks, is_shared
7. **RelationshipSummaryResponse** — source, target, keywords, source_chunks
8. **ExtractionStatsResponse** — totals, processing_time_ms
9. **ChunkDetailResponse** — full chunk detail with extraction_metadata
10. **CharRange** — start, end
11. **ExtractedEntityInfo** — id, name, entity_type, description
12. **ExtractedRelationshipInfo** — source_name, target_name, relation_type
13. **ExtractionMetadataInfo** — model, gleaning_iterations, duration_ms, tokens
14. **EntityProvenanceResponse** — sources, related_entities, total_extraction_count
15. **EntitySourceInfo** — document_id, chunks, first_extracted_at
16. **ChunkSourceInfo** — chunk_id, start_line, end_line, source_text
17. **RelatedEntityInfo** — entity_id, entity_name, relationship_type, shared_documents
18. **DocumentFullLineageResponse** — document_id, metadata, lineage
19. **ChunkLineageResponse** — chunk_id, document_id, position info, entity_count

## Risk Assessment

- Low risk: purely additive, no existing code changes
- All field names verified against TypeScript SDK and backend Rust types
