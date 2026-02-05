# OODA-09: Act - Document Magic Numbers in text_grouping.rs

## Actions Taken

1. **Added WHY comment for 100pt threshold** (line ~309):
   - Explains 100pt (~13% of US Letter height) captures title/author zone
   - Documents purpose: logging elements for header classification debugging

2. **Enhanced WHY comments for author zone** (15-80pt):
   - 15pt: ~2% of page, below header margin
   - 80pt: ~10% of page, above abstract start
   - Documents author zone purpose and boundaries

3. **Added WHY comment for 30pt gap threshold** (lines ~576-578):
   - 30pt (~4% of page height) indicates section boundary
   - Single-spaced text ~12-14pt, so 30pt = 2+ blank lines
   - Separates main content from bottom sections

4. **Added WHY comments for continuation thresholds**:
   - 30 chars: author name fragments are short
   - 20pt: slightly below author zone for edge cases

## Results

- **All tests pass**: 452 lib tests ✅
- **No logic changes**: Comments only
- **No clippy warnings**: ✅

## Magic Numbers Now Documented

| Value | Location | Purpose |
|-------|----------|---------|
| 100.0pt | line ~309 | Top zone for logging |
| 15.0pt | line ~412 | Author zone lower bound |
| 80.0pt | line ~412 | Author zone upper bound |
| 30.0pt | line ~576 | Vertical gap threshold |
| 20.0pt | line ~425 | Continuation detection |
| 30 chars | line ~425 | Short text threshold |

## Next Steps

- OODA-10: Look for more undocumented magic numbers in other files
- OODA-11: Add test for line-level style preservation
- OODA-12: Document pymupdf_grouper.rs magic numbers
