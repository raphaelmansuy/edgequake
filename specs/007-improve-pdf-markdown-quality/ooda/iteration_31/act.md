# IT31 — Act: Remove lopdf Legacy Code

## Changes Made

### Files Deleted (22 files, ~13,600 lines)

**Backend modules (13 files):**
- `src/backend/extraction_engine.rs` (1,359 lines)
- `src/backend/content_parser.rs` (712 lines)
- `src/backend/element_processing.rs` (389 lines)
- `src/backend/font_handling.rs` (616 lines)
- `src/backend/glyph_list.rs` (391 lines)
- `src/backend/encodings.rs` (1,371 lines)
- `src/backend/column_detection.rs` (942 lines)
- `src/backend/text_grouping.rs` (1,542 lines)
- `src/backend/truetype_cmap.rs` (201 lines)
- `src/backend/block_builder.rs` (437 lines)
- `src/backend/lattice.rs` (1,711 lines)
- `src/image_extraction.rs` (482 lines)
- `src/processors/image_processor.rs` (307 lines)

**Debug binaries (10 files):**
- `src/bin/count_ops.rs`, `debug_merge.rs`, `debug_page1.rs`, `diagnose_fonts.rs`
- `test_decode.rs`, `trace_content.rs`, `trace_ctm.rs`, `trace_elements.rs`
- `trace_page1.rs`, `trace_y.rs`

**Examples (7 files):**
- `examples/convert_one_tool.rs`, `debug_all_page1.rs`, `debug_page2_xcoords.rs`
- `debug_page_content.rs`, `debug_page_coords.rs`, `debug_tj_kerning.rs`
- `debug_tm_scale.rs`

### Files Modified (6 files)

1. **Cargo.toml**: Removed `lopdf` feature, dependency, and `trace_content` [[bin]]. Default features: `["pdfium"]` only.
2. **src/backend/mod.rs**: Removed all `#[cfg(feature = "lopdf")]` module declarations and `ExtractionEngine` re-export. Simplified trait doc.
3. **src/lib.rs**: Removed `image_extraction` module, lopdf-gated `ImageExtractionProcessor` re-exports.
4. **src/extractor.rs**: Simplified backend selection — PdfiumBackend → MockBackend (no lopdf fallback).
5. **src/processors/mod.rs**: Removed `image_processor` module and `ImageExtractionProcessor` re-export.
6. **src/backend/pdfium_backend.rs**: Updated doc comments (removed ExtractionEngine references).

## Test Results
- **440 lib tests pass** (131 tests removed with lopdf modules)
- **Pre-existing integration test failures**: `test_extract_full`, `test_extract_text` fail with PdfiumBackend on `sample.pdf` — NOT a regression (confirmed by testing on pre-change code)
- **clippy**: No new warnings

## Net Impact
- **12,025 lines removed** (git diff --stat)
- Single extraction pipeline: PdfiumBackend only
- Cleaner feature flags: `default = ["pdfium"]`
