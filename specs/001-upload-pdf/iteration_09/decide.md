# OODA-09: Decide

## Decision

Wire `PipelineProgressCallback` into `process_pdf_processing()` by:

1. Creating callback once before the vision/text extraction branches
2. Replacing `extract_to_markdown()` with `extract_to_markdown_with_progress()` in all paths
3. Adding necessary imports

## Rationale

Using first principles:

1. **Single callback creation**: Avoids duplication, callback is thread-safe
2. **Same method signature**: with_progress takes same inputs plus callback
3. **Backward compatible**: If callback isn't consumed, no harm done
4. **Visible progress**: Users finally see page-by-page extraction

## Action Items

1. [ ] Add imports for `PipelineProgressCallback` at top of processor.rs
   - File: `edgequake-api/src/processor.rs`
   - Est: 2 min

2. [ ] Create callback before extraction block
   - File: `edgequake-api/src/processor.rs` (around line 1280)
   - Est: 3 min

3. [ ] Replace `extract_to_markdown()` with `extract_to_markdown_with_progress()`
   - File: `edgequake-api/src/processor.rs` (3 locations)
   - Est: 10 min

4. [ ] Run tests to verify no breakage
   - Est: 5 min

## Success Metrics

- [ ] Builds without errors
- [ ] All existing PDF processing tests pass
- [ ] Progress callback is invoked (visible in logs if using LoggingProgress)

## Testing Strategy

- Unit tests: existing tests still pass
- Integration tests: next iteration (OODA-10) will add specific test
