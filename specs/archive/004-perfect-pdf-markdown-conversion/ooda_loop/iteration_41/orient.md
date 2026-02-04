# OODA-41: Orient - Analysis of Algorithm Gap

## Date: 2026-02-04

## Root Cause Analysis

The F1 gap of 21.4% is primarily caused by **reading order errors** in two-column documents.

### Why pymupdf4llm Works Better

1. **Block-level processing**: Works with pre-computed text blocks from MuPDF's native layout engine, not raw characters

2. **Smart sort key in Phase 3**: The key insight is this algorithm:

   ```python
   for box in new_rects:
       # Find left-most rect that overlaps vertically with this box
       left_rects = [r for r in new_rects if r.x1 < box.x0 and (box.y0 <= r.y0 <= box.y1 or box.y0 <= r.y1 <= box.y1)]
       if left_rects:
           key = (left_rects[-1].y0, box.x0)  # Use left rect's Y for sort
       else:
           key = (box.y0, box.x0)  # Original position
   ```

   This ensures that when there's a left column block at the same vertical position, the RIGHT column block uses the LEFT block's Y for sorting, preventing right-column text from appearing before left-column text at the same vertical level.

3. **Boundary normalization**: By aligning x0/x1 within 3pt, blocks in the same column are grouped correctly even if their edges vary slightly.

### Our Current Weaknesses

1. **Character-level extraction**: We extract characters and group them, leading to fragmentation
2. **No left-rect lookup**: We sort by absolute (y, x) without considering neighboring blocks
3. **Threshold-based column detection**: DBSCAN clustering is good but doesn't normalize boundaries

## Risk Assessment

| Change                             | Risk   | Mitigation                    |
| ---------------------------------- | ------ | ----------------------------- |
| Add Phase 2 boundary normalization | Low    | Unit test with exact values   |
| Add Phase 3 smart sort key         | Medium | May affect single-column docs |
| Refactor reading_order.rs          | Medium | Keep existing tests passing   |

## First Principles Decision

The pymupdf4llm algorithm uses a **block-first, then sort** approach:

```
Characters → Blocks (by native engine) → Sort blocks → Read in order
```

Our approach:

```
Characters → Elements → Lines → Blocks → Sort → Read
```

The additional steps cause information loss. However, we can adopt the **smart sort key** from Phase 3 without changing our extraction pipeline.

## Implementation Strategy

1. **Don't change extraction pipeline** - too risky
2. **Add post-processing step** after blocks are built
3. **Implement smart sort key** for reading order
4. **Add boundary normalization** for column detection

## Expected Impact

- `01_2512.25075v1.pdf`: F1 0.552 → 0.75+ (estimated)
- Average F1: 0.686 → 0.80+ (estimated)

Conservative estimate because we're not changing the extraction fundamentals, just the ordering.
