# Observe – OODA-19: Test Coverage Gap Analysis

## Current State

- 454 lib tests pass
- Need to identify gaps in test coverage

## Test Distribution Analysis

Top tested files (>10 tests):

- `schema/geometry.rs` - 29 tests
- `processors/text_cleanup.rs` - 24 tests
- `config.rs` - 24 tests
- `formula/detector.rs` - 21 tests
- `backend/lattice.rs` - 19 tests

Files with thin coverage (<3 tests):

- `backend/pdfium.rs` - 1 test (requires external lib)
- `pipeline/pymupdf_pipeline.rs` - 1 test
- `backend/pdfium_backend.rs` - 2 tests
- `processors/font_analysis.rs` - 2 tests
- `processors/provider.rs` - 2 tests
- `processors/stats.rs` - 2 tests

## Target for Improvement

`processors/font_analysis.rs` with only 2 tests:

- `test_valid_size_range` - basic range check
- `test_median_calculation` - median calculation

## Missing Test Cases

1. Even number of elements (median of 2 middle values)
2. Single element median
3. Large dataset with outliers
4. Empty document font detection
5. Document with only headers (no body text)
