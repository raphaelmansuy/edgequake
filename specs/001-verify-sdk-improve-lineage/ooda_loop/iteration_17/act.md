# OODA-17: Act — Python SDK Lineage Tests

## Execution
- Created `sdks/python/tests/test_lineage.py` with 35 tests in 10 classes
- All 35 new tests pass
- Full suite: 520 passed, 32 skipped (was 485, +35)
- Committed as `e5329c4a`

## Test Coverage Added
- TestEntityLineageFields (7 tests): source_id, metadata, timestamps, degree
- TestEntityCreateMetadata (3 tests): serialization alias name→entity_name
- TestEntityDetail (2 tests): inherited fields + extra
- TestLineageGraph (4 tests): nodes, edges, root_id
- TestDocumentFullLineage (3 tests): chunk_count, entity_count, processing_time
- TestChunkLineageInfo (3 tests): position, doc_id, entity count
- TestProvenanceRecordLineage (4 tests): confidence, extraction_method
- TestRelationshipMetadata (2 tests): source_id, metadata dict
- TestProviderStatusLineage (1 test): provider_name/model/status
- TestLineageEdgeCases (6 tests): nested metadata, zero counts, roundtrips
