# Orient – OODA-20: Add Percentile and Stats Tests

## Strategy

Add tests for:

1. **Percentile edge cases**:
   - Single element array
   - Two element array
   - Very large array (10+ elements)

2. **Percentile boundary values**:
   - 10th percentile (used for alignment tolerance)
   - 30th percentile (used for line spacing)
   - 50th percentile (median for body font size)

## Implementation

All tests can be unit tests for the static `percentile()` function, no complex Document setup needed.
