# OODA Iteration 21 — Orient

## Analysis

### Why This Bug Exists

The pdfium backend was added in OODA-43 to bring accurate font style detection (bold/italic from font descriptors). However, the implementation overlooked that all downstream processors expect **normalized document coordinates** (Y=0 at top), not raw PDF coordinates (Y=0 at bottom).

The lopdf backend (extraction_engine.rs) has explicit Y normalization (line ~462):

```rust
// Normal PDF coordinate system: lower Y = bottom of page
// To convert to document order (Y=0 at top), we flip: normalized_y = max_y - y
e.y = max_y - e.y;
```

This normalization was never added to the pdfium backend.

### Why TextGrouper Masks the Bug

The TextGrouper in `pymupdf_grouper.rs` uses its own coordinate-aware sorts:

- `chars_to_spans()`: sorts by `y0` descending (handles raw PDF coords)
- `group_lines_simple()`: sorts by `y1` descending
- `sort_blocks_reading_order()`: uses `y_inverted = -(block.y1)` for ascending sort

These all correctly handle raw PDF coordinates. The blocks EXIT the TextGrouper in correct reading order. But then `LayoutProcessor.process()` uses `ReadingOrderDetector.single_column_order()` which sorts by ascending Y — and that's where the reversal happens.

### Design Decision

Normalize Y coordinates at the **schema::Block conversion boundary** in `pdfium_backend.rs`:

- TextGrouper continues to work with raw PDF coords (its internal sorts handle them correctly)
- All schema::Block objects get normalized coords (Y=0 at top, consistent with lopdf path)
- No changes needed to any processor, renderer, or reading order detector

This is the same strategy as the lopdf backend: normalize at the boundary between extraction and processing.

### Required Formula

For each coordinate:

```
normalized_y = page_height - pdf_y
```

For bounding boxes, swap y1/y2 to maintain the y1 < y2 invariant:

```
new_y1 = page_height - old_y1  (old top → new top, small value)
new_y2 = page_height - old_y0  (old bottom → new bottom, large value)
```

### Risk Assessment

- **Low risk**: Only changes pdfium_backend.rs conversion functions
- **No test breakage**: All 569 tests use lopdf path (no pdfium library in CI)
- **Backward compatible**: lopdf path unchanged
