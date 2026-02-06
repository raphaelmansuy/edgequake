# OODA Iteration 08 - Orient

## Analysis

### Option Evaluation Matrix

| Option                   | Effort        | User Value | Risk   | Recommendation   |
| ------------------------ | ------------- | ---------- | ------ | ---------------- |
| A: `force_reindex` param | Medium (2-3h) | High       | Low    | ✅ SELECTED      |
| B: `/reindex` endpoint   | Medium (3-4h) | Medium     | Low    | Future work      |
| C: Hybrid                | High (5-6h)   | High       | Medium | Overkill for now |

### Option A: Add `force_reindex` Parameter (Selected)

**Changes Required**:

1. **pdf_upload.rs** - Add `force_reindex` field to `PdfUploadOptions`:

   ```rust
   pub struct PdfUploadOptions {
       pub force_reindex: bool,  // NEW
       // ... existing fields
   }
   ```

2. **pdf_upload.rs** - Modify duplicate handling logic:

   ```rust
   if let Some(existing) = find_pdf_by_checksum(...) {
       if options.force_reindex {
           // Delete existing graph/vector data
           // Reset processing status
           // Create new processing task
       } else {
           return Ok(duplicate_response);
       }
   }
   ```

3. **Cleanup operations for re-indexing**:
   - Delete vectors associated with document
   - Delete graph entities/relationships for document
   - Reset document status
   - Create new extraction task

### Data Flow for Re-indexing

```
┌──────────────────────────────────────────────────────────────┐
│                    PDF Upload with force_reindex=true        │
└──────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────┐
│ 1. Calculate SHA-256 checksum                                │
│ 2. Find existing PDF by checksum                             │
│ 3. IF EXISTS AND force_reindex=true:                         │
│    a. Get associated document_id                             │
│    b. Delete vectors for document                            │
│    c. Delete graph entities/relationships for document       │
│    d. Update pdf_documents.processing_status = 'pending'     │
│    e. Create new TaskType::PdfProcessing task                │
│ 4. ELSE IF EXISTS AND force_reindex=false:                   │
│    Return "duplicate" status                                 │
│ 5. ELSE (NEW):                                               │
│    Store PDF and create processing task                      │
└──────────────────────────────────────────────────────────────┘
```

### First Principles Analysis

**Why re-indexing is valuable**:

1. LLM extraction quality improves over time (model updates)
2. User may want to test different extraction parameters
3. Existing documents may have failed extraction (Ollama timeout)
4. Development/debugging requires repeated testing

**What re-indexing must do**:

1. Clear old extraction results (vectors, entities, relationships)
2. Re-run extraction pipeline with current LLM/config
3. Preserve document metadata (filename, upload date)
4. Update processing timestamps

### Risk Assessment

| Risk                 | Likelihood | Impact | Mitigation                                 |
| -------------------- | ---------- | ------ | ------------------------------------------ |
| Orphaned vectors     | Low        | Medium | Delete vectors by document_id              |
| Orphaned graph nodes | Low        | Medium | Delete by document reference               |
| Race condition       | Low        | Low    | Use transaction for cleanup                |
| Data loss            | Low        | High   | Only delete derived data, preserve raw PDF |

### Implementation Plan

1. Add `force_reindex` field to `PdfUploadOptions`
2. Add multipart parsing for `force_reindex`
3. Add cleanup helper function `cleanup_document_data()`
4. Modify duplicate detection to check `force_reindex`
5. Add `update_pdf_status()` to reset processing state
6. Write tests for re-indexing flow

### Dependencies

- `edgequake_storage::VectorStorage` - Need `delete_by_document()` or similar
- `edgequake_storage::GraphStorage` - Need `delete_document_data()`
- `edgequake_storage::PdfDocumentStorage` - Need `update_processing_status()`

## Decision

Implement Option A: Add `force_reindex` parameter to PDF upload endpoint.

This provides immediate value with moderate effort and low risk. The raw PDF data is preserved, only derived data (vectors, graph) is cleared for re-extraction.
