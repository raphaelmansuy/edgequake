# OODA-10: Act - Document Constants in pymupdf_grouper.rs

## Actions Taken

1. **Added WHY comment for column_overlap: 0.5** (line ~110):
   - 50% horizontal overlap threshold for same-column detection
   - Handles indented paragraphs while preventing adjacent column merging

2. **Added WHY comment for COLUMN_GAP_THRESHOLD: 10.0** (line ~305):
   - 10pt is less than typical column gutter (14-20pt)
   - Larger than word gaps (<5pt)
   - Provides margin for detection uncertainty

3. **Added WHY comment for page_width < 100.0** (line ~503):
   - 100pt ≈ 1.4 inches is too small for readable content
   - Typical pages: US Letter (612pt), A4 (595pt)
   - Skip column detection for narrow content

## Results

- **All tests pass**: 452 lib tests ✅
- **No logic changes**: Comments only
- **No clippy warnings**: ✅

## Constants Now Documented in pymupdf_grouper.rs

| Value | Purpose |
|-------|---------|
| 0.5 (column_overlap) | Same-column overlap threshold |
| 10.0 (COLUMN_GAP_THRESHOLD) | Column gap detection |
| 100.0 (page_width) | Minimum page width for column detection |

## Next Steps

- OODA-11: Review and document constants in block_classifier.rs
- OODA-12: Review markdown renderer for magic numbers
- OODA-13: Add more integration tests for edge cases
