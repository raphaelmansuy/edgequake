# OODA Iteration 08 - Act

## Implementation Summary

### Changes Made

1. **PdfUploadOptions struct** (pdf_upload.rs:57-75)
   - Added `force_reindex: bool` field
   - WHY: Allows users to explicitly request re-indexing of duplicate PDFs

2. **Multipart parsing** (pdf_upload.rs:384-390)
   - Added parsing for `force_reindex` field in multipart form
   - WHY: Enables API clients to pass the parameter

3. **Duplicate handling logic** (pdf_upload.rs:415-493)
   - When `force_reindex=true` on duplicate:
     - Clears existing graph/vector data via `clear_document_derived_data()`
     - Resets PDF status to `Processing`
     - Creates new processing task
     - Returns status="reindexing"
   - When `force_reindex=false` (default):
     - Returns status="duplicate" (unchanged behavior)

4. **clear_document_derived_data()** (pdf_upload.rs:1201-1274)
   - New helper function to clear graph entities and edges
   - Preserves raw PDF and markdown content
   - Handles source_id tracking for multi-document entities

### Build Result

```
cargo build --package edgequake-api
Finished `dev` profile [unoptimized + debuginfo] target(s) in 11.23s
```

### Test Result

```
cargo test --package edgequake-api --lib
test result: ok. 444 passed; 0 failed; 0 ignored
```

## API Documentation

### Upload PDF with Re-index

**Endpoint**: `POST /api/v1/documents/pdf`

**New Parameter**:
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `force_reindex` | boolean | false | When true, re-processes duplicate PDF |

**Response Statuses**:
| Status | Meaning |
|--------|---------|
| `processing` | New PDF being processed |
| `duplicate` | Existing PDF, not re-indexed |
| `reindexing` | Existing PDF being re-processed |

### Example: Force Re-index

```bash
curl -X POST http://localhost:8080/api/v1/documents/pdf \
  -H "X-Tenant-ID: tenant123" \
  -H "X-Workspace-ID: workspace456" \
  -F "file=@document.pdf" \
  -F "force_reindex=true"
```

**Response**:

```json
{
  "pdf_id": "abc123",
  "document_id": null,
  "status": "reindexing",
  "task_id": "pdf-xyz789",
  "message": "Re-indexing document. Previous graph/vector data cleared.",
  "estimated_time_seconds": 30,
  "metadata": {
    "filename": "document.pdf",
    "file_size_bytes": 1048576,
    "page_count": 16,
    "sha256_checksum": "...",
    "vision_enabled": true,
    "vision_model": "gpt-4o-mini"
  }
}
```

## Commit

```
OODA-08: Add force_reindex parameter for PDF upload

- Add force_reindex field to PdfUploadOptions struct
- Parse force_reindex from multipart form data
- When duplicate detected with force_reindex=true:
  - Clear existing graph/vector data
  - Reset PDF processing status
  - Create new processing task
  - Return status="reindexing"
- Add clear_document_derived_data() helper function
- All 444 tests pass

Implements: Document re-indexing support
```

## Next Steps

1. OODA-09: Configure OpenAI as default provider
2. OODA-10: Implement clean tenant setup for E2E tests
3. E2E test: Upload same PDF with force_reindex=true
