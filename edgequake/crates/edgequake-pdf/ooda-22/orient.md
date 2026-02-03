# OODA-22: Orient Phase

## Problem Pattern Recognition

The element merge function uses gap-based cross-column detection:
```rust
let large_gap_indicates_column = gap > large_gap_threshold 
    && current_in_left_half && next_in_right_half;
```

This fails when `current_end_x` (estimated from accumulated text) overflows beyond `next.x`, making the gap negative.

## Why Previous Cross-Column Checks Failed

1. **Primary check** (`margin_to_column`):
   - Requires `current.x < 100`
   - But accumulated text starting at X=134 doesn't meet this

2. **Secondary check** (`large_gap_indicates_column`):
   - Requires `gap > large_gap_threshold`
   - But gap is negative when end_x overflows

3. **Overlap check** (`overlapping`):
   - `next.x >= current.x && next.x < current_end_x`
   - With overflowed end_x (605), right column elements (X=310) appear to "overlap"

## Key Insight

The merge condition `!likely_cross_column && (overlapping || ...)` allows merging when:
- Cross-column check fails (because gap is negative)
- Overlapping is true (because end_x overflowed)

This creates a false positive for merging cross-column elements.

## Strategic Options

1. **Fix gap calculation**: Use actual element boundaries instead of estimated end_x
2. **Add absolute position check**: If current started left AND next is right column → don't merge
3. **Limit end_x estimation**: Cap the estimated width to prevent overflow
