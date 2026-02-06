# Iteration 07: OBSERVE - Multi-Column Layout Analysis

**Date:** 2025-02-05
**Focus:** Three-column layout support (mission critical priority: 60→85)

## Current Implementation Analysis

### `pymupdf_grouper.rs::detect_columns()` (lines 532-607)

The current algorithm only supports **two columns**:

```rust
// Line 603-604: Only returns TWO columns
vec![(page_left, gutter_center), (gutter_center, page_right)]
```

**Algorithm:**

1. Finds ONE gutter near page center (±20%)
2. Splits page into left/right at gutter
3. Cannot handle 3+ columns

### PyMuPDF4LLM Approach (multi_column.py)

PyMuPDF4LLM doesn't detect fixed column count. Instead:

1. Extract text blocks
2. Join blocks that don't cross gutters
3. Smart reading order sort using "left-most overlapping block"

Key insight from line 284-309:

```python
# For each block Q, find left-most block P with vertical overlap
# Sort key = (P.y0, Q.x0) → ensures Q comes after P
```

## Gap Analysis

| Feature    | Current   | PyMuPDF4LLM | Gap      |
| ---------- | --------- | ----------- | -------- |
| 2-column   | ✅ Works  | ✅ Works    | None     |
| 3-column   | ❌ Broken | ✅ Works    | Critical |
| 4+-column  | ❌ Broken | ✅ Works    | Critical |
| Mixed cols | ❌ Broken | ✅ Works    | Critical |

## Three-Column Gold Standard

From `test-data/gold/07-multi-column/003_three_column_simple.md`:

```markdown
Left column text. This is content in the leftmost column...
Middle column text starts. This is the middle column...
Right column text. This is the rightmost column...
```

Expected reading order: Left → Middle → Right (top to bottom within each)

## Root Cause

The `detect_columns()` function searches for ONE gutter near center. For 3 columns:

- Page divided into thirds (33%, 66%)
- Two gutters needed: ~33% and ~66%
- Current search only finds center (~50%) → misses both gutters

## Proposed Fix Strategy

### Option A: Multiple Gutter Detection

Scan for ALL gutters (not just center):

1. Build histogram of line left/right edges
2. Find valleys (gaps) in histogram
3. Each valley = potential gutter
4. Return N+1 columns for N gutters

### Option B: Block-Based (PyMuPDF4LLM style)

Don't detect columns explicitly:

1. Group text into blocks based on proximity
2. Never join blocks across large horizontal gaps
3. Sort using "left-most overlapping" algorithm

Option A is simpler and maintains current architecture. Recommend implementing Option A.
