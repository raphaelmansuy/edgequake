# OODA Iteration 04 - Observe

## Date: 2026-02-04

## Re-read Mission File ✅

Confirmed reading of `specs/005-perfect-pdf-pymupdf4llm-inspired-conversion.md` at start of this iteration.

---

## Current Status

### Quality Metrics (from Iteration 03)

| Metric            | Current | Target  | Gap    |
| ----------------- | ------- | ------- | ------ |
| **Quality Score** | 0.573   | >= 0.95 | -0.377 |
| ROUGE-L (order)   | 0.491   | >= 0.90 | -0.409 |
| Word F1 (content) | 0.914   | >= 0.95 | -0.036 |
| Structure Score   | 0.295   | >= 0.80 | -0.505 |
| Format Score      | 0.312   | >= 0.70 | -0.388 |

**Key Insight**: ROUGE-L (order) is the biggest problem. Content accuracy (Word F1 = 0.914) is already high.

---

## Code Analysis

### 1. Line Tolerance Settings

**File**: `edgequake/crates/edgequake-pdf/src/layout/pymupdf_grouper.rs`
**Line 34**: `line_tolerance: 5.0`

```rust
impl Default for GroupingParams {
    fn default() -> Self {
        Self {
            // WHY: Increased from 3pt to 5pt to handle font style variations
            line_tolerance: 5.0,  // <-- SHOULD BE 3.0 per pymupdf4llm
            block_gap: 10.0,
            column_overlap: 0.5,
        }
    }
}
```

**Problem**: pymupdf4llm uses `tolerance=3` in `get_raw_lines()` at line 30.
The 5.0 value was a workaround that may be merging lines incorrectly.

### 2. Reading Order Algorithm

**File**: `edgequake/crates/edgequake-pdf/src/layout/reading_order.rs`
**Lines 95-121**: `single_column_order()`

Current algorithm:

```rust
// Sort by Y position (top to bottom), then X (left to right)
indices.sort_by(|&a, &b| {
    let y_a = (bbox_a.y1 / self.line_tolerance).floor();
    let y_b = (bbox_b.y1 / self.line_tolerance).floor();
    if y_a != y_b {
        y_a.partial_cmp(&y_b).unwrap()  // ASCENDING Y
    } else {
        bbox_a.x1.partial_cmp(&bbox_b.x1).unwrap()  // left to right
    }
});
```

**Problem**: This is a simple Y-then-X sort. It doesn't implement the **smart sort key** algorithm from pymupdf4llm `join_rects_phase3()`.

### 3. Smart Sort Key Algorithm (from pymupdf4llm)

**File**: `zz-explore/pymupdf4llm/pymupdf4llm/pymupdf4llm/helpers/multi_column.py`
**Lines 280-320**:

```python
"""
Sorting approach:
1. Extraction should start with the block whose top-left corner is the
   left-most and top-most.
2. Any blocks further to the right should be extracted later - even if
   their top-left corner is higher up on the page.
3. Sort key for block Q = (P.y0, Q.x0), where P is the left-most block
   with vertical overlap.

         Q +---------+
           | next is |
 P +-------+   |  this   |
   | left  |   |  block  |
   | block |   +---------+
   +-------+
"""
for box in new_rects:
    left_rects = sorted(
        [r for r in new_rects
         if r.x1 < box.x0  # strictly to the left
         and (box.y0 <= r.y0 <= box.y1 or box.y0 <= r.y1 <= box.y1)],  # vertical overlap
        key=lambda r: r.x1,
    )
    if left_rects:
        key = (left_rects[-1].y0, box.x0)  # use P's y0, Q's x0
    else:
        key = (box.y0, box.x0)  # default
```

**This is the missing algorithm!** We need to implement this in Rust.

### 4. Multi-column Order

**File**: `edgequake/crates/edgequake-pdf/src/layout/reading_order.rs`
**Lines 125-230**: `multi_column_order()`

The current implementation assigns blocks to columns and processes column by column.
This is a simpler approach than pymupdf4llm's smart sort key.

---

## Reference: Join Phases from pymupdf4llm

### Phase 1: Vertical Join (lines 192-206)

```python
delta = (0, 0, 0, 10)  # allow 10pt gap below
for rect pair:
    if (rect0 + delta) intersects rect1:
        join rects
```

### Phase 2: Boundary Normalization (lines 208-238)

```python
for rect:
    x0 = min([r.x0 for r in rects if |r.x0 - rect.x0| <= 3])
    x1 = max([r.x1 for r in rects if |r.x1 - rect.x1| <= 3])
```

### Phase 3: Smart Sort (lines 240-320)

```python
for box:
    left_rects = [r for r in rects if r.x1 < box.x0 and vertical_overlap(r, box)]
    if left_rects:
        key = (left_rects[-1].y0, box.x0)
    else:
        key = (box.y0, box.x0)
```

---

## Observations Summary

1. **Line tolerance too high**: 5pt vs 3pt (pymupdf4llm default)
2. **Missing smart sort algorithm**: We use simple Y-then-X instead of P-overlap key
3. **Missing join phases**: Phase 1, 2, 3 from multi_column.py not implemented
4. **Content is mostly correct**: Word F1 = 0.914 means we extract the right words
5. **Order is broken**: ROUGE-L = 0.491 means words are scrambled

---

## Files to Modify

| File                        | Change                               |
| --------------------------- | ------------------------------------ |
| `layout/pymupdf_grouper.rs` | Change `line_tolerance: 5.0` → `3.0` |
| `layout/reading_order.rs`   | Add smart sort key algorithm         |
| `layout/mod.rs`             | Wire up new algorithm                |
