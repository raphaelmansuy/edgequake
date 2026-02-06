# OODA Iteration 21 — Observe

## Context

The pdfium backend produces **completely reversed reading order** when converting PDFs. The title "AI Services" (at top of page 1) appears as block 13 (last), while "Blueprint pack" (at bottom of page 1) appears as block 0 (first).

## Root Cause Analysis

### Coordinate System Mismatch

**PDF coordinate system:** Y=0 at BOTTOM, Y increases UPWARD.

**Document coordinate system (expected by processors):** Y=0 at TOP, Y increases DOWNWARD.

The lopdf backend (extraction_engine.rs, line ~462) normalizes Y coordinates:
```rust
e.y = max_y - e.y;
```

The pdfium backend passes raw PDF coordinates through TextGrouper → schema::Block **without any normalization**.

### Impact Path

1. `PdfiumExtractor.extract_chars_from_page()` returns `y0 = bounds.bottom().value` (smaller in PDF coords = bottom of page)
2. `TextGrouper.group()` correctly handles raw PDF coords (sorts by descending Y internally)
3. `convert_text_block_to_schema_block()` copies raw PDF coords to `schema::Block.bbox`
4. `LayoutProcessor.process()` runs `ReadingOrderDetector.single_column_order()` which sorts by **ascending** Y (assumes Y=0 at top)
5. Result: content at bottom of PDF page (small Y) sorts first → **reversed reading order**

### Debug Evidence

**BEFORE processing (correct order from TextGrouper):**
```
block 0 bbox=[72,636,234,659]: 'AI Services'        ← title at top (large Y)
block 3 bbox=[72,587,212,602]: 'Executive summary'   ← below title
```

**AFTER processing (LayoutProcessor reversed):**
```
block 0 (Paragraph): 'Blueprint pack...'             ← bottom content now first
block 13 (SectionHeader): 'AI Services'              ← title now last
```

## Affected Files

- `src/backend/pdfium.rs` — extracts raw PDF coordinates (correct behavior)
- `src/backend/pdfium_backend.rs` — missing Y normalization (the bug)
- `src/layout/reading_order.rs` — `single_column_order()` assumes normalized coords (correct assumption)
- `src/backend/extraction_engine.rs` — lopdf backend normalizes correctly (reference implementation)
