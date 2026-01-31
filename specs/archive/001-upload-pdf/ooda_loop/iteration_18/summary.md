# OODA-18: Phase 2 Backend Implementation Summary

## Mission Re-Read ✅

- [x] Re-read `/Users/raphaelmansuy/Github/03-working/edgequake/specs/001-upload-pdf.md`
- [x] Confirmed objectives: 6 pipeline phases, edgequake-pdf first, real-time UI
- [x] Current phase: Phase 2 Backend Implementation COMPLETE

## Phase 2 Completion Status

All Phase 2 backend implementation objectives have been achieved:

| Objective                                            | Status | OODA    | Commit    |
| ---------------------------------------------------- | ------ | ------- | --------- |
| Instrument PDF extractor with progress callbacks     | ✅     | OODA-04 | (earlier) |
| Instrument vision processor with page-level progress | ✅     | OODA-11 | (earlier) |
| Add progress persistence to task storage             | ✅     | OODA-12 | (earlier) |
| Connect callbacks to persistent storage              | ✅     | OODA-13 | 53910821  |
| GET /documents/pdf/progress/{track_id}               | ✅     | OODA-14 | c77f96ab  |
| WebSocket /ws/progress/{track_id}                    | ✅     | OODA-15 | d364e45b  |
| Wire processor to callbacks                          | ✅     | OODA-16 | a2522f7a  |
| Error recovery endpoints (retry, cancel)             | ✅     | OODA-17 | 3115de7a  |

---

## Backend API Contract

### REST Endpoints

#### PDF Progress Tracking

| Method   | Path                                        | Description                 |
| -------- | ------------------------------------------- | --------------------------- |
| `GET`    | `/api/v1/documents/pdf/progress/{track_id}` | Get current upload progress |
| `POST`   | `/api/v1/documents/pdf/{pdf_id}/retry`      | Retry failed PDF processing |
| `DELETE` | `/api/v1/documents/pdf/{pdf_id}/cancel`     | Cancel in-progress PDF      |

#### WebSocket Endpoints

| Path                      | Description                           |
| ------------------------- | ------------------------------------- |
| `/ws/progress`            | Global progress events (all uploads)  |
| `/ws/progress/{track_id}` | Filtered progress for specific upload |

---

### Data Models

#### PdfUploadProgress (edgequake-tasks/src/progress.rs)

```rust
/// Complete progress state for a PDF upload.
pub struct PdfUploadProgress {
    pub track_id: String,           // Unique upload identifier
    pub pdf_id: String,             // PDF document ID
    pub filename: String,           // Original filename
    pub status: UploadStatus,       // Pending, Processing, Completed, Failed
    pub phases: [PhaseStatus; 6],   // One per PipelinePhase
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

/// Status of a single pipeline phase.
pub enum PhaseStatus {
    Pending,
    Active { current: usize, total: usize, percent: f32 },
    Completed,
    Failed { error: String },
}

/// 6 pipeline phases for PDF processing.
pub enum PipelinePhase {
    Upload,         // File upload & validation
    PdfConversion,  // PDF → Markdown via edgequake-pdf
    Chunking,       // Text splitting
    Embedding,      // Vector generation
    Extraction,     // Entity extraction
    GraphStorage,   // Graph indexing
}
```

#### PdfOperationResponse (edgequake-api/src/handlers/pdf_upload.rs)

```rust
/// Response for retry/cancel operations.
pub struct PdfOperationResponse {
    pub success: bool,
    pub pdf_id: String,
    pub message: String,
    pub task_id: Option<String>,  // Present for retry, absent for cancel
}
```

---

### WebSocket Protocol

#### Connection Flow

```
Client                           Server
  |                                 |
  |-- GET /ws/progress/{track_id} -->|
  |<-- 101 Switching Protocols ------|
  |                                 |
  |<-- ProgressSnapshot -------------|  // Initial state on connect
  |                                 |
  |<-- PdfPageProgress --------------|  // Per-page updates
  |<-- PdfPageProgress --------------|
  |                                 |
  |<-- ProgressEvent -----------------|  // Completion/error
  |                                 |
```

#### Event Types

```rust
/// Events sent over WebSocket
pub enum ProgressEvent {
    /// Progress snapshot on connection
    ProgressSnapshot {
        pdf_progress: HashMap<String, PdfUploadProgress>,
    },

    /// Per-page progress update
    PdfPageProgress {
        task_id: String,    // track_id
        page: usize,
        total_pages: usize,
        phase: String,      // "pdf_conversion"
        message: String,
    },

    /// Chunk processing failure
    ChunkFailure {
        task_id: String,
        chunk_id: String,
        error: String,
    },
}
```

---

### Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           FRONTEND                                       │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐                  │
│  │  React Query │  │  WebSocket   │  │   Progress   │                  │
│  │    Hooks     │  │    Hook      │  │   Components │                  │
│  └──────┬───────┘  └──────┬───────┘  └──────────────┘                  │
└─────────┼─────────────────┼──────────────────────────────────────────────┘
          │                 │
          ▼                 ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         edgequake-api                                   │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │                         Routes                                     │  │
│  │  GET /progress/{track_id}  WS /progress/{track_id}               │  │
│  │  POST /{pdf_id}/retry      DELETE /{pdf_id}/cancel               │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                                                                         │
│  ┌──────────────────┐  ┌──────────────────┐  ┌────────────────────┐   │
│  │  PipelineState   │  │ ProgressBroadcaster│ │ PipelineProgress  │   │
│  │  (persistence)   │  │ (WebSocket events) │ │ Callback          │   │
│  └────────┬─────────┘  └────────┬───────────┘ └────────┬──────────┘   │
│           │                     │                      │              │
│           ▼                     ▼                      │              │
│  ┌──────────────────────────────────────────────────────┼───────────┐  │
│  │                    Processor (PDF Task Handler)     ◄┘           │  │
│  │  1. with_filename() - sets callback filename                      │  │
│  │  2. Callbacks fire: on_extraction_start, on_page_complete, etc.   │  │
│  │  3. remove_pdf_progress() - cleanup after completion              │  │
│  └──────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         edgequake-pdf                                   │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │  PdfExtractor with ProgressCallback trait                        │  │
│  │  - on_extraction_start(total_pages)                              │  │
│  │  - on_page_complete(page_num, markdown)                          │  │
│  │  - on_extraction_complete()                                       │  │
│  │  - on_page_error(page_num, error)                                │  │
│  └──────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────┘
```

---

### Event Flow: PDF Upload to Completion

```
1. User uploads PDF
   └─> POST /api/v1/documents/pdf
       └─> Returns {track_id, pdf_id, task_id}

2. Client connects WebSocket
   └─> WS /ws/progress/{track_id}
       └─> Server sends ProgressSnapshot

3. Worker picks up task
   └─> PipelineProgressCallback.on_extraction_start()
       └─> PipelineState.start_pdf_progress()
       └─> ProgressBroadcaster.broadcast(PdfPageProgress)
       └─> WebSocket client receives event

4. Per-page processing
   └─> PipelineProgressCallback.on_page_complete(page_num)
       └─> PipelineState.update_pdf_phase()
       └─> ProgressBroadcaster.broadcast(PdfPageProgress)
       └─> WebSocket client receives event

5. Completion
   └─> PipelineProgressCallback.on_extraction_complete()
       └─> PipelineState.complete_pdf_phase()
       └─> Processor removes progress: remove_pdf_progress()
       └─> Client WebSocket closes or receives final event

6. Error handling
   └─> on_page_error(page_num, error)
       └─> PipelineState.fail_pdf_phase()
       └─> User can POST /{pdf_id}/retry

7. Cancellation
   └─> DELETE /{pdf_id}/cancel
       └─> PipelineState.request_cancellation()
       └─> Worker checks is_cancellation_requested()
       └─> Status set to Failed with "Cancelled by user"
```

---

### Test Coverage

| Component                | Tests       | Status |
| ------------------------ | ----------- | ------ |
| PipelineProgressCallback | 7           | ✅     |
| PdfOperationResponse     | 1           | ✅     |
| progress.rs types        | 6           | ✅     |
| WebSocket handlers       | Integration | ✅     |

Total edgequake-api tests: **436 passing**

---

## Next Steps: Phase 3 Frontend Integration (OODA-26+)

1. **React Query Hooks Analysis** (OODA-26)
   - Review existing `use-ingestion-progress.ts`
   - Plan WebSocket hook integration

2. **PdfUploadProgress Component** (OODA-27-30)
   - 6-phase timeline display
   - Real-time percentage updates
   - Error banners with retry button

3. **WebSocket Hook with Reconnection** (OODA-31-33)
   - Auto-reconnect on disconnect
   - Fallback to polling
   - Connection status indicator

4. **Document Manager Integration** (OODA-34-36)
   - Replace generic "Processing..." status
   - Add upload history table
   - Filter/search capabilities

5. **Testing & Polish** (OODA-37-40)
   - Playwright E2E tests
   - Edge case handling
   - Performance optimization

---

## Commits This Session

| Commit     | Description                                                 |
| ---------- | ----------------------------------------------------------- |
| `53910821` | OODA-13: Connect callbacks to persistent progress storage   |
| `c77f96ab` | OODA-14: GET /documents/pdf/progress/{track_id}             |
| `d364e45b` | OODA-15: WebSocket /ws/progress/{track_id}                  |
| `a2522f7a` | OODA-16: Wire processor to callback with filename + cleanup |
| `3115de7a` | OODA-17: Error recovery endpoints (retry, cancel)           |
