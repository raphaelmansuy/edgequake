# Observation - Iteration 14

## Files Examined
- `sdks/rust/src/resources/documents.rs` (63 lines) - Has list/get/delete/status/upload_text/scan/deletion_impact/track
- `sdks/rust/src/resources/chunks.rs` (26 lines) - Has list/get
- `sdks/rust/src/resources/provenance.rs` (35 lines) - Has for_entity/lineage
- `sdks/rust/src/types/operations.rs` (268 lines) - Has LineageGraph, ChunkDetail, ProvenanceRecord

## Tests Run
- `cargo test` in `sdks/rust/`: 54 passed + 1 doc-test
- `cargo build`: Clean (0 warnings)

## Gap Identified
- No `get_lineage()` or `get_metadata()` methods on DocumentsResource
- No `get_lineage()` method on ChunksResource
- No types for DocumentFullLineage or ChunkLineageInfo
