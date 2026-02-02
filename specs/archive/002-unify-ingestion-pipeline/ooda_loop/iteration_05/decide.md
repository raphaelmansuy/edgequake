# OODA-05: Decide

## Date: 2026-02-01

## Decision

Implement **Option A**: Include tenant_id/workspace_id in metadata JSON at all PDF processing stages.

## Specific Changes

### 1. Fix `process_pdf_processing` (processor.rs:1612-1625)

Add tenant_id and workspace_id to the metadata JSON:

```rust
let text_data = edgequake_tasks::TextInsertData {
    text: markdown,
    file_source: pdf.filename.clone(),
    workspace_id: data.workspace_id.to_string(),
    metadata: Some(json!({
        "source": "pdf_upload",
        "source_type": "pdf",
        "pdf_id": data.pdf_id.to_string(),
        "filename": pdf.filename,
        "page_count": pdf.page_count,
        "file_size_bytes": pdf.file_size_bytes,
        "tenant_id": data.tenant_id.to_string(),      // ADD
        "workspace_id": data.workspace_id.to_string(), // ADD
    })),
};
```

### 2. Fix `ensure_document_source_type` (processor.rs:1260-1275)

When creating new metadata, propagate tenant/workspace from task context:

- Extract tenant_id/workspace_id from task data
- Include in new metadata JSON

### 3. Fix `update_document_status` (processor.rs:1188-1200)

When creating new metadata, propagate tenant/workspace from task context:

- Accept optional tenant/workspace parameters
- Include in new metadata JSON

### 4. Add PdfProcessingData.tenant_id

Check if `PdfProcessingData` struct includes tenant_id - if not, add it.

## Priority

**Critical** - This bug prevents PDF documents from appearing in the UI.

## Implementation Order

1. Check/add tenant_id to PdfProcessingData struct
2. Update process_pdf_processing to include context in metadata
3. Update ensure_document_source_type to propagate context
4. Update update_document_status to propagate context
5. Build and test
6. Verify via Playwright E2E

## Test Cases

1. Upload PDF via frontend
2. Verify document appears in list
3. Verify document metadata has tenant_id and workspace_id
4. Verify query returns PDF document
5. Verify entities reference correct workspace
