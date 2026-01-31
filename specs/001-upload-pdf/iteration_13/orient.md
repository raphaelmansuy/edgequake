# Iteration 13: Orient

## Gap Analysis

| Current State | Desired State | Gap | Priority |
|--------------|---------------|-----|----------|
| `PipelineProgressCallback` emits events only | Also persist to `pdf_progress` HashMap | Sync→Async bridge needed | HIGH |
| No initialization of `PdfUploadProgress` on start | Call `start_pdf_progress()` in `on_extraction_start` | Missing call | HIGH |
| No phase update on page progress | Call `update_pdf_phase()` in `on_page_complete` | Missing call | HIGH |
| No cleanup after completion | Call `remove_pdf_progress()` in processor | Missing call | MEDIUM |

## Risk Assessment

- **Risk 1**: Async methods in sync trait callbacks
  - Mitigation: Use `tokio::spawn()` with fire-and-forget pattern
  - Ignored errors are OK (progress is best-effort, not critical path)

- **Risk 2**: Spawned tasks outlive request
  - Mitigation: Progress is stored in `PipelineState` which lives for app lifetime
  - No resource leaks possible

- **Risk 3**: Missing `filename` in `PipelineProgressCallback`
  - Mitigation: Add `filename` field to callback, pass from processor

## First Principles Analysis

- **Core problem**: We have two separate mechanisms - ephemeral events (broadcast) and persistent state (HashMap). Callbacks only emit to broadcast.

- **Fundamental constraint**: `ProgressCallback` trait is sync; `PipelineState` methods are async.

- **Minimal solution**: Use `tokio::spawn()` in each callback method to call async methods. This is the standard pattern in Rust for sync→async bridging.

- **Why this matters**: Without this, the GET /api/v1/documents/pdf/:id/progress endpoint would return empty data.

## Alternative Approaches

1. **Option A: tokio::spawn for each call**
   - Pros: Simple, idiomatic, no trait changes
   - Cons: Many spawns (6 callbacks × N pages = many tasks)
   - Selected: YES

2. **Option B: Channel queue with single consumer**
   - Pros: Fewer spawns, batching possible
   - Cons: More complexity, additional channel type
   - Selected: NO (over-engineered for this use case)

3. **Option C: Make PipelineStateInner use std::sync::RwLock**
   - Pros: Fully synchronous access
   - Cons: Major refactor, blocks tokio runtime if contended
   - Selected: NO (dangerous, architectural change)

## Selected Approach

**Option A: Use `tokio::spawn()` in each callback method**

Changes needed:

1. Add `filename: String` field to `PipelineProgressCallback`
2. Add `with_filename()` builder method (or include in constructor)
3. In `on_extraction_start()`: spawn `start_pdf_progress()` and `start_pdf_phase(PdfConversion)`
4. In `on_page_complete()`: spawn `update_pdf_phase()`
5. In `on_extraction_complete()`: spawn `complete_pdf_phase()`
6. In `on_page_error()`: spawn `fail_pdf_phase()` if total failure

## Data Flow After Changes

```text
┌────────────────────────┐
│     PdfExtractor       │
│ extract_with_progress  │
└───────────┬────────────┘
            │
            ▼
┌────────────────────────────────────────────────────────────┐
│          PipelineProgressCallback                          │
│                                                            │
│  on_page_complete(page, len)                               │
│    ├─► emit_pdf_page_progress() → broadcast (ephemeral)    │
│    ├─► broadcast_event() → WebSocket clients               │
│    └─► tokio::spawn(update_pdf_phase()) → HashMap persist  │
└────────────────────────────────────────────────────────────┘
                                   │
                                   ▼
                    ┌──────────────────────────────┐
                    │ PipelineState.pdf_progress   │
                    │ HashMap<String, PdfUpload>   │
                    │                              │
                    │ Queryable by GET endpoint    │
                    └──────────────────────────────┘
```
