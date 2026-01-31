# Iteration 13: Observe

## Mission Re-Read ✅
- [x] Re-read `/Users/raphaelmansuy/Github/03-working/edgequake/specs/001-upload-pdf.md`
- [x] Confirmed objectives: 6-phase pipeline, edgequake-pdf first, real-time UI
- [x] Current phase: Phase 2 - Backend Implementation (Iterations 11-25)

## Current Gap

We have:
1. ✅ `PipelineProgressCallback` - bridges ProgressCallback → events (OODA-08)
2. ✅ `PipelineState.pdf_progress` - stores PdfUploadProgress (OODA-12)

But they're not connected:
```
PipelineProgressCallback
    │
    ├── emit_pdf_page_progress() → broadcast (ephemeral)
    │
    └── ??? → PipelineState.update_pdf_phase() (persistent)
                    NOT CONNECTED YET!
```

## Solution

Update `PipelineProgressCallback` callback methods to also call:
- `pipeline_state.start_pdf_progress()` in `on_extraction_start()`
- `pipeline_state.update_pdf_phase()` in `on_page_start()` and `on_page_complete()`
- `pipeline_state.complete_pdf_phase()` in `on_extraction_complete()`
- `pipeline_state.fail_pdf_phase()` in `on_page_error()` if fatal

## Challenge: Async Methods in Sync Trait

The `ProgressCallback` trait methods are **synchronous**:
```rust
fn on_page_complete(&self, page_num: usize, markdown_len: usize);
```

But `PipelineState` methods are **async**:
```rust
pub async fn update_pdf_phase(&self, ...);
```

### Solutions:
1. **Option A**: Use `tokio::spawn()` to run async update in background
2. **Option B**: Use `block_on()` to block (NOT recommended in async runtime)
3. **Option C**: Add a channel to queue updates for async processing
4. **Option D**: Make PipelineStateInner use `std::sync::RwLock` for sync access

**Recommended**: Option A - Fire and forget with `tokio::spawn`

## Code Analysis

Current `PipelineProgressCallback.on_page_complete`:
```rust
fn on_page_complete(&self, page_num: usize, markdown_len: usize) {
    let total = self.total_pages.load(Ordering::SeqCst);

    // Emit to PipelineState (broadcast)
    self.pipeline_state.emit_pdf_page_progress(...);

    // OODA-10: Also broadcast to WebSocket clients
    self.broadcast_event(ProgressEvent::PdfPageProgress {...});
}
```

Needs to add:
```rust
    // OODA-13: Also persist to queryable storage
    let state = self.pipeline_state.clone();
    let track_id = self.task_id.clone();
    tokio::spawn(async move {
        state.update_pdf_phase(
            &track_id,
            PipelinePhase::PdfConversion,
            page_num,
            &format!("Extracting page {} of {}...", page_num, total),
        ).await;
    });
```

## Data Gathered

1. ProgressCallback trait is sync (can't be changed without breaking)
2. PipelineState uses tokio RwLock (requires async)
3. Need async bridge via spawn
4. track_id is already available in PipelineProgressCallback

## Questions

1. Should errors in spawn be logged? (Yes, but fire-and-forget is OK)
2. Need to call start_pdf_progress first? (Yes, in on_extraction_start)
3. What about cleanup? (Should be done by processor after completion)

## Files to Modify

1. `edgequake-api/src/pipeline_progress_callback.rs` - Add spawn calls
2. `edgequake-api/src/processor.rs` - Add cleanup call after processing
