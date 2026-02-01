# OODA-05 Act: Fix PDF Document Visibility Bug

## Date: 2026-02-01

## Summary

Fixed critical bug where PDF documents uploaded via frontend were not visible in the documents list. The root cause was missing tenant_id/workspace_id context in document metadata.

## Changes Made

### 1. types.rs (edgequake-tasks, lines 615-632)
**File:** `edgequake/crates/edgequake-tasks/src/types.rs`
**Change:** Added `tenant_id: Uuid` field to `PdfProcessingData` struct

```rust
pub struct PdfProcessingData {
    pub pdf_id: Uuid,
    pub tenant_id: Uuid,        // OODA-05: Added for multi-tenant context
    pub workspace_id: Uuid,
    pub enable_vision: bool,
    // ...
}
```

**WHY:** Task data needs to carry tenant context so processor can include it in document metadata.

### 2. pdf_upload.rs (lines 829-840)
**File:** `edgequake/crates/edgequake-api/src/handlers/pdf_upload.rs`
**Change:** Include tenant_id when creating PdfProcessingData

```rust
let task_data = PdfProcessingData {
    pdf_id,
    tenant_id,              // OODA-05: Pass tenant context
    workspace_id,
    enable_vision,
    // ...
};
```

**WHY:** Upload handler has access to tenant context and must pass it to processing task.

### 3. processor.rs (lines 1652-1668, 1633-1640, 1246-1316)
**File:** `edgequake/crates/edgequake-api/src/processor.rs`

**Change 3a:** Include tenant_id/workspace_id in TextInsertData metadata (lines 1652-1668)
```rust
metadata: Some(json!({
    "source": "pdf_upload",
    "source_type": "pdf",
    "tenant_id": data.tenant_id.to_string(),
    "workspace_id": data.workspace_id.to_string(),
    // ...
})),
```

**Change 3b:** Updated `ensure_document_source_type` call (lines 1633-1640)
```rust
self.ensure_document_source_type(
    &document_id,
    &source_type,
    tenant_id.as_deref(),
    Some(&data.workspace_id),
)
.await?;
```

**Change 3c:** Modified `ensure_document_source_type` signature (lines 1246-1316)
Added `tenant_id: Option<&str>` and `workspace_id: Option<&str>` parameters.
When creating new metadata, now includes these fields for multi-tenant visibility.

**WHY:** Document metadata MUST contain tenant/workspace context or it becomes invisible in workspace-filtered queries.

## Test Results

### Before Fix
- PDF upload completes (status: indexed)
- Documents list shows: **0 documents**
- Root cause: Document metadata missing tenant_id/workspace_id

### After Fix
- PDF upload completes (status: completed)  
- Documents list shows: **1 document** (AgenticPlatformReference Architecture.pdf)
- Document metadata includes: `tenant_id: "7a1e4dca-ffe5-44a9-92ca-bf737acbed00"`, `workspace_id: "cd284095-67f8-47b2-a85c-e2f4f4fbb532"`
- 12 entities extracted, 6 relationships, $0.0057 cost

## Verification Steps

1. Started backend with fix: `make backend-bg`
2. Navigated to documents page via Playwright
3. Uploaded `AgenticPlatformReference Architecture.pdf`
4. Waited 30s for processing to complete
5. Verified document appears in list with correct metadata

## Commit

Ready to commit with message:
```
fix(processor): Include tenant/workspace in PDF document metadata

OODA-05: Fix PDF document visibility bug where uploaded PDFs were
not appearing in the documents list.

Root cause: Document metadata created without tenant_id/workspace_id
fields, so workspace-filtered queries returned 0 results.

Changes:
- Add tenant_id field to PdfProcessingData struct
- Pass tenant_id from pdf_upload handler to processing task
- Include tenant_id/workspace_id in TextInsertData metadata
- Modify ensure_document_source_type to propagate tenant/workspace context

Tested: PDF upload → processing → document visible in correct workspace

Refs: SPEC-002, OODA-05
```
