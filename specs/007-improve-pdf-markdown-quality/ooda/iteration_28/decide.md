# OODA IT28 — Decide

## Actions

1. **Add vertical overlap check to `can_add_span()`** — `pymupdf_structs.rs`
   - After baseline/top tolerance check fails, check if span is ≥80% vertically contained within line's y-range
   - This catches em dashes, bullets, dots, and other narrow-height glyphs

2. **Preserve em/en dashes in OCR text normalization** — `text_cleanup.rs`  
   - Change `("\u{2014}", "-")` to `("\u{2014}", "—")` (identity mapping, documenting we considered it)
   - Same for en dash: `("\u{2013}", "–")`

3. **Remove em dash from hyphen-no-space rules** — two locations:
   - `Line::text()` in `pymupdf_structs.rs`: Remove `|| span.text.starts_with('—')` and `|| prev.text.ends_with('—')`
   - `convert_text_block_to_schema_block()` in `pdfium_backend.rs`: Same removal

4. **Also implemented**: `merge_same_line_blocks()` function in `pdfium_backend.rs` for schema::Block-level horizontal merging (uses block height as font-size proxy since Block has no font_size field)

## Success Criteria
- Title renders as `# AI Services — Elitizon`
- 569 tests pass
- No regressions in other content
