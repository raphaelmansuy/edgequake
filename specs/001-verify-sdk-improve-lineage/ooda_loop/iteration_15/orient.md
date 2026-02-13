# OODA-15 Orient: Rust SDK Lineage Gap Analysis

## Gaps Found
- No tests for Entity source_id, metadata, timestamps JSON roundtrips
- No tests for CreateEntityRequest with source_id/metadata
- No tests for ProvenanceRecord field-level validation
- No tests for LineageGraph/Node/Edge deserialization
- No tests for DocumentFullLineage, ChunkLineageInfo, EntityStatistics
- Only 1 existing provenance test (for_entity), no lineage test
