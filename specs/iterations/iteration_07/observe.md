# OODA-07 Observe: Coordinate System Mismatch

## Current State

- Quality: 0.724 (target ≥0.95)
- ROUGE-L: 0.701 (target ≥0.90)

## Problem: Text Interleaving

Example from v2_2512.25072v1.md:

```
Line 21: ## I. INTRODUCTION
Line 23: Humanoid robots have the potential...  (LEFT COLUMN START)
Line 25: , Jitendra Malik                        ← WRONG! Should be in header
Line 27: manipulate objects [1]...               ← FROM RIGHT COLUMN
Line 31: succeed in such settings...             ← LEFT COLUMN (should be before 27)
```

## Root Cause: Y-Coordinate Mismatch

### PyMuPDF Coordinate System (origin at TOP-LEFT)

- `y0` = TOP of rect (smaller number = higher on page)
- `y1` = BOTTOM of rect

### PDFium Coordinate System (origin at BOTTOM-LEFT)

- `y0` = BOTTOM of rect (smaller number = lower on page)
- `y1` = TOP of rect (larger number = higher on page)

### The Bug in `compute_smart_sort_key`

Current code (pymupdf_grouper.rs:536-538):

```rust
// Use left block's top Y as the sort key Y
left_block.y0 as i32  // BUG: y0 is BOTTOM, not TOP!
```

Should be:

```rust
left_block.y1 as i32  // CORRECT: y1 is TOP in PDFium coords
```

### Also affects `has_vertical_overlap`

Current (line 558-560):

```rust
(b.y0 <= a.y0 && a.y0 <= b.y1) || (b.y0 <= a.y1 && a.y1 <= b.y1)
```

This might be correct since we're checking if points fall within range, regardless of which end.

## Files to Modify

- `layout/pymupdf_grouper.rs` - `compute_smart_sort_key` function
