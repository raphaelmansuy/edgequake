````markdown
# OODA-06 Act: Peak Detection Implementation

## Implementation Summary

Successfully implemented peak detection approach for two-column layout detection in academic papers.

## Changes Made

### File: `edgequake/crates/edgequake-pdf/src/backend/column_detection.rs`

**1. New Peak Detection Algorithm (Lines 47-89)**

Added `find_two_peaks()` as primary detection method:

- Finds the two most prominent peaks in the start-position histogram
- Peaks represent column left margins (where text begins)
- Left peak ≈ X=55-70 (left column margin)
- Right peak ≈ X=305-320 (right column margin)
- Boundary = right_peak - 10 (places in gutter before right column)

**2. Start-Position-Only Histogram (Lines 288-301)**

Changed `compute_vertical_projection()`:

- Previously counted element SPANS (caused gutter to be filled)
- Now counts only START positions (shows where lines begin)
- Creates clear gap between column clusters

**3. Fixed Closest Gap Selection (Lines 103-111)**

Changed from `.find()` to `.min_by()`:

```rust
// BEFORE (wrong):
let center_gap = gaps.iter().find(|&&gap| ...);

// AFTER (correct):
let center_gap = gaps.iter()
    .filter(|&&gap| (gap - center).abs() < center_range)
    .min_by(|a, b| { /* compare distances to center */ })
    .copied();
```
````

**4. Peak Detection Algorithm (Lines 359-443)**

New `find_two_peaks()` function:

1. Groups consecutive non-zero bins into regions
2. Finds regions with significant count (≥5% of total)
3. Sorts by peak height, takes top 2
4. Validates separation (≥30% of page width)
5. Returns (left_peak_x, right_peak_x)

## Test Results

Tested on `agentfail_2601.22984v1.pdf` (39-page arXiv paper):

**Before Fix:**

```
Detected SINGLE-COLUMN layout (left_starts=19, right_starts=3)
```

**After Fix:**

```
Peak detection: left_peak=65.0, right_peak=315.0, boundary=305.0
Detected TWO-COLUMN layout (peak method) with boundary at 305.0
```

## Verification

- Two-column detection now works on body pages
- Left column text correctly processed before right column
- Gutter boundary at X≈295-310 (correct for arXiv format)
- Some appendix pages with figures correctly detected as single-column

## Markitdown Comparison

| Metric           | EdgeQuake   | Markitdown |
| ---------------- | ----------- | ---------- |
| Output lines     | 3163        | ~4000      |
| Headers detected | 96          | Similar    |
| Tables rendered  | Yes         | Better     |
| Column order     | Correct now | Correct    |

## Remaining Issues (for OODA-07+)

1. **Word tokenization**: "BrowseComp" → "Browse Comp" (hyphen/space issue)
2. **Table structure**: Some tables have too many columns
3. **Side-by-side tables**: Table 2 and 3 merged incorrectly
4. **Reading order glitches**: Minor issues in complex layouts

## Quality Metrics (Estimated)

| Metric                    | Before | After |
| ------------------------- | ------ | ----- |
| TPS (Text Preservation)   | 65%    | 80%   |
| SFS (Structural Fidelity) | 25%    | 60%   |
| ROA (Reading Order)       | 15%    | 75%   |

## Code Quality

- ✅ `cargo build --release` - Success
- ✅ `cargo clippy` - No warnings
- ✅ `cargo test` - All tests pass
- ✅ Debug logging added for troubleshooting

```

```
