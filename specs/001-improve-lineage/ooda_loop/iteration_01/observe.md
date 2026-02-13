# Observation - Iteration 01

## Files Examined

- `edgequake/crates/edgequake-core/src/types/document.rs` (257 lines) — Document struct with id, content, file_path, status, content_length, content_summary, chunk_ids, metadata (JSON blob)
- `edgequake/crates/edgequake-core/src/types/chunk.rs` (130 lines) — Chunk struct with id, content, tokens, chunk_order_index, full_doc_id, file_path
- `edgequake/crates/edgequake-pipeline/src/lineage.rs` (717 lines) — SourceSpan, ExtractionMetadata, ChunkLineage, EntitySource, EntityLineage, RelationshipLineage, DocumentLineage, LineageBuilder
- `edgequake/crates/edgequake-storage/src/pdf_storage.rs` (508 lines) — PdfDocument with pdf_id, workspace_id, document_id, filename, file_size_bytes, sha256_checksum, page_count, vision_model
- `edgequake/crates/edgequake-api/src/handlers/lineage.rs` (604 lines) — Endpoints: get_chunk_detail, get_entity_provenance, get_entity_lineage, get_document_lineage
- `edgequake/crates/edgequake-api/src/handlers/lineage_types.rs` (528 lines) — DTOs for all lineage API responses
- `edgequake_webui/src/types/lineage.ts` (334 lines) — TypeScript lineage types
- `edgequake_webui/src/types/index.ts` (1172 lines) — Document interface with lineage field
- `edgequake_webui/src/components/document/metadata-sidebar.tsx` — MetadataSidebar with LineageTree, KeyStats, SourceInfoGrid, ProcessingDetails
- `sdks/rust/src/types/documents.rs` — Rust SDK document types (no lineage methods)
- `sdks/typescript/src/resources/documents.ts` (187 lines) — TS SDK document resource (no lineage methods)
- `sdks/typescript/src/types/lineage.ts` (260 lines) — TS SDK lineage types match Rust DTOs
- `sdks/python/edgequake/resources/documents.py` (483 lines) — Python SDK (no lineage methods)

## Tests Run

- `cargo test -p edgequake-pipeline --lib -- lineage` → 8 passed, 0 failed
  - test_source_span, test_extraction_metadata, test_chunk_lineage, test_entity_lineage, test_document_lineage, test_lineage_builder
  - test_pipeline_with/without_lineage_tracking

## Current State

### Document Level

- Document.id: MD5 hash of content (deterministic, enables dedup)
- Document.file_path: Optional, set on creation
- Document.content_length: Set in bytes on creation
- Document.metadata: `Option<serde_json::Value>` — generic JSON blob, underutilized
- MISSING: file_size (separate from content_length), document_type, sha256_checksum

### PDF Level

- PdfDocument has comprehensive metadata: pdf_id (UUID), sha256_checksum, file_size_bytes, page_count, vision_model
- PdfDocument.document_id links to Document (set after processing)
- Bidirectional linkage incomplete: Document doesn't store pdf_id

### Chunk Level

- Chunk has: id (MD5), content, tokens, chunk_order_index, full_doc_id, file_path
- MISSING from Chunk struct: start_line, end_line, start_offset, end_offset
- MISSING from Chunk struct: embedding_model, llm_model used for this chunk
- ChunkLineage (in lineage.rs) has these position fields, but Chunk struct does NOT

### Lineage Level

- DocumentLineage has SPEC-032 provider fields (extraction/embedding provider+model+dimension)
- ExtractionMetadata tracks llm_model, tokens, timing, cache per chunk
- MISSING: embedding_model per chunk (only at document level)
- LineageBuilder correctly populates all lineage structures

### API Level

- Routes: GET /lineage/entities/{name}, GET /lineage/documents/{id}, GET /chunks/{id}, GET /entities/{id}/provenance
- MISSING: Consolidated GET /documents/{id}/lineage (currently exists at /lineage/documents/{id})
- MISSING: GET /documents/{id}/metadata (single-call full metadata)
- MISSING: GET /chunks/{id}/lineage (chunk-specific lineage with parent refs)
- extraction_metadata field in ChunkDetailResponse is always None (line 196 comment: "Would need to be stored during extraction")

### SDK Level

- No lineage retrieval methods in any SDK (Rust, TypeScript, Python)
- TypeScript SDK has lineage types that match Rust DTOs

### WebUI Level

- MetadataSidebar shows: KeyStats, LineageTree, EntityRelationStats, SourceInfoGrid, ProcessingDetails
- Depends on document.lineage being populated
- UNKNOWN: Whether lineage is actually populated in API responses for documents
