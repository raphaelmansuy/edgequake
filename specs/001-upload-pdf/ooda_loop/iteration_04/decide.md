# Iteration 04: Decide

## Decision

Add `extract_to_markdown_with_progress()` method to `PdfExtractor` that:
1. Accepts `Arc<dyn ProgressCallback>`
2. Calls `backend.extract_with_progress(pdf_bytes, callback)`
3. Applies processors (no progress for now)
4. Renders to Markdown

## Rationale

- Backend already supports progress (OODA-03)
- PdfExtractor is the public API that callers use
- Need to wire progress through to complete the chain

## Action Items

1. [x] Add import for `ProgressCallback` in extractor.rs
2. [x] Add `extract_document_with_progress()` internal method
3. [x] Add `extract_to_markdown_with_progress()` public method
4. [x] Add integration test with CountingProgress
5. [x] Update module documentation

## Success Metrics

- [x] New method compiles
- [x] Test verifies callbacks are invoked
- [x] Existing tests still pass

## Testing Strategy

- Use CountingProgress to verify callbacks reach backend
- Test with same PDFs as OODA-03
