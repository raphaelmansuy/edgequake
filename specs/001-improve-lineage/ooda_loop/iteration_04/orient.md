# Analysis - Iteration 04

## Gaps Identified

1. **PDF early metadata lacks lineage fields** — `file_size_bytes` and `sha256_checksum` are available from `PdfDocument` but not stored in early KV metadata. Users see incomplete lineage until processing completes.

2. **Markdown upload metadata misses unified lineage fields** — `content_length` and `content_hash` exist under different names but `file_size_bytes`, `sha256_checksum`, `document_type` are absent, breaking lineage consistency.

3. **`ensure_document_source_type` creates sparse metadata** — New metadata entries don't include `document_type`, `file_size_bytes`, or `sha256_checksum`.

4. **TextInsertData for PDF missing `sha256_checksum`** — Only `file_size_bytes` is forwarded, not the checksum.

## Possible Solutions

### Solution A: Add lineage fields directly in all metadata creation points

- **Approach**: Add `file_size_bytes`, `sha256_checksum`, `document_type` at every metadata creation site (early PDF metadata, markdown upload, `ensure_document_source_type`)
- **Pros**: Comprehensive coverage, fields available immediately
- **Cons**: Multiple edit points, needs block after `ensure_document_source_type` to merge from task metadata
- **Risk**: Low — all fields are `Option<T>` compatible, no schema change

### Solution B: Centralize metadata enrichment in `update_document_status_with_stats`

- **Approach**: Only add fields at completion time
- **Pros**: Single edit point
- **Cons**: Fields unavailable during processing stages, breaks lineage for in-progress documents
- **Risk**: Medium — violates early-availability principle

### Solution C: Refactor `ensure_document_source_type` into generic metadata merger

- **Approach**: Accept a JSON map of fields to merge into existing metadata
- **Pros**: Extensible, DRY
- **Cons**: Large refactor, changes function signature
- **Risk**: Medium — broader blast radius

## Recommendation

**Solution A** — Direct field injection at all creation points. The changes are small, targeted, and immediately effective. A metadata enrichment block after `ensure_document_source_type` in `process_text_insert` handles the task-metadata-to-document-metadata propagation cleanly.

## First Principles Justification

- **Lineage must be complete from creation** — metadata should never be "eventually consistent"
- **Unified field names** — regardless of source type, lineage fields should use the same names
- **Backward compatible** — all new fields are optional, won't break existing documents
