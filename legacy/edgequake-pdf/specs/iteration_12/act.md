# OODA-12: ACT — Column Assignment and Reference Detection Fixes

## Implementation Summary

**Date**: 2025-01-30
**Focus**: Fix cross-column reference merging in two-column PDFs

## Changes Made

### 1. Gap Zone Column Assignment (src/backend/text_grouping.rs)

**Lines**: 241-275

**Before**:

```rust
if elem.x < column_boundary {
    left_column.push(elem);  // Wrong for elements at X≈313 with boundary=320
}
```

**After**:

```rust
let page_center = _page_width / 2.0;
if elem.x < page_center {
    left_column.push(elem);
}
```

**WHY**: Column boundary detection finds gap center, not column edge. Right column content starts left of gap center. Page center is more reliable.

### 2. Academic Reference Pattern Detection (src/processors/layout_processing.rs)

**Lines**: 207-228

Added `is_academic_ref` check:

```rust
let is_academic_ref = trimmed_b.len() > 2
    && trimmed_b.starts_with('[')
    && trimmed_b.chars().skip(1).take_while(|c| c.is_ascii_digit()).count() >= 1
    && trimmed_b.contains(']');

if ... || is_academic_ref || ... {
    return false;  // Don't merge
}
```

**WHY**: Prevents BlockMergeProcessor from merging references with preceding text.

## Quality Metrics

### Before OODA-12

| Metric              | Score |
| ------------------- | ----- |
| Text Preservation   | 83.6% |
| Structural Fidelity | 74.5% |
| Overall Quality     | 79.1% |

### After OODA-12

| Metric              | Score | Delta     |
| ------------------- | ----- | --------- |
| Text Preservation   | 83.5% | -0.1%     |
| Structural Fidelity | 77.3% | **+2.8%** |
| Overall Quality     | 80.4% | **+1.3%** |

### Per-PDF Changes

| PDF         | Before | After | Delta     |
| ----------- | ------ | ----- | --------- |
| ccn         | 83.1%  | 83.1% | 0%        |
| 2900_Goyal  | 85.5%  | 85.5% | 0%        |
| v2          | 68.4%  | 69.2% | +0.8%     |
| AlphaEvolve | 81.1%  | 82.1% | +1.0%     |
| agent       | 79.2%  | 79.2% | 0%        |
| 01          | 76.2%  | 83.8% | **+7.6%** |
| one_tool    | 80.1%  | 80.2% | +0.1%     |

## Test Results

- 4 smoke tests pass
- Comprehensive quality test passes
- No regressions

## Remaining Issues

- v2 structural fidelity still at 52.8% (was 50.4%)
- Need to investigate why v2 has so many missing list items
- May need reference-section-specific processing

## Next Steps (OODA-13)

1. Investigate v2 reference section in detail
2. Check if references are being detected as Text instead of ListItem
3. Consider adding reference section header detection to trigger special processing
