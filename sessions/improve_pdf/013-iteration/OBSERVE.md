# Loop 013 - OBSERVE Phase

## Test Results

```
Total tests: 113/113 PASSING ✅
Status: All unit and integration tests passing
```

## Metrics Baseline

```
Documents processed: 5
Table Accuracy:      2.4%   ⚠️  (Target: 15-20%)
Style Accuracy:      31.5%  ⚠️  (Target: 40-50%)
Robustness:          100.0% ✅
Performance:         90.0%  ✅

Composite Score:     32.5/100 ⚠️ (Target: 40-45/100)
```

## Drift Analysis

```
Total drifts: 3052

By Severity:
  🔴 CRITICAL: 857
  🟠 MAJOR: 909
  🟡 MINOR: 1286

By Category:
  content:mismatch: 2067 occurrences (68%)
  style:mismatch: 470 occurrences (15%)
  list:mismatch: 282 occurrences (9%)
  table:mismatch: 140 occurrences (5%)
  heading:mismatch: 82 occurrences (3%)
```

## Key Observations

### 1. Table Structure vs Content Mismatch

**Structural Success (Loop 012):**

- Column detection improved: 2 → 13 columns
- DBSCAN clustering correctly identifies column boundaries
- Grid structure matches expected layout

**Content Failure:**

- Table Accuracy unchanged at 2.4%
- Cell content completely incorrect
- Text from outside tables appearing in cells
- Multiple values merged into single cells

**Example:** `one_tool_2512.20957v2.mdf.gen`

Generated (WRONG):

```
| One Tool Is Enough: Reinforcement Learning for Repository-Level LLM Agents |  |  |  |  |  |  |  |  |  |  |  |  |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Locating the files and functions requiring modifi- cation in large open-source software (OSS) repos- itories is challenging... [entire abstract in one cell!] |
```

Gold (CORRECT):

```
| Agent Pipeline | Model | Function-level Recall | Funct Precision | Funct Sample-F1 | Funct IoU | File-level Recall | File Precision | File Sample-F1 | File IoU |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| **Closed-source** |  |  |  |  |  |  |  |  |  |
| RepoSearcher | Claude3.7-Sonnet | 66.80 | 28.30 | 19.90 | 17.89 | 89.71 | 33.15 | 21.04 | 20.67 |
```

### 2. Root Cause Diagnosis

**Problem Location:** `lattice.rs::extract_text_in_rect()`

**Current Implementation Issues:**

1. **±2pt tolerance too loose:**

   ```rust
   if elem.bbox.x0 >= (bbox.x0 - 2.0) && elem.bbox.x1 <= (bbox.x1 + 2.0)
       && elem.bbox.y0 >= (bbox.y0 - 2.0) && elem.bbox.y1 <= (bbox.y1 + 2.0)
   ```

   This allows text 4pt outside the cell boundary to be included!

2. **5pt Y-binning creates vertical drift:**

   ```rust
   let y_bin = (elem.bbox.y0 / 5.0).round() as i32;
   ```

   Text elements 5pt apart get merged, causing row spillover

3. **No exact grid alignment:**
   - Cells should align precisely to column/row boundaries
   - Current code uses rectangle containment, not grid coordinate matching

### 3. Impact Analysis

**Why This Matters (ROI Calculation):**

- Table Accuracy: 2.4% with 40% weight = 0.96 composite points
- Style Accuracy: 31.5% with 40% weight = 12.6 composite points

**If we improve Table Accuracy to 15%:**

- Table contribution: 15% × 0.40 = 6.0 composite points
- Gain: +5.04 composite points (+15.5% overall)

**If we improve Table Accuracy to 20%:**

- Table contribution: 20% × 0.40 = 8.0 composite points
- Gain: +7.04 composite points (+21.7% overall)

This is HIGH ROI work - fixing cell content extraction could improve composite score to 37-39/100 in a single iteration!

## Test Case Evidence

### Document Performance Comparison

| Document              | Table Accuracy | Style Accuracy | Composite | Notes                                  |
| --------------------- | -------------- | -------------- | --------- | -------------------------------------- |
| one_tool_2512.20957v2 | 11.4%          | 20.0%          | 31.6      | Best table performer, still only 11.4% |
| AlphaEvolve           | 0.3%           | 50.2%          | 39.2      | High style, near-zero tables           |
| agent_2510.09244v1    | 0.0%           | 44.0%          | 36.6      | No tables detected correctly           |
| 2900_Goyal_et_al      | 0.0%           | 39.3%          | 34.7      | No tables detected correctly           |
| ccn_2512.21804v1      | 0.0%           | 3.9%           | 20.6      | Both metrics very low                  |

**Average:** 2.4% table, 31.5% style

### Pattern: Column Detection Works, Content Assignment Fails

- Best document: 11.4% table accuracy (still poor!)
- 60% of documents: 0% table accuracy (complete failure)
- Column boundaries correctly detected (DBSCAN success)
- Cell content assignment broken (extract_text_in_rect failure)

## Conclusion

Loop 012 fixed structural issues (column detection). Loop 013 must fix content extraction to realize the gains. The path forward is clear: tighten bounding box matching, eliminate Y-binning, and use exact grid coordinate alignment.

**Next Phase:** ORIENT - Deep dive into extract_text_in_rect() implementation and design improved cell-text assignment algorithm.
