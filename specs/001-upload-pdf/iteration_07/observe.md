# OODA-07: Observe

## Mission Re-Read ✅
- [x] Re-read `/Users/raphaelmansuy/Github/03-working/edgequake/specs/001-upload-pdf.md`
- [x] Confirmed objectives: 6-phase progress tracking, edgequake-pdf first, real-time WebSocket updates
- [x] Current phase: Phase 1 - Architecture & Design (Iterations 1-10)

## Code Analysis

### File: `edgequake/crates/edgequake-tasks/src/pipeline_state.rs`
- Lines: 1-170
- Purpose: Thread-safe pipeline state management for real-time updates
- Current behavior: Supports ChunkProgress and ChunkFailure events
- Dependencies: tokio broadcast channel, serde Serialize

### File: `edgequake/crates/edgequake-api/src/processor.rs`
- Lines: 1210-1400 (process_pdf_processing)
- Purpose: PDF processing task handler
- Current behavior: Calls `PdfExtractor::extract_to_markdown()` without progress
- Dependencies: `edgequake_pdf::PdfExtractor`, `pipeline_state` for events

## Data Gathered

1. **PipelineEvent enum** in `edgequake-tasks/pipeline_state.rs`:
   - Already has `ChunkProgress` and `ChunkFailure` variants
   - Missing `PdfPageProgress` variant for PDF extraction

2. **PipelineState methods**:
   - `emit_chunk_progress()` - emits ChunkProgress events
   - `emit_chunk_failure()` - emits ChunkFailure events
   - Missing `emit_pdf_page_progress()` method

3. **ProgressBroadcaster** in `edgequake-api/websocket_types.rs`:
   - OODA-06 added `PdfPageProgress` to `ProgressEvent` enum
   - `ProgressBroadcaster` wraps tokio broadcast channel
   - Can be used directly or via `PipelineState`

4. **Integration Gap**:
   - `edgequake-pdf` has `ProgressCallback` trait
   - `edgequake-api` has `ProgressBroadcaster`
   - No adapter bridges them yet

## Questions to Answer Next Iteration
- Should `BroadcastingProgressCallback` live in edgequake-api or edgequake-tasks?
- How to get task_id and pdf_id into the callback closure?
- Should we use `PipelineState.emit_*` or `ProgressBroadcaster.send()` directly?
