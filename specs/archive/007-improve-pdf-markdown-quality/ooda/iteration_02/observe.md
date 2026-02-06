# OODA Iteration 02 - Observe

**Mission**: Improve PDF-to-Markdown Conversion Quality
**Mission File**: `specs/007-improve-pdf-markdown-quality.md` (RE-READ ✓)
**Date**: 2026-02-05

---

## 1. Current State After Iteration 01

### 1.1 Changes Made

- Added header_margin, footer_margin, page_height to GroupingParams
- Implemented filtering in chars_to_spans()
- Added 3 unit tests
- Tests: 497 passing

### 1.2 Outstanding Issues from Observation

From test output (AI_Services\_\_Elitizon.pdf):

1. **Text fragmentation** - Still present
2. **Reading order** - Content out of sequence
3. **List markers missing** - Bullet detection weak
4. **Column merging artifacts** - Two columns concatenated

---

## 2. Deep Dive: Reading Order Algorithm

### 2.1 Current Reading Order Implementation

**File**: `edgequake/crates/edgequake-pdf/src/layout/reading_order.rs`

Analyzed reading order algorithm and compared with PyMuPDF4LLM's `multi_column.py`:

### 2.2 PyMuPDF4LLM's join_rects_phase3 (lines 283-311)

The key insight:

```python
# Sorting approach guided by:
# 1. Extraction should start with block whose top-left is left-most and top-most
# 2. Blocks further right should be extracted later - even if top-left is higher
# 3. Sort using tuple (y, x) where y is NOT smaller than left-most block with vertical overlap
# 4. Example: if block Q is to the right of P with vertical overlap,
#    Q's sort key = (P.y, Q.x) - ensuring P comes before Q

for box in new_rects:
    # Find left-most rect that overlaps vertically
    left_rects = sorted([r for r in new_rects
                         if r.x1 < box.x0
                         and (box.y0 <= r.y0 <= box.y1 or box.y0 <= r.y1 <= box.y1)],
                        key=lambda r: r.x1)
    if left_rects:
        key = (left_rects[-1].y0, box.x0)  # Use left block's Y
    else:
        key = (box.y0, box.x0)  # Original position
```

### 2.3 Our Current Implementation

`compute_smart_sort_key()` in reading_order.rs:

1. Finds blocks to the LEFT (correct)
2. **BUG**: Uses `max_by(x2)` to find the RIGHT-MOST left block (should be LEFT-MOST per PyMuPDF4LLM)
3. Only applies within columns, not for final merge

`merge_column_orders_with_footer_smart()`:

- Processes columns SEQUENTIALLY (left-to-right, all of column 1 then all of column 2)
- Does NOT apply Phase 3 smart sorting to the final result
- Headers/spanning elements may not be properly interleaved

### 2.4 Gap Analysis

| Aspect            | PyMuPDF4LLM             | Our Implementation        |
| ----------------- | ----------------------- | ------------------------- |
| Phase 3 sort      | Applied to ALL blocks   | Only within columns       |
| Left-block finder | Uses left-most (min x1) | Uses right-most (max x2)  |
| Vertical overlap  | Checks y0 OR y1 overlap | Checks full range overlap |
| Final ordering    | Global smart sort       | Sequential column merge   |

### 2.5 Test Baseline

```
cargo test --lib reading_order -- 497 tests passing
```
