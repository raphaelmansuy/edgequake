# Iteration 01 - Observe

**Mission File**: `./specs/002-unify-ingestion-pipeline.md`

**Date**: 2026-02-01

---

## Territory Mapping

### Current Architecture Overview

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                           CURRENT INGESTION FLOW                              │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────────┐ │
│  │                         PDF UPLOAD PATH                                  │ │
│  │                                                                         │ │
│  │  POST /api/v1/documents/pdf                                             │ │
│  │         │                                                               │ │
│  │         ▼                                                               │ │
│  │  pdf_upload.rs (handlers/pdf_upload.rs)                                 │ │
│  │         │                                                               │ │
│  │         ▼                                                               │ │
│  │  Store raw PDF in DB (PdfDocumentStorage)                               │ │
│  │         │                                                               │ │
│  │         ▼                                                               │ │
│  │  Create TaskType::PdfProcessing task                                    │ │
│  │         │                                                               │ │
│  │         ▼                                                               │ │
│  │  Background: PDF → Markdown (edgequake-pdf crate)                       │ │
│  │         │                                                               │ │
│  │         ▼                                                               │ │
│  │  Create internal document → run_ingestion_pipeline()                    │ │
│  │                                                                         │ │
│  └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────────┐ │
│  │                      MARKDOWN/TEXT UPLOAD PATH                          │ │
│  │                                                                         │ │
│  │  POST /api/v1/documents                                                 │ │
│  │         │                                                               │ │
│  │         ▼                                                               │ │
│  │  documents.rs (handlers/documents.rs)                                   │ │
│  │         │                                                               │ │
│  │         ├─ Sync mode: run_ingestion_pipeline() directly                 │ │
│  │         │                                                               │ │
│  │         └─ Async mode: Create TaskType::TextInsert task                 │ │
│  │                  │                                                      │ │
│  │                  ▼                                                      │ │
│  │           Background: run_ingestion_pipeline()                          │ │
│  │                                                                         │ │
│  └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

### Key Files Identified

#### Backend (Rust)

| File                                              | Lines | Purpose                                        |
| ------------------------------------------------- | ----- | ---------------------------------------------- |
| `edgequake-api/src/handlers/pdf_upload.rs`        | 1200  | PDF upload, validation, storage, task creation |
| `edgequake-api/src/handlers/documents.rs`         | 4036  | Text/Markdown upload, sync/async processing    |
| `edgequake-api/src/handlers/pipeline.rs`          | 214   | Pipeline status and control                    |
| `edgequake-api/src/pipeline_progress_callback.rs` | 658   | PDF extraction progress → WebSocket            |
| `edgequake-pipeline/src/progress.rs`              | 793   | Pipeline stages and progress tracking          |
| `edgequake-tasks/src/lib.rs`                      | ~500  | Task types: TextInsert, PdfProcessing          |

#### Frontend (TypeScript/React)

| File                                                | Lines | Purpose                                    |
| --------------------------------------------------- | ----- | ------------------------------------------ |
| `components/documents/document-manager.tsx`         | 1571  | Upload UI, document list, progress display |
| `components/documents/status-badge.tsx`             | 222   | Status visualization with icons/colors     |
| `components/documents/ingestion-progress-panel.tsx` | 337   | Real-time ingestion progress               |
| `lib/api/edgequake.ts`                              | 1639  | API client with typed endpoints            |

### Status Tracking Analysis

#### Current Status Types

**Backend PipelineStage (edgequake-pipeline/src/progress.rs:44-60)**:

```rust
pub enum PipelineStage {
    Preprocessing,   // Initial validation
    Chunking,        // Document splitting
    Extracting,      // Entity/relationship extraction
    Gleaning,        // Re-extraction for missed entities
    Merging,         // Graph merge
    Summarizing,     // Description summarization
    Embedding,       // Vector generation
    Storing,         // Persist to storage
    Finalizing,      // Cleanup
}
```

**Frontend statusConfig (status-badge.tsx:37-53)**:

```typescript
const statusConfig = {
  pending: { ... },
  processing: { ... },
  chunking: { ... },
  extracting: { ... },
  embedding: { ... },
  indexing: { ... },
  completed: { ... },
  indexed: { ... },
  failed: { ... },
  cancelled: { ... },
}
```

**OBSERVATION**: Backend has 9 stages, frontend shows only 6 processing states. **GAP identified**.

### PDF-Specific Progress Tracking

**PipelinePhase (edgequake-tasks/src/progress.rs)**:

- `PdfConversion` - PDF → Markdown
- `EntityExtraction` - LLM extraction
- `VectorIndexing` - Embedding storage

**OBSERVATION**: PDF has separate phase enum that doesn't align with PipelineStage.

### Progress Broadcasting

Two parallel systems exist:

1. **PipelineState** (internal) - `edgequake-tasks/src/lib.rs`
   - Holds progress state in memory
   - Provides `get_status()` for REST polling

2. **ProgressBroadcaster** (WebSocket) - `edgequake-api/src/handlers/websocket.rs`
   - Broadcasts events to connected clients
   - Uses `ProgressEvent` enum

**OBSERVATION**: Dual system creates complexity. Need to unify event emission.

### Document Status in Database

**Document metadata stored in KV storage** (documents.rs:509-525):

```rust
let doc_metadata = serde_json::json!({
    "id": document_id,
    "title": request.title,
    "content_summary": content_summary,
    "status": initial_status,  // "pending" or "processing"
    ...
});
```

**PDF processing status** (edgequake-storage/src/traits.rs):

```rust
pub enum PdfProcessingStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}
```

**OBSERVATION**: Two separate status enums - one for documents, one for PDFs. **GAP identified**.

### Frontend Upload Flow

**DocumentManager.tsx upload handling**:

1. For PDF files: calls `uploadPdfDocument(file, options)`
2. For text files: calls `uploadDocument(content, options)`

**uploadPdfDocument** creates FormData, posts to `/api/v1/documents/pdf`
**uploadDocument** posts JSON to `/api/v1/documents`

**OBSERVATION**: Two separate API calls and response handling paths.

---

## Key Observations

### 1. Two Separate Upload Endpoints

- `/api/v1/documents` - Text/Markdown
- `/api/v1/documents/pdf` - PDF only

### 2. Inconsistent Status Tracking

- Backend `PipelineStage` (9 stages)
- Frontend `statusConfig` (6 processing states)
- PDF-specific `PipelinePhase` (3 phases)
- Document `status` field (string, ad-hoc values)

### 3. Progress Events Not Unified

- PDF extraction has dedicated progress callback
- Text ingestion uses generic pipeline progress
- No unified progress structure for both paths

### 4. Error Handling Differs

- PDF errors stored in `errors` JSON field
- Document errors stored in `error_message` field

### 5. Frontend Displays Inconsistent

- PDF shows page-by-page progress
- Text shows generic progress bar
- Different components for each type

---

## Questions to Answer

1. Should PDF and Markdown share a single upload endpoint?
2. How to unify stage naming between backend and frontend?
3. Should error structures be unified?
4. What unified progress events should look like?

---

## Next Steps

- **Orient**: Analyze gaps and propose unified architecture
- **Decide**: Prioritize changes by impact
- **Act**: Implement unified types and status flow
