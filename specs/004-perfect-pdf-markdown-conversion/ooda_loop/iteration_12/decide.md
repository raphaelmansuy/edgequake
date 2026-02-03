# OODA-12: Decide

## Decision

Implement **Option A**: Skip Y-sort for multi-column pages.

## Rationale

1. **Root cause is clear**: The Y-sort destroys text_grouping's column-aware order
2. **Minimal risk**: Only changes behavior for multi-column pages
3. **text_grouping already sorts by Y**: The sort is redundant for column content
4. **Simple implementation**: One conditional check

## Implementation Plan

### File: `src/backend/extraction_engine.rs`

### Location: Lines 621-638

### Current Code:

```rust
// Sort blocks by Y coordinate for correct reading order
//
// After Y-normalization: lower Y = top of page, higher Y = bottom
// For ALL layouts, we now sort by Y since Y-normalization has already
// established a consistent coordinate system where ascending Y = top to bottom.
//
// For multi-column layouts, the content is already organized by text_grouping
// (left column first, then right column), and the Y values within each
// column section will naturally sort correctly.
//
// WHY sort here? The text grouping may return lines in an order that reflects
// the order elements were parsed from the PDF (not necessarily top-to-bottom).
// After Y-normalization, sorting by Y gives the correct visual reading order.
blocks.sort_by(|a, b| {
    a.bbox
        .y1
        .partial_cmp(&b.bbox.y1)
        .unwrap_or(std::cmp::Ordering::Equal)
});
```

### New Code:

```rust
// Sort blocks by Y coordinate for correct reading order
//
// OODA-12 FIX: Only sort for single-column layouts. For multi-column layouts,
// text_grouping.rs already establishes correct reading order:
// - Elements are sorted by Y within each column in group_single_column_layout()
// - Columns are concatenated in correct order: left column first, then right column
//
// Sorting multi-column pages by Y destroys this order by interleaving blocks
// at similar Y coordinates from different columns.
//
// Example: Two-column REFERENCES section
//   Before fix: [ref1, ref2, ref3, ref4] (interleaved by Y)
//   After fix:  [ref1, ref3, ref2, ref4] (left col, then right col)
//
if columns.len() <= 1 {
    // Single-column: sort by Y for top-to-bottom reading order
    blocks.sort_by(|a, b| {
        a.bbox
            .y1
            .partial_cmp(&b.bbox.y1)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
} else {
    // Multi-column: trust text_grouping's column-aware order
    // Log for visibility during debugging
    tracing::debug!(
        "OODA-12: Skipping Y-sort for {}-column page (blocks={}, using text_grouping order)",
        columns.len(),
        blocks.len()
    );
}
```

## Expected Impact

### Metrics Improvement

- v2 structure score: 53.6% → ~80%+ (estimated)
- Overall average: 80.5% → ~85%+ (estimated)

### Test Verification

1. Run comprehensive tests before and after
2. Compare v2 REFERENCES section output
3. Verify no regression on other PDFs

## Commit Plan

After implementation:

```
git add .
git commit -m "OODA-12: Fix multi-column reading order by skipping Y-sort for multi-column pages"
```
