# OODA-14: Act - Add Heading Level Tests

## Actions Taken

1. **Added `test_heading_level_classification` test** to `layout/block_classifier.rs`:
   - Tests H1 at 2.0x ratio (20pt on 10pt body)
   - Tests H2 at 1.8x ratio (18pt on 10pt body)
   - Tests H1 conservative at 1.6x ratio (16pt on 10pt body)
   - Tests Paragraph at 1.0x ratio (10pt on 10pt body)
   - Tests edge case at exactly 1.5x threshold (15pt on 10pt body)

2. **Added helper function** `make_heading_block` for creating test blocks

3. **Added WHY comments** explaining conservative H1 approach

## Results

- **Test result**: PASS
- **Total lib tests**: 453 (was 452, +1 new)
- **All tests pass**: ✅

## Test Coverage Matrix

| Ratio | Expected | Test Case |
|-------|----------|-----------|
| >= 2.0 | H1 | ✅ 20pt/10pt |
| 1.7-2.0 | H2 | ✅ 18pt/10pt |
| 1.5-1.7 | H1 (conservative) | ✅ 16pt/10pt |
| < 1.5 | Paragraph | ✅ 10pt/10pt |
| = 1.5 | Paragraph (edge) | ✅ 15pt/10pt |

## Next Steps

- OODA-15: Add test for Roman numeral header detection
- OODA-16: Add test for letter subsection detection
- OODA-17: Add integration test for multi-level heading document
