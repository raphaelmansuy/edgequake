# OODA Loop 1 - ORIENT

## Directory: `crates/edgequake-pdf/src/backend`

## Root Cause Analysis

### Problem: Table Accuracy at 2.4% (Baseline)

**First Principles Analysis:**

PDFs encode tables in two ways:

1. **Explicit Lattice:** Graphical lines (horizontal/vertical) that form cell boundaries
2. **Implicit Layout:** Text positioning without visible lines (whitespace-based columns)

Current `lattice.rs` implementation:

- ✅ Detects explicit lattice tables using connected components of graphical lines
- ✅ Groups text elements by cell boundaries
- ❌ **FAILS**: Uses overly strict `crossing_ratio > 0.1` threshold (line 360)

### Why Current Approach Fails

**Problem:** The crossing_ratio heuristic assumes text elements (words) should never cross column boundaries. This is WRONG because:

1. **Word-level extraction:** PDF text extraction produces word-level `TextElement` objects. A single multi-word cell content gets broken into multiple elements.

2. **Multi-line cells:** Cells with wrapped text have elements on different Y coordinates. If cell boundaries are detected between lines within the same cell, the "crossing" check falsely rejects the table.

3. **Font width estimation:** Code uses `elem.font_size * 0.5` to estimate character width (line 336), which is approximate. Real character widths vary significantly, leading to incorrect crossing detection.

### Mathematical Foundation (First Principles)

A **table cell** is defined by:

- Bounding box: `[x1, y1, x2, y2]`
- Contains text elements where:  
  `x1 ≤ elem.x ≤ x2` AND `y1 ≤ elem.y ≤ y2`

Current crossing check is GEOMETRIC (element.right > boundary.left), but doesn't account for:

- Cell **membership** (which cell does this element belong to?)
- Cell **merging** (spanning multiple columns/rows)
- **Alignment tolerance** for hand-drawn tables

### Evidence from Logs

```
crossing_ratio=0.12 (2/16)   → Rejected (only 2 out of 16 elements "cross")
crossing_ratio=0.40 (20/50)  → Rejected (20 out of 50 cross)
crossing_ratio=0.85 (34/40)  → Rejected (massive crossing)
```

**Hypothesis:** Most rejections are FALSE POSITIVES where legitimate multi-word or multi-line cell content is being flagged as "crossing" column boundaries.

## Key Insights

1. **Threshold too strict:** 0.1 (10%) is too low. Real tables with multi-line cells routinely have 20-40% apparent "crossing" due to word-level granularity.

2. **Missing cell clustering:** Instead of checking raw geometric crossing, should cluster text elements into cells FIRST, then validate table structure.

3. **No cell span detection:** Code assumes 1 element = 1 cell, ignoring merged cells that span multiple columns.

## Proposed Solutions (Ranked by Impact)

### Solution A: Relax crossing_ratio threshold (QUICK WIN)

- Change from `0.1` to `0.3` or `0.4`
- Pros: 1-line change, immediate improvement
- Cons: Still fundamentally flawed approach
- **Predicted Score Impact:** +10-15 points (Table Accuracy: 2.4% → 15-20%)

### Solution B: Cell-based clustering (CORRECT APPROACH)

- Replace geometric crossing check with cell assignment algorithm:
  1. For each text element, assign to closest cell center
  2. Check if cell has reasonable text density
  3. Validate table structure by checking cell occupancy patterns
- Pros: Mathematically sound, handles multi-line cells
- Cons: More complex, requires cell clustering implementation
- **Predicted Score Impact:** +25-35 points (Table Accuracy: 2.4% → 40-60%)

### Solution C: Add cell span detection (COMPREHENSIVE)

- Detect merged cells by analyzing text bounding boxes vs. grid structure
- Adjust cell extraction to handle colspan/rowspan
- Pros: Handles complex tables with merged cells
- Cons: Significant implementation effort
- **Predicted Score Impact:** +35-45 points (Table Accuracy: 2.4% → 60-80%)

## Decision Criteria

Following OODA spec: "Select the smallest patch expected to improve the score"

**→ Choose Solution A for OODA Loop 1**

Reasoning:

- Minimal change (1 line)
- Quick validation cycle
- Establishes measurement baseline
- Can iterate to B/C in subsequent loops

## Next: DECIDE phase
