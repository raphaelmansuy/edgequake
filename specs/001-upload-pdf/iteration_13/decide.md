# Iteration 13: Decide

## Decision

We will add `tokio::spawn()` calls in `PipelineProgressCallback` to persist progress to `PipelineState.pdf_progress` HashMap, enabling queryable progress via GET endpoint.

## Rationale

1. **Sync-to-async bridge**: `ProgressCallback` trait is sync; `PipelineState` methods are async. `tokio::spawn()` is the idiomatic Rust pattern for this.

2. **Fire-and-forget is OK**: Progress persistence is best-effort. If a spawn fails, the broadcast events still work. User experience is degraded but not broken.

3. **Minimal changes**: No trait changes, no major refactors. Just add spawn calls in existing callback methods.

## Action Items

1. [x] Add `filename: String` field to `PipelineProgressCallback` struct
2. [x] Add `with_filename()` builder method
3. [x] In `on_extraction_start()`: spawn `start_pdf_progress()` and `start_pdf_phase(PdfConversion, total_pages)`
4. [x] In `on_page_complete()`: spawn `update_pdf_phase(PdfConversion, page_num, message)`
5. [x] In `on_extraction_complete()`: spawn `complete_pdf_phase(PdfConversion)`
6. [x] In `on_page_error()`: spawn `update_pdf_phase()` with error message
7. [x] Add tests for the spawn behavior (verify progress is stored)
8. [x] Update `processor.rs` to pass filename to callback

## Success Metrics

- [x] `cargo test --package edgequake-api --lib pipeline_progress_callback` passes
- [x] After callback fires, `PipelineState.get_pdf_progress(track_id)` returns data
- [x] No clippy warnings

## Testing Strategy

- **Unit tests**: Test that `start_pdf_progress`, `update_pdf_phase`, `complete_pdf_phase` are called by verifying `get_pdf_progress()` returns expected data
- **Need async wait**: Since spawn is fire-and-forget, tests need `tokio::time::sleep` to let spawned tasks complete
- **Verification**: `assert!(progress.phases[PdfConversion].current > 0)`

## Code Template

```rust
fn on_extraction_start(&self, total_pages: usize) {
    // ... existing emit calls ...

    // OODA-13: Persist to queryable storage
    let state = self.pipeline_state.clone();
    let track_id = self.task_id.clone();
    let pdf_id = self.pdf_id.clone();
    let filename = self.filename.clone();
    let pages = total_pages;
    tokio::spawn(async move {
        state.start_pdf_progress(&track_id, &pdf_id, &filename).await;
        state.start_pdf_phase(&track_id, PipelinePhase::PdfConversion, pages).await;
    });
}
```
