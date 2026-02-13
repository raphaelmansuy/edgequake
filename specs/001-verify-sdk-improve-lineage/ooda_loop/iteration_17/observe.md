# OODA-17: Observe — Python SDK Lineage Tests

## Baseline
- 485 tests passing across 16 test files
- Existing coverage: get_lineage, provenance, metadata at service level
- Types: Entity (source_id, metadata, timestamps), EntityCreate (serialization_alias), LineageNode/Edge/Graph, DocumentFullLineage, ChunkLineageInfo, ProvenanceRecord

## Gap Analysis
- No tests for Entity lineage fields (source_id, metadata, timestamps)
- No tests for EntityCreate serialization alias (name→entity_name)
- No tests for EntityDetail subclass
- No tests for LineageGraph with nodes/edges
- No tests for ChunkLineageInfo full fields
- No tests for ProvenanceRecord field-level validation
- No tests for RelationshipCreate source_id
