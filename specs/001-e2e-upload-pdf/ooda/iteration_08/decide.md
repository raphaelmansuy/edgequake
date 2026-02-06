# OODA Iteration 08 - Decide

## Decision

**Implement `force_reindex` parameter for PDF upload endpoint.**

## Implementation Steps

### Step 1: Update `PdfUploadOptions` struct

**File**: `edgequake/crates/edgequake-api/src/handlers/pdf_upload.rs`

Add new field:
```rust
pub struct PdfUploadOptions {
    pub force_reindex: bool,  // NEW: Force re-indexing of duplicate PDF
    // ... existing fields
}
```

### Step 2: Parse `force_reindex` from multipart

**File**: `edgequake/crates/edgequake-api/src/handlers/pdf_upload.rs`

Add parsing in the multipart loop:
```rust
Some("force_reindex") => {
    if let Ok(text) = field.text().await {
        options.force_reindex = text.parse().unwrap_or(false);
    }
}
```

### Step 3: Modify duplicate handling logic

**File**: `edgequake/crates/edgequake-api/src/handlers/pdf_upload.rs`

When duplicate detected and `force_reindex=true`:
1. Get existing document_id from pdf_documents
2. Call `clear_document_data()` to remove graph/vector data
3. Reset pdf_documents.processing_status to 'pending'
4. Create new PdfProcessing task
5. Return status="reindexing"

### Step 4: Add `clear_document_data()` helper

This function will:
1. Delete vectors associated with document
2. Delete graph entities/relationships for document
3. Keep raw PDF and markdown (for faster re-extraction if LLM unchanged)

### Step 5: Add `reset_pdf_processing_status()` to storage

**File**: `edgequake/crates/edgequake-storage/src/pdf_storage.rs`

Add method to reset PDF status to 'pending'.

### Step 6: Write tests

1. Test re-upload with `force_reindex=false` → returns "duplicate"
2. Test re-upload with `force_reindex=true` → returns "reindexing"
3. Test graph/vector cleanup on re-index
4. Test task creation on re-index

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Race condition | Reset status atomically with task creation |
| Data loss | Only delete derived data (vectors, graph), preserve raw PDF |
| Orphaned tasks | Cancel existing pending tasks before creating new one |

## Success Criteria

1. ✅ Upload same PDF with `force_reindex=true` triggers re-processing
2. ✅ Old graph entities/vectors are cleaned up
3. ✅ New extraction task is created
4. ✅ All existing tests pass
5. ✅ No regression in normal upload flow

## Effort Estimate

- Code changes: 2-3 hours
- Testing: 1 hour
- Documentation: 30 minutes

**Total: ~4 hours**
