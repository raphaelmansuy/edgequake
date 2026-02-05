# OODA-11: Act - Document Constants in markdown.rs

## Actions Taken

1. **Added WHY comment for list indentation constants** (line ~251):
   - 72pt = 1 inch = standard PDF left margin
   - 20pt ≈ 0.28" per level = standard typographic indent step
   - Formula: (indent - margin) / step_size = nesting level

2. **Added WHY comment for table row Y-tolerance** (line ~607):
   - 10pt Y-tolerance for same-row detection
   - Matches other tolerances (block_gap, line joining) in codebase
   - Cells on same row should have Y positions within 10pt

## Results

- **All tests pass**: 452 lib tests ✅
- **No logic changes**: Comments only
- **No clippy warnings**: ✅

## Constants Now Documented in markdown.rs

| Value | Purpose |
|-------|---------|
| 72.0pt | Standard PDF left margin (1 inch) |
| 20.0pt | List indent step (~0.28 inch per level) |
| 10.0pt | Table row Y-tolerance |

## Next Steps

- OODA-12: Check block_classifier.rs for undocumented constants
- OODA-13: Review pymupdf_structs.rs for additional documentation needs
- OODA-14: Add ASCII diagram for table rendering algorithm
