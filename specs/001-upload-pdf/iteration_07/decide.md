# OODA-07: Decide

## Decision

We will add `PdfPageProgress` event to `PipelineEvent` enum and `emit_pdf_page_progress()` method to `PipelineState`. This keeps the event system consistent with existing ChunkProgress events.

## Rationale

Using first principles:

1. **Consistency**: ChunkProgress and PdfPageProgress are both pipeline events
2. **Single Source of Truth**: PipelineState already handles broadcast channel
3. **Testability**: PipelineState has existing test patterns
4. **Future-proof**: Other callers can emit PDF progress events

## Action Items

1. [x] Add `PdfPageProgress` variant to `PipelineEvent` enum
   - File: `edgequake-tasks/src/pipeline_state.rs`
   - Est: 5 min

2. [x] Add `emit_pdf_page_progress()` method to `PipelineState`
   - File: `edgequake-tasks/src/pipeline_state.rs`
   - Est: 5 min

3. [ ] Add unit test for emit_pdf_page_progress
   - File: `edgequake-tasks/src/pipeline_state.rs`
   - Est: 5 min

## Success Metrics

- [x] `PdfPageProgress` event serializes correctly with serde
- [x] `emit_pdf_page_progress()` method compiles
- [ ] Unit test passes
- [ ] edgequake-tasks crate builds without errors

## Testing Strategy

- Unit tests: test `emit_pdf_page_progress` method
- Integration tests: verify event received by subscriber (next iteration)
