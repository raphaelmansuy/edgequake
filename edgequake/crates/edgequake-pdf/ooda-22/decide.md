# OODA-22: Decide Phase

## Chosen Solution: Absolute Position Cross-Column Check

Add a new check that uses absolute X positions instead of gap/overlap:

```rust
let current_started_left = current.x < 200.0;
let next_is_right_column = next.x > 280.0;
let absolute_column_boundary = current_started_left && next_is_right_column;
```

This catches the cross-column case even when:

- Gap is negative (due to overflowed end_x)
- Overlapping appears true

## Rationale

1. **Simple and robust**: Uses the element's original X position, not estimated end positions
2. **Academic paper standard**: Left column typically X < 280, right column X > 280
3. **Doesn't break existing cases**: Added as additional check, not replacing others

## Implementation Location

`edgequake/crates/edgequake-pdf/src/backend/element_processing.rs`

In the `merge()` function, before the cross-column check, add:

```rust
// OODA-22 FIX: Absolute position cross-column check
let current_started_left = current.x < 200.0;
let next_is_right_column = next.x > 280.0;
let absolute_column_boundary = current_started_left && next_is_right_column;

// Combine with existing checks
let likely_cross_column = absolute_column_boundary || large_gap_indicates_column || margin_to_column;
```

## Expected Impact

- Prevent right-column elements from being merged into left-column text
- Improve element preservation from ~43 to ~53 per page
- Increase quality score by recovering missing content
