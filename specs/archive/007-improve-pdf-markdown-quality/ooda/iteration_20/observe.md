# OODA Iteration 20 - Observe

## Problem: Table PDFs Misclassified as Two-Column Layouts

### Evidence

Converting `004_simple_table_2x3.pdf` produces:

```markdown
# **Simple Table**

This document contains a simple 2-column by 3-row table:

## **Name**

Alice
Charlie
25 35
```

Expected: A proper markdown table with headers and rows.

### Tracing Output (Key Findings)

```
Zone detection X-range: min=78.0, max=372.4, left_zone_end=275.4, right_zone_start=293.8
Zone counts: left=6, gap=0, right=4 (total=10)
Detected TWO-COLUMN layout with boundary at 299.9
```

The column detection classified the page as two-column because:

- Left zone (x < 275): 6 elements (title, description, Name, Alice, Bob, Charlie)
- Right zone (x > 294): 4 elements (Age, 25, 30, 35)
- Balance ratio: 4/6 = 0.67 (passes the > 0.15 threshold)

### Block Layout (BEFORE processing)

```
block 0: 'Simple Table'  [250,0,369,18]  ← title (spanning)
block 1: 'This document...'  [78,34,386,44]  ← description (wide)
block 2: 'Name'           [218,66,244,78]  ← table header col1
block 3: 'Age'            [367,66,386,78]  ← table header col2
block 4: 'Alice'          [223,91,251,101] ← table cell
block 5: 'Bob'            [225,109,242,119]← table cell
block 6: '30'             [372,109,383,119]← table cell
block 7: 'Charlie'        [218,127,257,137]← table cell
block 8: '25'             [372,91,383,101] ← table cell
block 9: '35'             [372,127,383,137]← table cell
```

### Pipeline Impact

1. Column detection returns TWO-COLUMN → page gets `columns.len() == 2`
2. OODA-34: TableDetectionProcessor SKIPS pages with > 1 column
3. OODA-16: Even the "strict mode" fallback runs but...
4. Wait - reading more carefully: OODA-34 skips BUT OODA-16 re-enables in strict mode
5. BUT the blocks arrive already reordered by column reading order:
   - Left column blocks first: Name, Alice, Bob, Charlie
   - Right column blocks: Age, 25, 30, 35
   - This DESTROYS the table's row alignment

### Root Cause

The column detection in `column_detection.rs` cannot distinguish between:

- **Two-column text layout**: Long text flowing in separate columns
- **Table grid layout**: Short cells arranged in rows and columns

Both patterns have elements in left and right zones, but tables have:

- Very short text per "column" element (typically < 15 characters)
- Precise Y-alignment between left and right elements (same row)
- Grid-like structure (consistent number of items per column)
