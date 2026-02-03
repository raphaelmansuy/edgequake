# OODA-22: Act Phase

## Changes Made

### File: `src/backend/element_processing.rs`

Added absolute position cross-column check in the `merge()` function:

```rust
// OODA-22 FIX: The previous gap-based check fails when current_end_x overflows.
// When text accumulates, estimated end_x can be 500+ even though actual
// content ends at ~275. This causes gap to be negative, and the large_gap
// check incorrectly returns false.
//
// New approach: Use ABSOLUTE X positions as the primary cross-column check.
// In academic papers:
// - Left column: X ≈ 55-280
// - Right column: X ≈ 305-540
// If current started in left column AND next is clearly in right column,
// do NOT merge regardless of gap.
let current_started_left = current.x < 200.0;
let next_is_right_column = next.x > 280.0;
let absolute_column_boundary = current_started_left && next_is_right_column;

// OODA-22: Combine all cross-column checks - absolute position is most reliable
let likely_cross_column = absolute_column_boundary || large_gap_indicates_column || margin_to_column;
```

### File: `src/backend/extraction_engine.rs`

Also fixed the `group_into_lines` function to accept pre-detected column boundary instead of detecting again after table filtering (related fix discovered during investigation).

## Results

### Before OODA-22
- Overall Quality: 86.5%
- Element loss on merge: 12 pages losing right column content

### After OODA-22
- Overall Quality: **87.5%** (+1.0 percentage points)
- Element loss on merge: 2 pages (edge cases with narrow columns)

### Per-Document Changes
| Document | Before | After | Delta |
|----------|--------|-------|-------|
| 2502.v1.multi_modal | 86.6% | 90.5% | +3.9% |
| 2502.v2.multi_modal | 85.7% | 88.3% | +2.6% |
| one_tool_2512 | 82.8% | 83.3% | +0.5% |
| **Overall** | 86.5% | 87.5% | **+1.0%** |

## Verification

```bash
cargo test -p edgequake-pdf --test comprehensive_quality --features comprehensive-tests --release
# Overall Quality: 87.5%
```
