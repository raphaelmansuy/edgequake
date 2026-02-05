# OODA-15: Act - Add Subsection Pattern Tests

## Actions Taken

1. **Added `test_subsection_patterns` test** to `layout/block_classifier.rs`:
   - Tests letter subsection: "A. Background", "B. Policy", "Z. Final"
   - Tests numeric section: "1. INTRODUCTION", "2. METHODS"
   - Tests numeric subsection: "2.1. Agentic Training", "3.2 Architecture"
   - Tests invalid cases for each pattern type

2. **Discovered function behavior**:
   - `is_numeric_section_header` requires ". " (dot space), not just space
   - Updated test cases to match actual function requirements

## Results

- **Test result**: PASS
- **Total lib tests**: 454 (was 453, +1 new)
- **All tests pass**: ✅

## Test Coverage Matrix

| Function                       | Valid Cases | Invalid Cases           |
| ------------------------------ | ----------- | ----------------------- |
| `is_letter_subsection_header`  | ✅ A/B/Z. X | ✅ A.NoSpace, AB., 1.   |
| `is_numeric_section_header`    | ✅ 1./2. X  | ✅ 1.1., lowercase, 2 X |
| `is_numeric_subsection_header` | ✅ 2.1./3.2 | ✅ 2., no pattern       |

## Next Steps

- OODA-16: Add test for abstract header detection
- OODA-17: Fix the unused_mut warning in pymupdf_grouper.rs
- OODA-18: Add ASCII diagram for column detection algorithm
