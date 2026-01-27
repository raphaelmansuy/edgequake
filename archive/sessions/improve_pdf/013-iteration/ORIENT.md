# Loop 013 - ORIENT Phase

## Problem Statement

Loop 012 successfully fixed table column detection using DBSCAN geometric clustering (2 → 13 columns detected). However, Table Accuracy remains at 2.4% because cell content extraction is completely broken. Text from outside tables, adjacent cells, and even entire paragraphs are being incorrectly assigned to table cells.

## Root Cause Analysis

### Current Implementation: `extract_text_in_rect()` (lines 505-542)

```rust
fn extract_text_in_rect(
    &self,
    text_elements: &[TextElement],
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
) -> String {
    let mut contained: Vec<&TextElement> = text_elements
        .iter()
        .filter(|elem| {
            let cx = elem.x;  // ❌ Using center point, not bbox
            let cy = elem.y;
            let tol = 2.0;  // ❌ ±2pt tolerance = 4pt total slop
            cx >= min_x - tol && cx <= max_x + tol &&
            cy >= min_y - tol && cy <= max_y + tol
        })
        .collect();

    // ❌ 5pt Y-binning merges vertically adjacent elements
    contained.sort_by(|a, b| {
        let row_a = (a.y / 5.0).round() as i32;
        let row_b = (b.y / 5.0).round() as i32;
        // ...
    });
    // ...
}
```

### Three Critical Flaws

#### 1. **±2pt Tolerance Creates 4pt Slop Zone**

**Impact:** Text elements with centers up to 2pt outside the cell boundary are included.

**Concrete Example:**

```
Cell boundary: x ∈ [100, 200], y ∈ [50, 70]
Text element: x=98, y=68 (center 2pt left of boundary)
Result: INCLUDED (should be EXCLUDED)

Cell boundary: x ∈ [100, 200], y ∈ [50, 70]
Adjacent cell text: x=202, y=68 (2pt right of boundary)
Result: INCLUDED (should be in next cell!)
```

**Real-world consequence:** "Agent Pipeline" from column 1 + "Model" from column 2 → "Agent Pipeline Model" in one cell

#### 2. **Using Center Point Instead of Bounding Box**

**Impact:** Text elements whose centers are inside the cell but whose rendered glyphs extend outside are included, and vice versa.

**Concrete Example:**

```
Text "Performance" with wide glyphs:
- elem.x (center): 150
- elem.bbox: x0=120, x1=180 (60pt wide)

Cell boundary: x ∈ [140, 160] (20pt wide)
Center check: 150 ∈ [140, 160] → INCLUDED
Reality: Text extends 140-120=20pt left, 180-160=20pt right → OVERLAPS 2 CELLS!
```

**Real-world consequence:** Long words like "Reinforcement Learning" span multiple columns, get assigned to wrong cell

#### 3. **5pt Y-Binning Merges Rows**

**Impact:** Text elements within 5pt vertically are treated as same row, causing vertical spillover.

**Concrete Example:**

```
Row 1: y ∈ [100, 110]
Row 2: y ∈ [90, 100]

Text in Row 1: y=102 → bin = (102/5).round() = 20
Text in Row 2: y=98  → bin = (98/5).round()  = 20
Result: Same bin! Both texts assigned to same cell despite being in different rows.
```

**Real-world consequence:** Multi-line table cells get content from adjacent rows merged together

### Architecture Context

**Table Extraction Pipeline:**

```
detect_tables()
  ↓
create_table_block()
  ├─ Identify grid lines (horizontal/vertical)
  ├─ Extract Y coords (rows) ← WORKS ✅
  ├─ Extract X coords (cols) or detect_columns_by_clustering() ← WORKS ✅
  ├─ Build grid: rows × columns ← WORKS ✅
  └─ For each cell: extract_text_in_rect() ← BROKEN ❌
```

**What's Working:**

- Grid structure detection: rows and columns correctly identified
- Column clustering (Loop 012): 2 → 13 columns, matches gold standard
- Row boundaries: Extracted from horizontal lines in PDF

**What's Broken:**

- Cell text assignment: extract_text_in_rect() includes wrong text

### Evidence from Real Data

**Document:** `one_tool_2512.20957v2.mdf.gen`

**Generated Output (Line 3):**

```
| Locating the files and functions requiring modifi- cation in large open-source software (OSS) repos- itories is challenging due to their scale and struc- tural complexity... [ENTIRE ABSTRACT IN ONE CELL] |
```

**Gold Standard (Lines 135-141):**

```
| Agent Pipeline | Model | Function-level Recall | Funct Precision | ... |
| --- | --- | --- | --- | --- |
| RepoSearcher | Claude3.7-Sonnet | 66.80 | 28.30 | ... |
```

**Analysis:**

- Generated: 13 columns detected (correct structure!) but cell [0][0] contains entire abstract
- Gold: 10 columns with clean cell values
- **Hypothesis:** Abstract text is near table Y-coordinates, within ±2pt + 5pt binning → gets assigned to first cell
- **Validation:** Abstract appears BEFORE table in PDF, but Y-coordinates overlap due to loose tolerance

### Quantitative Impact Assessment

**Current Metrics:**

- Table Accuracy: 2.4% (140 table:mismatch drifts)
- Best document: 11.4% (one_tool_2512.20957v2)
- Worst documents: 0.0% (60% of test set)

**Expected Improvement with Fix:**

- Eliminate text spillover: +5-8% table accuracy
- Fix cell boundary precision: +5-7% table accuracy
- Remove Y-binning artifacts: +2-4% table accuracy

**Target:** 15-20% table accuracy (6-8× improvement)
**Composite Impact:** +5-7 composite points (+15-22% overall)

## First Principles Analysis

### What Should Cell Text Extraction Do?

**Goal:** Assign each text element to exactly ONE cell in the table grid, based on geometric containment.

**Constraints:**

1. **Exclusivity:** Text belongs to one cell only (no duplicates)
2. **Precision:** Text fully contained within cell boundaries → include
3. **Completeness:** All text within table bbox → assigned to some cell
4. **Robustness:** Handle minor PDF rendering variations (±0.5pt tolerance MAX)

**Correct Algorithm:**

```
For each cell (row_i, col_j):
  cell_bbox = [unique_x[j], unique_y[i+1], unique_x[j+1], unique_y[i]]

  For each text_element in text_elements:
    If text_element.bbox FULLY CONTAINED IN cell_bbox (with minimal tolerance):
      Assign text_element to cell
    End If
  End For
End For
```

**Key Differences from Current Implementation:**

1. Use text_element.bbox for containment check, not center point
2. Tight tolerance: 0.5pt instead of 2.0pt
3. No Y-binning: Use actual Y-coordinates from grid
4. Spatial containment only: No heuristic row/column merging

### Why Current Implementation Fails This Test

| Principle        | Current Code                   | Consequence                      |
| ---------------- | ------------------------------ | -------------------------------- |
| **Exclusivity**  | ±2pt tolerance allows overlaps | Text in multiple cells           |
| **Precision**    | Uses center point, not bbox    | Wide text spans multiple cells   |
| **Completeness** | Y-binning merges rows          | Text from adjacent rows mixed    |
| **Robustness**   | 2.0pt tolerance too large      | Text from outside table included |

## Design Constraints

### PDF Coordinate System Quirks

1. **Y-axis increases upward:** top > bottom in coordinate space
2. **TextElement fields:**
   - `elem.x`, `elem.y`: Center point of text
   - `elem.bbox`: BoundingBox with x0, y0, x1, y1 (full glyph extent)
3. **Grid coordinates:**
   - `unique_y[i]` > `unique_y[i+1]`: Row from top (i) to bottom (i+1)
   - `unique_x[j]` < `unique_x[j+1]`: Column from left (j) to right (j+1)

### Required Tolerance Analysis

**Why tolerance is needed:**

- PDF rendering imprecision: Line at y=100 might be y=100.2
- Font positioning: Baseline vs glyph top varies
- Grid construction: Horizontal lines might not perfectly align with text baselines

**How much tolerance:**

- **0.5pt:** Safe for PDF rounding errors (~0.01 inch at 72 DPI)
- **1.0pt:** Conservative for font variations
- **2.0pt:** TOO LARGE - overlaps adjacent cells in compact tables

**Recommendation:** 0.5pt for bbox containment, 1.0pt for grid coordinate matching

## Comparison with Gold Standard Behavior

### What gold files show us:

**Example Table (one_tool_2512.20957v2.gold.md, line 135):**

```
| Agent Pipeline | Model | Function-level Recall | Funct Precision | ...
```

**Cell characteristics:**

- Single value per cell (no multi-word spanning issues)
- Clean boundaries (no text from adjacent cells)
- Correct vertical alignment (no row mixing)
- Proper column assignment (matches visual layout)

**Implication:** Gold markdown was created by a tool/human that:

1. Accurately identified cell boundaries
2. Assigned text based on strict geometric containment
3. Did not merge adjacent text based on proximity heuristics

**Our goal:** Match this behavior algorithmically using first principles geometry

## Next Steps

The path forward is clear:

1. **Rewrite extract_text_in_rect():**

   - Use elem.bbox for containment, not center point
   - Reduce tolerance to 0.5pt
   - Remove Y-binning (use actual grid coordinates)
   - Implement strict geometric containment test

2. **Add robust cell-text assignment:**

   - Check if text bbox overlaps multiple cells → assign to cell with largest overlap
   - Handle edge cases: text exactly on boundary → assign to left/top cell
   - Filter out text completely outside table bbox

3. **Validation:**
   - Test on one_tool_2512.20957v2: Should improve from 11.4% to 20%+
   - Test on zero-score documents: Should detect at least some table content
   - Run full validator: Target 15-20% table accuracy, 37-40 composite score

**Next Phase:** DECIDE - Design the improved algorithm with detailed implementation spec
