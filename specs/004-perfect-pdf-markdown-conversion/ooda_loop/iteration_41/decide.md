# OODA-41: Decide - Smart Sort Key Implementation

## Date: 2026-02-04

## Decision

Implement the **Phase 3 smart sort key** algorithm from pymupdf4llm in our `reading_order.rs`:

### Algorithm

For multi-column layouts:

1. For each block in the right column(s), find if there's a block in the left column that overlaps vertically
2. If found, use `(left_block.y, current_block.x)` as the sort key
3. This ensures left column content comes before right column content at the same vertical level

### Changes Required

1. **`reading_order.rs`**: Modify `multi_column_order()` to use smart sort key
2. **`reading_order.rs`**: Add `compute_smart_sort_key()` helper function
3. **Add boundary normalization**: 3pt tolerance for column edge alignment

### Pseudo-code

```rust
fn compute_smart_sort_key(block: &Block, all_blocks: &[Block], columns: &[BoundingBox]) -> (f32, f32) {
    // Find blocks to the LEFT of this block that overlap vertically
    let left_blocks: Vec<_> = all_blocks.iter()
        .filter(|b| {
            // Block is to the left (its right edge < our left edge)
            b.bbox.x2 < block.bbox.x1 - BOUNDARY_TOLERANCE &&
            // Vertical overlap exists
            (block.bbox.y1 <= b.bbox.y1 && b.bbox.y1 <= block.bbox.y2) ||
            (block.bbox.y1 <= b.bbox.y2 && b.bbox.y2 <= block.bbox.y2)
        })
        .collect();

    if let Some(left_block) = left_blocks.iter().max_by(|a, b| a.bbox.x2.partial_cmp(&b.bbox.x2).unwrap()) {
        // Use left block's Y for sort, but our X
        (left_block.bbox.y1, block.bbox.x1)
    } else {
        // No left block, use original position
        (block.bbox.y1, block.bbox.x1)
    }
}
```

### Constants (from pymupdf4llm first principles)

```rust
/// Tolerance for boundary normalization (pixels)
/// WHY: PDF text coordinates vary by 1-3 pixels even for aligned columns.
/// MuPDF/pymupdf4llm uses 3pt tolerance based on common PDF generation tools.
const BOUNDARY_ALIGNMENT_TOLERANCE: f32 = 3.0;

/// Tolerance for vertical gap joining (pixels)
/// WHY: Paragraphs in PDFs typically have 10-12pt leading. A gap < 10pt is
/// likely within the same logical block. pymupdf4llm uses 10pt.
const VERTICAL_GAP_TOLERANCE: f32 = 10.0;
```

## Priority

P0 - Critical for quality improvement

## Validation

Run after implementation:

```bash
python3 scripts/compare_against_pymupdf.py --pdf-dir edgequake/crates/edgequake-pdf/test-data/real_dataset --only 01_2512.25075v1
```

Target: F1 for `01_2512.25075v1.pdf` should improve from 0.552 to at least 0.70
