# OODA Iteration 21 — Act

## Changes Made

### 1. `src/backend/pdfium.rs` — New method for page dimensions

Added `extract_chars_and_page_sizes_from_bytes()` that returns characters AND per-page (width, height) from PDFium. This avoids hardcoding US Letter dimensions (612×792) and provides accurate page heights for Y normalization.

### 2. `src/backend/pdfium_backend.rs` — Y-coordinate normalization

**`extract()` and `extract_with_progress()`:**

- Switched from `extract_chars_from_bytes()` to `extract_chars_and_page_sizes_from_bytes()`
- Pass actual `page_height` to block/span conversion functions
- Use actual page dimensions when creating `Page` objects

**`convert_text_block_to_schema_block()`:**

- Added `page_height: f32` parameter
- Normalize Y: `norm_y1 = page_height - text_block.y1` (PDF top → doc top)
- Normalize Y: `norm_y2 = page_height - text_block.y0` (PDF bottom → doc bottom)
- Maintains `y1 < y2` invariant after flip

**`convert_span_to_text_span()`:**

- Added `page_height: f32` parameter
- Same Y normalization formula for span bounding boxes

**Tests:**

- Updated 3 test functions to pass `page_height=792.0`

## Verification Results

### Tests

```
test result: ok. 569 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
```

### Reading Order (Before Fix)

```
AFTER-PAGE1 block 0 (Paragraph): 'Blueprint pack...'     ← WRONG: bottom content first
AFTER-PAGE1 block 13 (SectionHeader): 'AI Services'      ← WRONG: title last
```

### Reading Order (After Fix)

```
AFTER-PAGE1 block 0 (SectionHeader) lvl=1: 'Elitizon'           ← CORRECT: title first
AFTER-PAGE1 block 8 (SectionHeader) lvl=1: 'AI Services'        ← CORRECT: second title
AFTER-PAGE1 block 9 (SectionHeader) lvl=4: 'Executive summary'  ← CORRECT: below title
```

## Root Cause Summary

The pdfium backend passed raw PDF coordinates (Y=0 at bottom) to schema::Block objects, but all downstream processors (LayoutProcessor, ReadingOrderDetector) expect document coordinates (Y=0 at top). The lopdf backend normalizes at extraction time; the pdfium backend was missing this normalization step.
