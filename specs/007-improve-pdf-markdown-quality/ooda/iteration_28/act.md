# OODA IT28 — Act

## Changes Made

### 1. `pymupdf_structs.rs` — `can_add_span()` vertical overlap
Added fallback check: if span's y-range is ≥80% contained within line's y-range, add it to the line.
This handles em dashes, bullets, and other narrow-height glyphs.

### 2. `pymupdf_structs.rs` — `Line::text()` em dash space fix
Removed `|| span.text.starts_with('—')` and `|| prev.text.ends_with('—')` from hyphen-no-space checks.
Em dashes are sentence-level punctuation, not word-joining hyphens.

### 3. `text_cleanup.rs` — Preserve em/en dashes
Changed `("\u{2014}", "-")` → `("\u{2014}", "—")` and `("\u{2013}", "-")` → `("\u{2013}", "–")`.

### 4. `pdfium_backend.rs` — Em dash space fix in `convert_text_block_to_schema_block()`
Same removal of em dash from hyphen-no-space checks.

### 5. `pdfium_backend.rs` — `merge_same_line_blocks()` function
New function that merges horizontally adjacent schema::Block objects on the same line.
Uses block height as proxy for font size (Block struct has no font_size field).

## Verification
- 569 tests pass
- Title: `# AI Services — Elitizon` ✅ (was: `# AI Services Elitizon -`)
- Em dash preserved as Unicode character
- Spaces correctly inserted around em dash

## Files Changed
- `src/layout/pymupdf_structs.rs` — can_add_span(), Line::text()
- `src/processors/text_cleanup.rs` — fix_ocr_text()
- `src/backend/pdfium_backend.rs` — merge_same_line_blocks(), convert_text_block_to_schema_block()
