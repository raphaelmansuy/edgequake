# OODA Iteration 21 — Decide

## Decision: Normalize Y coordinates in pdfium_backend.rs

### Changes Required

1. **`src/backend/pdfium.rs`** — Add `extract_chars_and_page_sizes_from_bytes()` method
   - Returns `(Vec<RawChar>, Vec<(f32, f32)>)` — chars plus per-page (width, height)
   - WHY: Need actual page heights for accurate Y normalization (not hardcoded 792.0)

2. **`src/backend/pdfium_backend.rs`** — Three changes:
   - `extract()`: Use new method, pass `page_height` to block conversion
   - `extract_with_progress()`: Same changes for progress-aware path
   - `convert_text_block_to_schema_block()`: Add `page_height` parameter, flip Y coords
   - `convert_span_to_text_span()`: Add `page_height` parameter, flip span Y coords
   - Update tests to pass `page_height=792.0`

### What NOT to Change

- `pymupdf_grouper.rs` — TextGrouper sorts correctly handle raw PDF coords
- `reading_order.rs` — Already correct for normalized coords
- `extraction_engine.rs` — lopdf path already works

### Verification

1. All 569 lib tests must pass
2. CLI conversion of `AI_Services_Elitizon.pdf` with pdfium must show title first
3. `cargo clippy` must pass
