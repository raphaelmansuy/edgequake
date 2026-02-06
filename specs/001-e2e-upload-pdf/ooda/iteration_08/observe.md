# OODA Iteration 08 - Observe

## Mission Re-read Confirmation

✅ Mission file read at: 2026-02-07 (timestamp)
✅ Critical Safety Mandate: COMPLIED

## Focus: Document Re-indexing

### Current Behavior Analysis

**PDF Upload Flow** (`edgequake/crates/edgequake-api/src/handlers/pdf_upload.rs`):

1. Parse multipart form data (lines 303-395)
2. Validate PDF file format and signature
3. Calculate SHA-256 checksum (line 366)
4. Check for duplicates via `find_pdf_by_checksum()` (lines 403-449)
5. If duplicate: Return status="duplicate" WITHOUT re-processing
6. If new: Store PDF and create background task

**Problem**: When same document uploaded, it returns "duplicate" status and does NOT re-index.

### Code Evidence

```rust
// pdf_upload.rs:403-449 - Duplicate detection
if let Some(existing) = pdf_storage
    .find_pdf_by_checksum(&workspace_id, &checksum)
    .await
{
    return Ok(Json(PdfUploadResponse {
        status: "duplicate".to_string(),  // <-- Returns immediately
        message: format!("PDF already uploaded with ID: {}", existing.pdf_id),
        ...
    }));
}
```

### TaskType::Reindex Status

```rust
// processor.rs:1983-1987
TaskType::Reindex => {
    // Reindexing not yet implemented
    Err(edgequake_tasks::TaskError::UnsupportedOperation(
        "Reindexing not yet implemented".to_string(),
    ))
}
```

The `TaskType::Reindex` exists but is not implemented.

### Data Model Investigation

**PDF Document Table** (`pdf_documents`):
- `pdf_id` (UUID, PK)
- `workspace_id` (UUID)
- `document_id` (UUID, nullable, FK to documents)
- `sha256_checksum` (text)
- `processing_status` (enum: pending, processing, completed, failed)

**Relationship Chain**:
```
pdf_documents → documents → entities/relationships (graph)
                         → embeddings (vectors)
```

### Re-indexing Requirements

1. **Option A: Add `force_reindex` parameter**
   - Add boolean field to upload multipart form
   - When true, delete existing graph/vector data
   - Re-process the document

2. **Option B: Implement separate `/reindex` endpoint**
   - POST `/api/v1/documents/:id/reindex`
   - Triggers re-extraction of entities/embeddings

3. **Option C: Hybrid approach**
   - `force_reindex` on upload
   - Plus dedicated `/reindex` endpoint for existing documents

### Service Health

```json
{
  "status": "healthy",
  "storage_mode": "postgresql",
  "llm_provider_name": "ollama",
  "components": {
    "kv_storage": true,
    "vector_storage": true,
    "graph_storage": true,
    "llm_provider": true
  }
}
```

### Additional Requirements Discovered

1. **OpenAI Provider**: Currently using Ollama, need to switch to OpenAI
2. **Clean Tenant**: Need tenant isolation for E2E tests
3. **Test Timeouts**: Need to add timeouts to all tests

## Next Steps (Orient Phase)

1. Analyze impact of each re-indexing option
2. Evaluate effort vs. value
3. Consider data model implications
4. Plan implementation approach
