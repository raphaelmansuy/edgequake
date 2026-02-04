# OODA-07 Orient: Fix Y-Coordinate for Smart Sort

## Analysis

### pymupdf4llm Algorithm (multi_column.py:319-323)
```python
if left_rects:
    key = (left_rects[-1].y0, box.x0)  # y0 = TOP of rect in PyMuPDF
else:
    key = (box.y0, box.x0)
```

The algorithm sorts by:
1. Y position of the leftmost overlapping block (to read left column first)
2. X position of current block (secondary sort)

### Current Bug

We use `left_block.y0` but in PDFium coords, `y0` is the BOTTOM of the rect.
This causes incorrect sorting because blocks at the top of the page get lower sort keys.

### Fix Strategy

Replace `y0` with `y1` in the sort key calculation:
- `left_block.y1` = top of the left block (correct for PDFium)
- `block.y1` = top of current block

Also need to verify the sort direction:
- PDFium: Higher Y = higher on page (y=0 at bottom)
- We want: Higher on page = comes first in output

Current code inverts Y: `-y_key` which is correct for sorting top-to-bottom.

### Expected Impact
- Proper left-to-right, top-to-bottom reading order
- ROUGE-L should improve significantly (0.70 → 0.80+)
