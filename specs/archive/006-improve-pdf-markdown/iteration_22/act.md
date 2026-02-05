# Act – OODA-22: Implement Layout Confidence Calculation

## What Changed

1. **Replaced TODO with actual implementation**:
   - Removed `confidence: 0.9` hardcoded value
   - Added `calculate_confidence()` method with weighted scoring

2. **Confidence Factors**:
   - **Reading order coverage (50%)**: ratio of blocks in reading order
   - **Column detection (30%)**: single=1.0, multi=0.95
   - **Region quality (20%)**: empty=0.8, over-fragmented=0.7, good=1.0

3. **Added 3 tests**:
   - `test_confidence_calculation_perfect` - ideal case
   - `test_confidence_calculation_missing_blocks` - low coverage
   - `test_confidence_calculation_empty` - edge case

## Code Location

- `edgequake/crates/edgequake-pdf/src/layout/mod.rs`

## Verification

```
cargo test layout::tests --lib
# Result: 7 passed (was 4)

cargo test --lib
# Result: 469 passed (up from 466)
```

## Value Added

- Removed one TODO from codebase
- Confidence now reflects actual analysis quality
- Documented WHY for each confidence factor
- Tests verify the calculation logic

## Remaining TODOs

4 more TODOs in edgequake-pdf:

- `extractor.rs:539` - Extract images
- `layout/pymupdf_grouper.rs:163` - Vertical text detection
- `layout/pymupdf_renderer.rs:156` - Table detection
- `backend/pdfium_backend.rs:296` - Image scanning

## Next Iteration

OODA-23: Continue addressing remaining TODOs
