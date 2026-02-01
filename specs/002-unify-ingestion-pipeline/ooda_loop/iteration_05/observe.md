# OODA-05: Observe

## Date: 2026-02-01

## Mission Reminder

**RE-READ**: `./specs/002-unify-ingestion-pipeline.md`

## Observation Context

Tested PDF upload E2E via Playwright browser automation:

1. Navigated to `http://localhost:3001/documents?workspace=zz`
2. Uploaded PDF `001-BEYONG-TRANFORMER-OUTLINE-V1_1.pdf` via drag-and-drop area
3. Frontend showed: "Processing 1 document(s)" ✓
4. Backend task created and completed successfully ✓
5. Documents list shows 0 documents ✗

## Evidence Gathered

### Task Status (SUCCESS)

```json
{
  "track_id": "pdf-f9027ceb-c17e-4faf-9661-6fdbe98e33b5",
  "status": "indexed",
  "task_type": "pdf_processing",
  "workspace_id": "cd284095-67f8-47b2-a85c-e2f4f4fbb532",
  "tenant_id": "7a1e4dca-ffe5-44a9-92ca-bf737acbed00"
}
```

### PDF Storage (SUCCESS)

```json
{
  "pdf_id": "8866e3c3-bbd6-4384-b86f-215c9844914d",
  "filename": "001-BEYONG-TRANFORMER-OUTLINE-V1_1.pdf",
  "status": "completed",
  "file_size_bytes": 355667,
  "processed_at": "2026-02-01T01:03:08.959887+00:00"
}
```

### Document Metadata (BUG FOUND)

```json
{
  "id": "001-BEYONG-TRANFORMER-OUTLINE-V1_1.pdf",
  "status": "completed",
  "entity_count": 10,
  "relationship_count": 6,
  "llm_provider": "openai",
  "llm_model": "gpt-4o-mini",
  "source_type": "pdf"
  // MISSING: "tenant_id"
  // MISSING: "workspace_id"
}
```

### Root Cause Analysis

Location: `edgequake-api/src/processor.rs`

**Bug 1**: Lines 1612-1625 (`process_pdf_processing`)

```rust
let text_data = edgequake_tasks::TextInsertData {
    workspace_id: data.workspace_id.to_string(),  // Set in struct
    metadata: Some(json!({
        "source": "pdf_upload",
        "source_type": "pdf",
        // MISSING: "tenant_id" and "workspace_id" in metadata JSON
    })),
};
```

**Bug 2**: Lines 1260-1275 (`ensure_document_source_type`)

- When creating NEW metadata for PDFs, tenant_id/workspace_id are not set

**Bug 3**: Lines 1188-1200 (`update_document_status`)

- When creating NEW metadata, tenant_id/workspace_id are not set

### Impact

- PDFs are processed successfully
- Entities and embeddings are stored WITH workspace_id ✓
- Document metadata stored WITHOUT workspace_id ✗
- Document list queries filter by workspace_id → 0 results

### Flow Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│ PDF Upload Flow - Workspace Context Propagation                 │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│ Frontend uploads PDF with:                                      │
│   X-Workspace-ID: cd284095-67f8-47b2-a85c-e2f4f4fbb532        │
│   X-Tenant-ID: 7a1e4dca-ffe5-44a9-92ca-bf737acbed00           │
│                          │                                      │
│                          ▼                                      │
│ ┌───────────────────────────────────────┐                      │
│ │ PDF Upload Handler (pdf_upload.rs)    │                      │
│ │ • Creates task with workspace_id      │ ✓ CORRECT            │
│ │ • Queues PdfProcessingData            │                      │
│ └───────────────────────────────────────┘                      │
│                          │                                      │
│                          ▼                                      │
│ ┌───────────────────────────────────────┐                      │
│ │ process_pdf_processing (processor.rs) │                      │
│ │ • Extracts markdown from PDF          │ ✓ CORRECT            │
│ │ • Creates TextInsertData              │                      │
│ │   • workspace_id in struct field      │ ✓                    │
│ │   • metadata JSON missing workspace   │ ✗ BUG                │
│ └───────────────────────────────────────┘                      │
│                          │                                      │
│                          ▼                                      │
│ ┌───────────────────────────────────────┐                      │
│ │ ensure_document_source_type           │                      │
│ │ • Creates doc metadata if not exists  │                      │
│ │ • Missing: tenant_id, workspace_id    │ ✗ BUG                │
│ └───────────────────────────────────────┘                      │
│                          │                                      │
│                          ▼                                      │
│ ┌───────────────────────────────────────┐                      │
│ │ Document List Query                   │                      │
│ │ • Filters by workspace_id             │                      │
│ │ • No match → 0 documents              │ ✗ SYMPTOM            │
│ └───────────────────────────────────────┘                      │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Files to Modify

1. `edgequake-api/src/processor.rs`
   - Line 1612-1625: Add tenant_id/workspace_id to metadata JSON
   - Line 1260-1275: Add tenant_id/workspace_id when creating new metadata
   - Line 1188-1200: Add tenant_id/workspace_id when creating new metadata

2. Consider: Should all status update methods accept tenant/workspace context?
