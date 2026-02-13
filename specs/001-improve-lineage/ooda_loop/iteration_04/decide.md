# Decision - Iteration 04

## Changes to Make

1. **processor.rs:1963** — Add `file_size_bytes`, `sha256_checksum`, `page_count`, `document_type: "pdf"` to early PDF metadata JSON
2. **processor.rs:2144** — Add `sha256_checksum` and `document_type: "pdf"` to `TextInsertData.metadata` for PDF processing
3. **documents.rs:781** — Add `file_size_bytes` (= content_length), `sha256_checksum` (= content_hash), `document_type: "markdown"` to markdown upload metadata
4. **processor.rs:835** — Add metadata enrichment block after `ensure_document_source_type` in `process_text_insert` to propagate `file_size_bytes`, `sha256_checksum`, `document_type` from task metadata to KV metadata
5. **processor.rs:1728** — Add `document_type` field when creating new metadata in `ensure_document_source_type`

## Priority

1. **High impact, low effort** — Changes 1-3 ensure lineage fields exist from creation
2. **High impact, medium effort** — Change 4 covers the enrichment pipeline gap
3. **Low effort** — Change 5 ensures fallback metadata entries also have document_type

## Expected Outcome

After implementation, every document metadata entry in KV storage will contain:
- `file_size_bytes` — consistent name for document size (bytes)
- `sha256_checksum` — consistent name for content integrity hash
- `document_type` — "pdf" or "markdown" distinguishing source type

These fields will be available from the moment the document appears in the UI, not only after processing completes.
