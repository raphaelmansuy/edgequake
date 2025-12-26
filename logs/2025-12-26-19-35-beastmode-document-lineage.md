# Task Log: Document Lineage and Tenant Guard Implementation

## Session Date: 2025-12-26

## Actions
- Created TenantGuard component to ensure tenant/workspace is always selected
- Updated upload_document handler to use TenantContext for tenant scoping
- Added DocumentLineage struct with extraction metadata fields
- Updated DocumentDetailResponse with content_hash, relationship_count, tenant_id, workspace_id, lineage
- Enhanced ProcessingStats with llm_model, embedding_model, entity_types, relationship_types, keywords
- Updated pipeline.process() to populate lineage fields from extractor and embedding provider
- Added model_name() method to EntityExtractor trait
- Updated processor to store lineage information in document metadata
- Updated frontend Document and DocumentLineage types
- Added lineage section to document detail page UI
- Fixed delete_document handler to also check for metadata/content keys

## Decisions
- Used Option<T> for all lineage fields to support legacy documents without lineage data
- Capped keywords at 50 entries to prevent excessive data storage
- Made model_name() a default trait method returning "unknown" for backwards compatibility
- Stored lineage data in document metadata JSON for simple retrieval

## Next Steps
- Consider adding lineage data to document list view (summary)
- Add unit tests for lineage data population
- Test with real LLM provider to verify model names are captured correctly

## Lessons/Insights
- The delete_document handler was only checking for chunk keys, causing 404 for documents with metadata
- Tests need tenant headers to properly test multi-tenancy scenarios
- ProcessingStats is a good place to aggregate lineage info during pipeline processing
