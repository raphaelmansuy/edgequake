# Iteration 11: Observe

## Mission Re-Read ✅

- [x] Re-read `/Users/raphaelmansuy/Github/03-working/edgequake/specs/001-upload-pdf.md`
- [x] Confirmed objectives: 6-phase pipeline, edgequake-pdf first, real-time UI
- [x] Current phase: Phase 2 - Backend Implementation (Iterations 11-25)

## Phase 2 Tasks Review

From mission file:

- [ ] observe.md: PDF worker task handler code review
- [ ] orient.md: Progress callback injection points
- [ ] decide.md: Progress update event schema
- [x] act.md: Instrument PDF extractor with progress callbacks (DONE: OODA-02, OODA-03, OODA-04)
- [ ] act.md: Instrument vision processor with page-level progress
- [ ] act.md: Add progress persistence to task storage
- [ ] act.md: Implement GET /api/v1/documents/pdf/:id/progress endpoint
- [ ] act.md: Add WebSocket /ws/progress/:track_id endpoint
- [ ] act.md: Add error recovery endpoints (retry, cancel)

## OODA 1-10 Summary (What We Built)

### Phase 1 Architecture Complete:

1. ✅ PipelinePhase, PhaseProgress, PdfUploadProgress types
2. ✅ ProgressCallback trait in edgequake-pdf
3. ✅ ExtractionEngine integration with progress
4. ✅ PdfExtractor.extract_to_markdown_with_progress()
5. ✅ Public exports verified
6. ✅ ProgressEvent::PdfPageProgress for WebSocket
7. ✅ PipelineEvent::PdfPageProgress for internal events
8. ✅ PipelineProgressCallback adapter (ProgressCallback → events)
9. ✅ Processor wiring with callback
10. ✅ Dual event system bridge (PipelineState + ProgressBroadcaster)

## Current Gap Analysis

### What Works:

- PDF extraction with progress callbacks fires events
- Events flow to both PipelineState and ProgressBroadcaster
- WebSocket handler can subscribe to ProgressBroadcaster

### What's Missing for Phase 2:

1. **Vision Processor Instrumentation** - Need page-level progress in VisionExtractor
2. **Progress Persistence** - Events are ephemeral, need storage for GET endpoint
3. **GET Endpoint** - `/api/v1/documents/pdf/:id/progress` not implemented
4. **Filtered WebSocket** - `/ws/progress/:track_id` doesn't filter by track_id
5. **Error Recovery** - Retry/cancel endpoints not implemented

## Code Analysis: Vision Processor

Let me examine the VisionExtractor to understand instrumentation needs.

### File: `edgequake-pdf/src/vision.rs`

```
Current: VisionExtractor.extract_from_pdf() -> Result<Document>
Missing: No progress callback integration
```

The VisionExtractor needs:

- `extract_from_pdf_with_progress()` method
- Callbacks: on_page_start, on_page_complete, on_page_error

## Questions for Next Steps

1. Does VisionExtractor process pages sequentially or in parallel?
2. How to share progress callback across async page processing?
3. Should progress be persisted to DB or kept in-memory?
4. What filtering strategy for WebSocket (track_id vs pdf_id)?

## Data Gathered

1. VisionExtractor exists in vision.rs but lacks progress callbacks
2. WebSocket endpoint exists at `/ws/pipeline/progress` but doesn't filter
3. No persistence layer for progress events
4. Retry/cancel not implemented

## Next Iteration Focus

OODA-11 should focus on **Vision Processor Instrumentation** since:

- Text extraction already has progress callbacks (OODA-04)
- Vision mode is the alternate path that also needs progress
- This completes the extraction layer instrumentation
