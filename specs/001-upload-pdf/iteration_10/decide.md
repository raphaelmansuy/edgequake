# OODA-10: Decide

## Decision

Modify `PipelineProgressCallback` to also send `ProgressEvent::PdfPageProgress` to `ProgressBroadcaster` for WebSocket delivery.

## Updated Approach

Instead of only sending to `PipelineState`, the callback will:

1. Send to `PipelineState.emit_pdf_page_progress()` (for internal pipeline coordination)
2. Also send to `ProgressBroadcaster.send()` (for WebSocket clients)

This requires adding `ProgressBroadcaster` as an optional field.

## Rationale

Using first principles:

1. **Dual purpose**: PipelineState for internal use, ProgressBroadcaster for frontend
2. **Minimal change**: Just add a field to existing adapter
3. **No new async tasks**: Direct send in callback methods
4. **Backward compatible**: ProgressBroadcaster is optional

## Action Items

1. [ ] Add `progress_broadcaster: Option<ProgressBroadcaster>` to PipelineProgressCallback
   - File: `edgequake-api/src/pipeline_progress_callback.rs`
   - Est: 5 min

2. [ ] In each callback method, also send ProgressEvent::PdfPageProgress
   - File: `edgequake-api/src/pipeline_progress_callback.rs`
   - Est: 10 min

3. [ ] Update processor.rs to pass progress_broadcaster to callback
   - File: `edgequake-api/src/processor.rs`
   - Est: 5 min

4. [ ] Add processor field for progress_broadcaster
   - File: `edgequake-api/src/processor.rs`
   - Est: 5 min

## Success Metrics

- [ ] Builds without errors
- [ ] WebSocket test receives PdfPageProgress events
- [ ] Existing tests still pass

## Testing Strategy

- Unit tests: Verify both event systems receive events
- Integration tests: WebSocket client receives PDF progress (next iteration)
