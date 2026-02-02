# OODA-06 Decide: Fix Column Detection

## Root Cause Confirmed

The column detector uses `.find()` to get the FIRST gap within range of center, but should use the CLOSEST gap to center.

From debug analysis:

```
Gaps found: [35.0, 117.5, 227.5]
```

- Gap at 227.5 is found first (within center_range of 91.8pt from center 306pt)
- But the actual gutter at X≈285 is not being detected or prioritized

## Issue 1: Wrong Gap Selection

**Current Code (column_detection.rs:57-59):**

```rust
let center_gap = gaps
    .iter()
    .find(|&&gap| (gap - center).abs() < center_range);
```

**Problem:** `.find()` returns the FIRST gap within range, not the closest to center.

**Fix:** Use `.min_by()` to find the gap CLOSEST to center:

```rust
let center_gap = gaps
    .iter()
    .filter(|&&gap| (gap - center).abs() < center_range)
    .min_by(|a, b| {
        let dist_a = (**a - center).abs();
        let dist_b = (**b - center).abs();
        dist_a.partial_cmp(&dist_b).unwrap()
    })
    .copied();
```

## Issue 2: Gap Not Detected at Gutter

The gap at X≈285 (actual gutter) may not be detected because:

1. The projection histogram is counting elements that span across the gap
2. The `min_gap_bins` = 4 (20pt) may be too wide for some documents

From histogram analysis:

- X 275-300: 1 position (gap zone)
- X 300-325: 50 positions (right column)

So there IS a gap at X≈280-300 but it's not being detected. Let me check if it's in the gaps list.

## Decision: Fix in Order

1. **Priority 1:** Fix gap selection to use CLOSEST to center
2. **Priority 2:** Verify gap detection is finding the gutter
3. **Priority 3:** Adjust element distribution check

## Implementation Plan

1. Modify `column_detection.rs:57-59` to use `min_by` instead of `find`
2. Add debug logging to show all gaps and distances from center
3. Test on agentfail PDF page 2
4. Verify two-column layout is detected correctly
