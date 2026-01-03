# OODA Loop 5 - OBSERVE (First Principles Metric Validation)

**Date:** 2026-01-03  
**Directory Scope:** Validator metrics validation  
**Focus:** Direct comparison of generated vs gold markdown to understand TRUE problem

## Validation Results

```
Table Accuracy:      2.4%
Style Accuracy:      31.1%
Composite Score:     32.4/100
```

## Per-Document Table Accuracy

- **2900_Goyal_et_al:** 0.0% (0 tables detected)
- **AlphaEvolve:** 0.3% (false positives)
- **agent_2510:** 0.0% (whitespace tables not detected)
- **ccn_2512:** 0.0%
- **one_tool:** 11.4% (SOME tables working)

## Direct Markdown Comparison (one_tool - 11.4% accuracy)

### Table Line Counts

```bash
Gold:      72 table lines
Generated: 56 table lines
Missing:   16 lines (22%)
```

### First Table Comparison

**GOLD (lines 135-142):**

```markdown
| Agent Pipeline    | Model            | Function-level Recall | Funct Precision | Funct Sample-F1 | Funct IoU | File-level Recall | File Precision | File Sample-F1 | File IoU |
| ----------------- | ---------------- | --------------------- | --------------- | --------------- | --------- | ----------------- | -------------- | -------------- | -------- |
| **Closed-source** |                  |                       |                 |                 |           |                   |                |                |          |
| RepoSearcher      | Claude3.7-Sonnet | 66.80                 | 28.30           | 19.90           | 17.89     | 89.71             | 33.15          | 21.04          | 20.67    |
| RepoNavigator     | Claude3.7-Sonnet | 31.03                 | 31.72           | 34.43           | 30.22     | 72.26             | 75.95          | 73.01          | 71.37    |

...
```

**GENERATED (lines 377-396):**

```markdown
| CoSIL                                                             | Training | Free | 48.61 | 13.40 | 19.81 | 12.12 | 78.35 |
| ----------------------------------------------------------------- | -------- | ---- | ----- | ----- | ----- | ----- | ----- |
| Agentless Training Free 25.20 14.30 16.14 12.28 75.65 19.76 29.88 | 19.30    |      |       |       |       |       |       |
| Orcaloca Training Free 29.92 20.98 22.77 18.92 52.17 52.15 50.93  | 48.72    |      |       |       |       |       |       |

...
```

## First Principles Problem Identification

### Problem 1: Wrong Table Start Row

**Gold starts at:** Line 135 - "| Agent Pipeline | Model | ..."  
**Generated starts at:** Line 377 - "| CoSIL | Training | Free | ..."

**Analysis:** "CoSIL" is row 18 of the gold table (line 152), not the header!

**Root cause:** Table boundary detection started mid-table instead of at the header row.

### Problem 2: Merged Cell Text Not Split

**GOLD (line 138 - row 3 of table):**

```
| RepoSearcher | Claude3.7-Sonnet | 66.80 | 28.30 | 19.90 | 17.89 | 89.71 | 33.15 | 21.04 | 20.67 |
```

10 clean cells.

**GENERATED (line 379 - corresponding row):**

```
| Agentless Training Free 25.20 14.30 16.14 12.28 75.65 19.76 29.88 | 19.30 |  |  |  |  |  |  |
```

First cell contains: `Agentless Training Free 25.20 14.30 16.14 12.28 75.65 19.76 29.88`

**Should be 9 separate cells:**

1. Agentless
2. Training Free
3. 25.20
4. 14.30
5. 16.14
6. 12.28
7. 75.65
8. 19.76
9. 29.88

Plus cell 10: 19.30 (correctly separated)

**Root cause:** PDF has ONE LARGE GRID CELL containing text positioned at multiple X-coordinates. `extract_text_in_rect()` dumps all text into one string instead of recognizing the X-position clusters as separate logical cells.

### Problem 3: False Positives

**GENERATED (end of file):**

```markdown
|Y ∩ *Y*ˆ ∗|
|_Y_ ∗|
| symbol tables. When such circumstances occur, the tool
|Y ∩ *Y*ˆ ∗|
```

Mathematical notation and sentence fragments being detected as table rows.

**Root cause:** Lattice detector finds grid lines around inline equations and text blocks, treating them as tables.

## First Principles Truth

The validator is CORRECT. The generated markdown is fundamentally broken:

1. **Headers missing:** Tables start mid-table instead of at header row
2. **Cell structure broken:** Multi-value cells not split properly
3. **False positives:** Non-table content detected as tables

The 11.4% accuracy for one_tool likely comes from:

- A few small tables that happen to be correctly extracted
- Partial credit for detecting table regions even if content is wrong
- The validation metric averaging across all detected tables

## Why Previous Loops Failed

### Loop 1 (crossing_ratio):

- Fixed grid detection sensitivity
- Helped detect more tables
- But didn't fix cell text splitting → no accuracy improvement

### Loop 2 (containment):

- Fixed which characters go in which cells
- But didn't fix cell boundary detection → no accuracy improvement

### Loop 3 (decorative filter):

- Cleaned cell content
- But didn't fix cell splitting or boundaries → no accuracy improvement

### Loop 4 (clustering):

- Attempted to infer columns from text positions
- But applied at wrong level (table-level instead of cell-level) → no accuracy improvement

## Root Cause (First Principles)

**The fundamental problem is PDF table structure ambiguity:**

### Type A: Well-structured tables

- Grid lines at every column/row boundary
- One logical cell = one grid cell
- Text within each cell is coherent
- **These work with current code (the 11.4% success)**

### Type B: Merged-cell tables (one_tool main table)

- Grid lines exist but span multiple logical columns
- One grid cell contains multiple values at different X-positions
- **These fail completely - this is the majority**

### Type C: Whitespace tables (agent_2510)

- No grid lines at all
- Columns defined by text alignment only
- **These are completely invisible to lattice detector**

## Next Steps (True Fix)

### For Type B (Merged Cells) - HIGH PRIORITY

**Modify `extract_text_in_rect()` to detect and split merged cells:**

1. Get all text elements in cell bbox
2. Cluster by X-position (epsilon ~20pt)
3. If multiple clusters → cell is merged
4. Return Vec<String> (one per cluster) instead of String
5. Update table building to handle variable column counts per row

**Expected impact:** Fix one_tool from 11.4% → 40-50%

### For Type C (Whitespace) - MEDIUM PRIORITY

**Already attempted in Loop 4 but wrong approach:**

- Clustering works but only when no grid lines
- Need to handle full whitespace tables (agent_2510)
- Current code path: lines 255-280 already does this
- **Why failing:** Clustering still doesn't work well enough

### For False Positives - LOW PRIORITY

**Add heuristics to reject non-table grids:**

- Check if cells contain equation-like content
- Check if "table" is single column with prose
- Increase minimum row/column requirements

## Conclusion

**The 2.4% table accuracy is accurate.** Most tables are fundamentally broken due to merged cell issues. The validator correctly identifies that cell content doesn't match gold standard.

**Next loop must fix merged cell splitting.**
