# OODA-09: Decide

## Decision Summary

Fix cross-column text merging in `element_processing.rs::merge()` using a margin-based column boundary detection heuristic.

## Problem Statement

In two-column PDF layouts, text elements from different columns at the same Y-coordinate are incorrectly merged:

```
BEFORE: "Abstract— Humanoid robots hold great promise for oper-manipulate objects [1]."
        ↑ Left column text                              ↑ Right column text

AFTER:  "Abstract— Humanoid robots hold great promise for oper-"
        "manipulate objects [1]. Achieving this level of dexterity and"
```

## Root Cause Analysis

1. **Element accumulation inflates estimated width**: When merging elements, `estimated_width = text.len() * char_width` grows unbounded
2. **Overlap check becomes true for cross-column elements**: With 300+ chars accumulated, estimated_width ≈ 1350pt, causing `next.x < current_end_x` to be true
3. **Gap calculation becomes negative**: `gap = next.x - current_end_x = 313.2 - 330.3 = -17.1pt`

## Decision: Margin-Based Column Detection

### Chosen Approach

Use document geometry to detect column boundaries:

```
┌────────────────────────────────────────────────────────────────┐
│     LEFT MARGIN      │      GUTTER       │    RIGHT COLUMN    │
│     (X < 100pt)      │   (100-300pt)     │    (X > 300pt)     │
├──────────────────────┴───────────────────┴────────────────────┤
│  Two-column PDF:                                               │
│  - Left column starts at X ≈ 64pt (margin)                     │
│  - Right column starts at X ≈ 313pt                            │
│                                                                │
│  Single-column PDF (e.g., Qwen.pdf):                          │
│  - Content starts at X ≈ 183pt (centered, NOT margin)          │
│  - Spans X = 183-650+ (no column boundary)                     │
└────────────────────────────────────────────────────────────────┘
```

### Key Discriminator: Left Margin vs Centered Content

- **If `current.x < 100` AND `next.x > 300`**: Column boundary (200pt span impossible in single line)
- **If `current.x >= 100`**: Centered/wide content, NOT a column boundary

### Implementation

```rust
// Primary check: Left margin to right column = definite column boundary
let current_in_left_margin = current.x < 100.0;  // Left margin region
let next_in_right_column = next.x > 300.0;       // Right column start
let margin_to_column = current_in_left_margin && next_in_right_column;

// Secondary check: Large gap indicates column boundary
let large_gap_threshold = char_width * 4.0;
let current_in_left_half = current.x < 250.0;
let next_in_right_half = next.x > 280.0;
let large_gap_indicates_column = gap > large_gap_threshold
    && current_in_left_half && next_in_right_half;

let likely_cross_column = margin_to_column || large_gap_indicates_column;
```

## Alternatives Considered

### 1. Gap-Only Detection (Rejected)

- **Threshold**: `gap > 4 * char_width`
- **Problem**: When end_x is overestimated, gap becomes negative (-17.1pt)
- **Verdict**: ❌ Cannot detect column boundary with negative gaps

### 2. Position-Only Detection (Rejected)

- **Threshold**: `current.x < 200 && next.x > 280`
- **Problem**: Breaks Qwen.pdf - 60pt title characters at X=320.6 trigger false positive
- **Verdict**: ❌ Too aggressive for single-column PDFs

### 3. Hybrid Position + Jump Detection (Rejected)

- **Threshold**: `gap < -50 && current.x < 200 && next.x > 300`
- **Problem**: gap=-17.1 doesn't meet -50 threshold
- **Verdict**: ❌ Threshold too strict for this case

### 4. Margin-Based Detection (Chosen) ✅

- **Key insight**: Left margin (X < 100) is unambiguous indicator of two-column layout
- **Works for v2 PDF**: `current.x=64 < 100` AND `next.x=313 > 300` → column boundary
- **Works for Qwen.pdf**: `current.x=183 >= 100` → NOT a column boundary
- **Verdict**: ✅ Robust discrimination

## Expected Impact

| Metric              | Before | After  | Target |
| ------------------- | ------ | ------ | ------ |
| Text Preservation   | 81.9%  | 81.9%  | 98%    |
| Structural Fidelity | 68.8%  | ~69.0% | 95%    |
| Overall Quality     | 75.3%  | ~75.4% | 95%    |

**Note**: This fix is necessary but not sufficient for 95% target. Additional improvements needed for:

- Table detection accuracy
- Multi-column reading order
- Header/footer filtering

## Risk Assessment

| Risk                                       | Likelihood | Impact | Mitigation              |
| ------------------------------------------ | ---------- | ------ | ----------------------- |
| False positives (wrong boundary detection) | Low        | Medium | Tested against Qwen.pdf |
| Narrow pages breaking (<200pt wide)        | Low        | Low    | Rare in academic papers |
| Three-column layouts                       | Medium     | Low    | May need refinement     |

## Next Steps (OODA-10+)

1. Run comprehensive quality tests to verify improvement
2. Investigate v2 PDF structural fidelity (47.2%) - likely table detection issues
3. Analyze 01_2512.25075v1 (64.9%) for additional structural problems
