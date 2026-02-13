# OODA-15 Observe: Rust SDK Lineage Tests

## Current State
- Rust SDK at `sdks/rust/` with 52 existing integration tests
- Uses wiremock for HTTP mocking
- Rich lineage types: Entity (source_id, metadata), CreateEntityRequest, ProvenanceRecord
- LineageGraph/Node/Edge, DocumentFullLineage, ChunkLineageInfo, EntityStatistics
- Client API: `client.provenance()` → ProvenanceResource with `for_entity()` and `lineage()`
- URLs: `/api/v1/entities/{name}/provenance` and `/api/v1/entities/{name}/lineage`
