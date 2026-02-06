# OODA Iteration 20 - Act

## Changes Made

### 1. Table-vs-Column Discriminator Function
**File:** `src/backend/column_detection.rs` (new function `looks_like_table_not_columns`)

Added a first-principles discriminator that detects when detected "two-column" layout
is actually a table grid:

```text
┌─────────────────────────────────────────────────────────────┐
│  TWO-COLUMN TEXT              TABLE GRID                    │
├─────────────────────────────────────────────────────────────┤
│  Long text (>15ch avg)        Short text (<15ch avg)        │
│  Independent Y flow           Precise Y-alignment (>60%)    │
└─────────────────────────────────────────────────────────────┘
```

Algorithm:
1. Filter out wide-spanning elements (width > 50% page)
2. Separate remaining by boundary into left/right
3. Compute avg text length for both
4. Count Y-aligned pairs (5pt tolerance)
5. If BOTH avg < 15 chars AND Y-alignment > 60% → table, not columns

### 2. Integrated Into All Detection Paths
**File:** `src/backend/column_detection.rs`

Added discriminator check to ALL five two-column detection paths:
- Peak detection (line ~138)
- Gap detection (line ~191)
- Bottom-only gap detection (line ~276)
- Zone-based detection (line ~358)
- arXiv fallback (line ~386)

### 3. Single-Column Signal Propagation
**File:** `src/backend/extraction_engine.rs` (line ~243)

When backend detects single-column (including table override), set `page.columns`
to a single full-page column instead of empty Vec. This prevents the LayoutProcessor
from re-detecting columns with DBSCAN (which would override the table decision).

### 4. Updated Existing Tests
**File:** `src/backend/column_detection.rs`

Updated `test_detect_two_columns` and `test_detect_columns_wide_page` to use
realistic text lengths (40+ chars) instead of short words ("left", "right").
The previous tests were unrealistic — real two-column text has paragraphs, not
single words.

### 5. New Tests (3 added)
- `test_table_grid_not_detected_as_columns` — Simulates 2×3 table → no column detection
- `test_real_columns_not_detected_as_table` — Real column paragraphs → columns detected
- `test_table_discriminator_requires_both_conditions` — Short text without Y-alignment → not table

## Test Results
```
test result: ok. 569 passed; 0 failed; 0 ignored; 0 measured
```

## Quality Improvement Evidence

### Simple Table PDF (004_simple_table_2x3.pdf)

**Before (IT19):** 0 tables detected, cells rendered as plain text
```
## **Name**
Alice
Charlie
25 35
```

**After (IT20):** 1 table detected, partial markdown table
```
## **Name**
| Alice | 25 |
| --- | --- |
| 30 |
| Charlie | 35 |
```

Table detection now WORKS! Header row still stolen by heading detection (future fix).

### Elitizon Business Document

**Before:** Empty headings (Typical use cases, Key outputs had no content)
**After:** Content now correctly follows headings

### Two-Column Paper (lightrag) — No Regression
Output unchanged, two-column detection still works correctly.

## Commit
```
OODA-IT20: Table-vs-column discriminator prevents tables from being misclassified as columns
```
