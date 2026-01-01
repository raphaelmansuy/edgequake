# Next Steps

## Immediate Actions

- [ ] **Fix Binary Compilation**: Investigate why `src/bin.rs` cannot resolve `edgequake_pdf`. It might require `cargo build` instead of `check`, or ensuring the library name is correct.
- [ ] **Address Warnings**: Fix unused imports and fields in `PdfExtractor`.
- [ ] **Unit Tests**: Add tests in `src/extractor.rs` that use `MockBackend` to verify the pipeline flow.

## Future Improvements

- [ ] **Pipeline Configuration**: Make `ProcessorChain` configurable via `PdfConfig` or a builder pattern on `PdfExtractor`.
- [ ] **Error Handling**: Improve error reporting from backends.
- [ ] **Vision Backend**: Implement a `VisionBackend` that uses the `render_page_to_image` capability (currently in `PdfiumBackend` but not exposed via trait).
  - We might need to add `render_page` to `PdfBackend` trait or create a separate `VisionBackend` trait.

## Testing Strategy

- **Unit Tests**: Use `MockBackend` to test `PdfExtractor` logic.
- **Integration Tests**: Use `PdfiumBackend` (guarded by feature flag) to test real PDF extraction.
- **E2E Tests**: Run the full pipeline on sample PDFs.
