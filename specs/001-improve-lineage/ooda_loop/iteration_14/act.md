# Action - Iteration 14

## Changes Made

### sdks/rust/src/types/operations.rs
- Added `DocumentFullLineage` struct (document_id, metadata, lineage)
- Added `ChunkLineageInfo` struct (17 fields matching API response)
- Both use `#[serde(default)]` for backward compat

### sdks/rust/src/resources/documents.rs
- Added `get_lineage(&self, id: &str) -> Result<DocumentFullLineage>` → GET /documents/:id/lineage
- Added `get_metadata(&self, id: &str) -> Result<serde_json::Value>` → GET /documents/:id/metadata

### sdks/rust/src/resources/chunks.rs
- Added `get_lineage(&self, id: &str) -> Result<ChunkLineageInfo>` → GET /chunks/:id/lineage
- Updated import to include `ChunkLineageInfo`

## Verification
- `cargo build` — CLEAN (0 warnings)
- `cargo test` — 54 passed + 1 doc-test, 0 failures
