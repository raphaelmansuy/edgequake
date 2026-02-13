# OODA-15 Decide: Rust SDK Lineage Test Plan

## Decision
Add 18 lineage/metadata tests to `sdks/rust/tests/integration_tests.rs`

## Tests Added
1. Entity source_id/metadata/timestamps (sync)
2. CreateEntityRequest with metadata (sync)
3. ProvenanceRecord fields (sync)
4. LineageGraph/Node/Edge JSON roundtrips (sync)
5. DocumentFullLineage deserialization (sync)
6. ChunkLineageInfo fields (sync)
7. EntityStatistics fields (sync)
8. Entity create sends source_id (async, wiremock)
9. Lineage via provenance service (async, wiremock)
10. Provenance for entity with confidence (async, wiremock)
