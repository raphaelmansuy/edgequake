# OODA-12: ORIENT — Column Assignment Root Cause

## Analysis Summary

The root cause of the v2 PDF's poor structural fidelity (50.4%) is **cross-column reference merging** in the REFERENCES section.

## Problem Chain

```
┌─────────────────────────┐
│ PDF Page 9 (References) │
│ 2-column layout         │
│ boundary = 320.0        │
└───────────┬─────────────┘
            │
            ▼
┌─────────────────────────────────────────────────────┐
│ Element [25] at X=313.2 (right column content)      │
│                                                      │
│ Current logic: GAP zone (305 < X < 335)             │
│ Assignment: if X < boundary → LEFT column           │
│ Result: 313.2 < 320 → WRONG! Assigned to LEFT       │
└─────────────────────────────────────────────────────┘
            │
            ▼
┌─────────────────────────────────────────────────────┐
│ [1] L. Sentis... (left column, X≈58)                │
│ [25] T. He, Z... (right column, X≈313)              │
│                                                      │
│ Both in "left" column → grouped into same line      │
│ → BLOCK-XRANGE: [58,313] = 255pt wide!              │
└─────────────────────────────────────────────────────┘
```

## Root Cause: Gap Zone Assignment Logic

**File**: `src/backend/text_grouping.rs`, lines 241-262

**Original Logic**:

```rust
// Gap zone: within ±15pt of boundary
if elem.x < column_boundary {
    left_column.push(elem);  // X=313.2 < 320 → LEFT (WRONG!)
}
```

**Problem**: Column boundary (320.0) is the gap center between columns. But the right column content actually STARTS at X≈313. Elements in the gap are assigned based on `X < boundary`, which incorrectly puts X=313 in LEFT.

## First Principles Analysis

A two-column layout has:

- Left column: typically X ∈ [~54, ~290]
- Gap: X ∈ [~290, ~320]
- Right column: typically X ∈ [~313, ~560]

The gap boundary (320) is detected from the whitespace between columns. But right column text starts slightly LEFT of this boundary.

**Correct Assignment**: Use page center (page_width/2 = 306 for 612pt page) as tie-breaker. Elements at X > 306 should go to RIGHT column.

## BlockMergeProcessor Enhancement

Also added academic reference `[N]` pattern detection to prevent merging when next block starts with a reference marker. This prevents:

- `...in ICRA., 2006.` + `[2] X. Cheng...` from merging

## Expected Impact

- References from right column (X≈313) now correctly assigned to RIGHT
- Each reference remains as separate block
- List detection can then mark them as ListItem type
- Structural fidelity should improve for reference-heavy papers
