# IT31 — Orient: Dead Code Impact Analysis

## Why Remove Now?

1. **Maintenance burden**: 13,602 lines of untested, unused code
2. **Feature flag confusion**: `default = ["pdfium", "lopdf"]` suggests both are needed
3. **Build time**: Compiling lopdf adds unnecessary build overhead
4. **Developer confusion**: New contributors can't tell which pipeline is active
5. **Mission directive**: Spec explicitly requires lopdf removal

## Risk Assessment

- **Low risk**: lopdf modules are feature-gated — removing them doesn't affect pdfium pipeline
- **Pre-existing test failures**: `test_extract_full` and `test_extract_text` integration tests fail because `sample.pdf` doesn't extract with PdfiumBackend (not our regression)
- **No external consumers**: No other crates in workspace reference lopdf types

## Dependencies to Track

- `Cargo.toml`: Remove `lopdf` dependency and feature
- `backend/mod.rs`: Remove all `#[cfg(feature = "lopdf")]` module declarations
- `lib.rs`: Remove `image_extraction` module and lopdf-gated re-exports
- `extractor.rs`: Simplify backend selection (remove lopdf fallback)
- `processors/mod.rs`: Remove `image_processor` module and re-exports
- Debug bins: All 10 files in `src/bin/` depend on lopdf
- Examples: 7 files reference lopdf types

## Files to Keep

- `elements.rs`: Shared `RawChar` struct used by pdfium
- `spatial.rs`: Shared spatial indexing used by layout module
- `mock.rs`: Test backend
- `pdfium.rs`, `pdfium_backend.rs`: Production backend
