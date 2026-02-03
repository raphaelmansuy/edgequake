# OODA-22: Observe Phase

## Symptom

Quality plateau at 86.5% despite many pages showing "TG-TWOCOL" (two-column detection).

## Investigation

### Column Detection Logs

```
Page 12 OODA08 column boundary: None
Page 20 OODA08 column boundary: None
```

Many pages incorrectly detected as single-column.

### Element X-Range Analysis

After adding logging to track X-ranges through the pipeline:

```
ENG-RAW-PAGE12: x_range=[134.3, 317.0] count=53
ENG-PROCESSED-PAGE12: max_x=151.7 count=43
```

**Key Finding**: Raw extraction has elements at X=317 (right column), but after element processing, max_x dropped to 151.7.

### Trace Through Processing Pipeline

1. Raw extraction: max_x = 317.0, count = 53
2. After dedup: max_x = 317.0, count = 53 (unchanged)
3. After merge: max_x = 151.7, count = 43 (10 elements LOST!)

The `merge()` function in `element_processing.rs` is consuming right-column elements.

## Root Cause Identified

In `element_processing.rs`, the merge function tracks `current_end_x` as an estimate:

```rust
current_end_x = current.x + current.text.chars().count() * font_size * 0.55
```

For accumulated text with 100+ chars:

- `current_end_x = 55 + 100 * 10 * 0.55 = 605pt`

This causes:

1. `gap = next.x - current_end_x = 310 - 605 = -295` (NEGATIVE!)
2. `large_gap_indicates_column` requires `gap > large_gap_threshold` → FALSE
3. `overlapping = next.x < current_end_x = 310 < 605` → TRUE

Result: Right column elements at X=310 get merged into left column text because `overlapping=true` and `likely_cross_column=false`.
