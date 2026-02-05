# OODA-12: Act - Document Constants in block_classifier.rs

## Actions Taken

1. **Added WHY comment for heading level ratios** (line ~133):
   - 2.0x: Very large (double body) = major heading (#)
   - 1.7x: Large (70% bigger) = secondary heading (##)
   - 1.5x: Medium = default to # (conservative)

2. **Added WHY comment for uppercase ratio** (line ~294):
   - 50% uppercase threshold for all-caps section detection
   - True all-caps = 100%, but OCR/extraction may have errors
   - 50% catches "ABSTRACT", "REFERENCES" with some lowercase mixed in

## Results

- **All tests pass**: 452 lib tests ✅
- **No logic changes**: Comments only
- **No clippy warnings**: ✅

## Constants Now Documented in block_classifier.rs

| Value         | Purpose                          |
| ------------- | -------------------------------- |
| 2.0x ratio    | Major heading (H1) threshold     |
| 1.7x ratio    | Secondary heading (H2) threshold |
| 0.5 uppercase | All-caps section detection       |

## Next Steps

- OODA-13: Review pdfium.rs for undocumented constants
- OODA-14: Add integration test for heading level classification
- OODA-15: Document table detection algorithm
