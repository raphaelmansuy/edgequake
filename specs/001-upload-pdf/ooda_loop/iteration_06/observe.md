# Iteration 06: Observe

## Mission Re-Read ✅

- [x] Re-read `/Users/raphaelmansuy/Github/03-working/edgequake/specs/001-upload-pdf.md`
- [x] Confirmed objectives: Real-time WebSocket updates < 500ms latency
- [x] Current phase: Phase 1 - Architecture & Design (Iterations 1-10)
- [x] Previous iterations:
  - OODA-01: Added PipelinePhase, PhaseProgress, PdfUploadProgress types
  - OODA-02: Added ProgressCallback trait
  - OODA-03: Integrated ProgressCallback into ExtractionEngine
  - OODA-04: Added extract_to_markdown_with_progress() to PdfExtractor
  - OODA-05: Verified exports (already done in OODA-02)

## Code Analysis

### WebSocket Infrastructure (Already Exists!)

**File**: `edgequake/crates/edgequake-api/src/handlers/websocket.rs`

- WebSocket endpoint at `/ws/pipeline/progress`
- Supports various event types (JobStarted, DocumentProgress, etc.)
- Uses broadcast channel for distributing events

**File**: `edgequake/crates/edgequake-api/src/handlers/websocket_types.rs`

- `ProgressEvent` enum with multiple variants
- `ProgressBroadcaster` for broadcasting events
- Missing: **PdfPageProgress** event for page-level extraction

### PDF Processing Location

**File**: `edgequake/crates/edgequake-api/src/processor.rs`

- Lines 1335, 1355, 1369: `extract_to_markdown(&pdf.pdf_data)`
- **Missing**: Should call `extract_to_markdown_with_progress()` with a callback

### Integration Points

1. **Add PdfPageProgress event** to `ProgressEvent` enum
2. **Create ProgressCallback implementation** that broadcasts events
3. **Modify processor.rs** to use `extract_to_markdown_with_progress()`

## Architecture

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                    WEBSOCKET PROGRESS FLOW                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  WebSocket Client                                                           │
│    ▲                                                                        │
│    │ ProgressEvent::PdfPageProgress { pdf_id, page, total, phase }         │
│    │                                                                        │
│  ProgressBroadcaster (tokio::broadcast)                                    │
│    ▲                                                                        │
│    │ on_page_complete(page, md_len)                                        │
│    │                                                                        │
│  BroadcastingProgressCallback (implements ProgressCallback)                │
│    ▲                                                                        │
│    │ extract_to_markdown_with_progress(pdf_bytes, callback)                │
│    │                                                                        │
│  DocumentTaskProcessor (processor.rs)                                      │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Current ProgressEvent Variants

```rust
pub enum ProgressEvent {
    JobStarted { ... },
    DocumentProgress { ... },        // Generic document progress
    DocumentFailed { ... },
    BatchCompleted { ... },
    JobFinished { ... },
    Message { ... },
    StatusSnapshot { ... },
    Heartbeat { ... },
    Connected { ... },
    CancellationRequested,
    ChunkFailure { ... },
    // MISSING: PdfPageProgress { pdf_id, page, total, phase, ... }
}
```

## Questions

1. Should PdfPageProgress be a separate event or extend DocumentProgress?
   → Separate event for clarity
2. How to connect ProgressCallback to ProgressBroadcaster?
   → Create BroadcastingProgressCallback struct
3. Thread safety?
   → ProgressBroadcaster uses broadcast channel (already thread-safe)
