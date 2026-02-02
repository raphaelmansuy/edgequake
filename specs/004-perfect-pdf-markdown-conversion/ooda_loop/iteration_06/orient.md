# OODA-06 Orient: Root Cause Analysis

## Problem Statement

Academic papers (arXiv style) are being detected as SINGLE-COLUMN when they are TWO-COLUMN.

## Evidence from Debug Logs

```
Page 2:
Found gap at X=227.5 (width=25 bins)
Projection gap at X=227.5: left=44, right=1, balance=0.02
Projection gap rejected: left=44, right=1, balance=0.02 (need >=5 each, balance>0.25)
```

The gap at X=227.5 is correctly detected but rejected because:

- left=44 elements (X < 227.5)
- right=1 element (X >= 227.5)
- balance=0.02 (way below 0.25 threshold)

## Root Cause: Element X Coordinate Issue

The problem is that elements in the RIGHT column have X coordinates that are LESS than the gap boundary (227.5).

This could be caused by:

1. **Text starting position in PDF is offset** - lopdf may return text position as the start of the text run, not the actual glyph positions
2. **Merged text elements** - After merging nearby elements, the X coordinate represents the leftmost position
3. **Gap position is miscalculated** - Gap midpoint may not accurately represent the column separator

## Deeper Analysis

Looking at how the check works:

```rust
let left_count = elements.iter().filter(|e| e.x < boundary).count();
let right_count = elements.iter().filter(|e| e.x >= boundary).count();
```

This uses `e.x` (start position) but should consider the element's full width:

- An element with `x=200, width=100` spans 200-300
- If boundary=227.5, this element is STRADDLING the boundary
- Current logic counts it as "left" (x=200 < 227.5)

For a two-column paper:

- Left column elements: x ≈ 55-280
- Right column elements: x ≈ 300-555
- Gap/gutter: x ≈ 280-310

If boundary is detected at 227.5 (within left column text), then:

- All left column elements have x < 227.5
- All right column elements have x > 227.5 (should be counted!)

Wait, but log shows only 1 element with x >= 227.5. Let me reconsider...

## Alternative Hypothesis: Skewed Element Distribution

Maybe the gap detection is finding a local gap within the LEFT COLUMN only:

- Gap at X=227.5 could be between lines of text in the left column
- The actual column gutter is likely at X ≈ 290-310

Looking at the gaps found:

```
Found gap at X=27.5 (width=11 bins)   <- margin
Found gap at X=110.0 (width=12 bins)  <- within left column
Found gap at X=230.0 (width=26 bins)  <- could be gutter or left-column gap
```

## Solution Strategy

1. **Use element width in boundary check**: Instead of just `e.x < boundary`, use `e.x + e.width/2` or center point
2. **Look for the WIDEST gap near center**: The column gutter should be wider than intra-column gaps
3. **Require right column elements to start AFTER gap + margin**: Right column text typically starts at X > 300

## First-Principles Approach

Academic papers have predictable layouts:

- Page width: 612pt (US Letter)
- Left margin: ~55pt
- Right margin: ~55pt
- Column width: ~240pt each
- Gutter: ~20pt at X ≈ 295-315

Algorithm improvement:

1. Find gaps wider than 20pt (gutter width)
2. Gap should be in center 40-60% of page
3. Elements in each column should have consistent left margins

## Files to Modify

1. `column_detection.rs:71-82` - Change element boundary classification
2. Consider element width when classifying left vs right column
