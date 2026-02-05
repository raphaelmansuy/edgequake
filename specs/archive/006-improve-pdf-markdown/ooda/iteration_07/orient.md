# Iteration 07: Orient

**Mission**: Improve PDF to Markdown Extraction Pipeline  
**Mission File**: `specs/006-improve-pdf-markdown.md`

## Analysis: Integration Test Location

### Where to Add Test

The best location is `src/layout/pymupdf_structs.rs` tests section, extending the existing style tests.

### Test Design

Create a sequence of RawChars with mixed styles and verify:

1. `chars_to_spans()` creates separate spans per style
2. Each span has correct style flags

### Alternative: Test with Real PDF

Could also add a test PDF with mixed styles to `test-data/gold/02-formatting/`.
However, unit tests are faster and more deterministic.
