# Orient – OODA-19: Add Font Analysis Tests

## Gap Identified

`processors/font_analysis.rs` has only 2 tests but handles critical functionality:

- Body font size detection (affects heading classification)
- Size range validation

## Test Strategy

Add tests for edge cases:

1. **Median edge cases**:
   - Even number of elements
   - Single element
   - Two elements

2. **Font analyzer with documents**:
   - Empty document → default 12pt
   - Document with only headers → default 12pt
   - Mixed content → median body size

## Implementation Plan

Add 4-5 new tests to increase coverage from 2 to 7+ tests.
