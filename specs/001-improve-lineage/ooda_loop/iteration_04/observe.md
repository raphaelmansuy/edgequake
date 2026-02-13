# Observation - Iteration 04

## Files Examined

- `edgequake/crates/edgequake-api/src/processor.rs` (2592 lines)
  - `process_pdf_processing()` (line 1893) — creates early metadata JSON for PDF documents in KV storage
  - `process_text_insert()` (line 782) — orchestrates text/PDF ingestion, calls `ensure_document_source_type`
  - `ensure_document_source_type()` (line 1637) — creates/updates document metadata in KV storage
  - `update_document_status_with_stats()` (line 1774) — finalizes metadata with processing stats at completion
- `edgequake/crates/edgequake-api/src/handlers/documents.rs` (4768 lines)
  - `upload_document()` (line 670) — creates initial metadata for markdown uploads
  - Metadata JSON (line 781) — stores `content_length`, `content_hash` but NOT `file_size_bytes`, `sha256_checksum`, `document_type`

## Current State

### PDF Documents

- Early metadata (line ~1963) includes: `id`, `title`, `file_name`, `source_type: "pdf"`, `pdf_id`, `tenant_id`, `workspace_id`, `track_id`
- **Gap**: Does NOT include `file_size_bytes`, `sha256_checksum`, `page_count` despite being available from `PdfDocument`
- `TextInsertData.metadata` JSON (line ~2144) includes `file_size_bytes` but NOT `sha256_checksum`
- `PdfDocument` struct has both `file_size_bytes` and `sha256_checksum` available

### Markdown Documents

- Upload metadata (line ~781) includes: `content_length`, `content_hash`, `source_type: "markdown"`
- **Gap**: Does NOT include `file_size_bytes` (same as content_length), `sha256_checksum` (same as content_hash), `document_type`
- These are effectively available but under different field names

### `ensure_document_source_type`

- Creates new metadata with `source_type` but NOT `document_type`
- Does not propagate `file_size_bytes` or `sha256_checksum`

## Tests Run

- `cargo test --workspace --lib`: 1698 passed, 0 failed (baseline)
- `cargo clippy -p edgequake-api`: 0 warnings

## Key Finding

The metadata flow has two naming inconsistencies and three missing fields:

1. `content_length` vs `file_size_bytes` — same concept, different names
2. `content_hash` vs `sha256_checksum` — same concept, different names
3. `source_type` exists but `document_type` is missing
4. PDF early metadata missing `file_size_bytes` and `sha256_checksum`
5. `ensure_document_source_type` doesn't propagate these fields
