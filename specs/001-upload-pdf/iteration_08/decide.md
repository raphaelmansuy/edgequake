# OODA-08: Decide

## Decision

Create `PipelineProgressCallback` struct in edgequake-api that implements `edgequake_pdf::ProgressCallback` and forwards events to `PipelineState`.

## Rationale

Using first principles:

1. **Separation of Concerns**: Adapter bridges pdf→tasks without coupling
2. **Testability**: Struct can be unit tested with mock PipelineState
3. **Reusability**: Can be used for vision extraction too
4. **Clarity**: Explicit field mapping makes intent clear

## Action Items

1. [ ] Create `pipeline_progress_callback.rs` module in edgequake-api
   - File: `edgequake-api/src/pipeline_progress_callback.rs`
   - Est: 10 min

2. [ ] Implement `ProgressCallback` trait for the struct
   - Map `on_progress(current, total, _)` → `emit_pdf_page_progress()`
   - Est: 10 min

3. [ ] Add module to mod.rs and test
   - File: `edgequake-api/src/lib.rs`
   - Est: 5 min

4. [ ] Add unit test
   - Est: 5 min

## Success Metrics

- [ ] `PipelineProgressCallback` compiles
- [ ] Implements `ProgressCallback` trait
- [ ] Unit test verifies event emission
- [ ] edgequake-api builds without errors

## Testing Strategy

- Unit tests: create callback, call methods, verify events received
- Integration tests: next iteration (OODA-09) will wire into processor
