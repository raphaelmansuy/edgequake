# EdgeQuake PDF Improvement - Progress Summary

**Date:** 2026-01-03
**Session:** OODA Loops for PDF→Markdown Quality

## Current Status

**Loops Completed:** 18/30 minimum
**Current Composite Score:** 44.1/100

### Score Breakdown

| Metric         | Score | Weight | Contribution |
| -------------- | ----- | ------ | ------------ |
| Table Accuracy | 27.2% | 40%    | 10.9         |
| Style Accuracy | 35.6% | 40%    | 14.2         |
| Robustness     | 100%  | 10%    | 10.0         |
| Performance    | 90%   | 10%    | 9.0          |
| **TOTAL**      | -     | -      | **44.1**     |

### Per-Document Performance

| Document              | Table | Style | Composite |
| --------------------- | ----- | ----- | --------- |
| 2900_Goyal_et_al      | 98.3% | 37.6% | 73.3      |
| AlphaEvolve           | 30.4% | 50.3% | 51.3      |
| agent_2510.09244v1    | 0.0%  | 58.8% | 42.5      |
| ccn_2512.21804v1      | 0.0%  | 7.0%  | 21.8      |
| one_tool_2512.20957v2 | 7.7%  | 23.5% | 24.7      |

## Loops Summary

### Loops 1-17 (Previous Work)

- Baseline establishment
- Table detection improvements (single-row rejection, header detection)
- Achieved 44.1/100 composite score

### Loop 18 (This Session)

**Focus:** Style accuracy via heading detection
**Changes:**

- Added font-size based heading detection
- Re-enabled SectionPatternProcessor
  **Result:** No score change (44.1 → 44.1)
  **Lesson:** Heading detection not the bottleneck

## Improvement Opportunities

### 1. Table Accuracy (27.2% - Highest Impact)

**Weight:** 40% of composite score
**Issues:**

- 3 documents with 0% table accuracy
- AlphaEvolve only 30.4%
- Cell content extraction problems

**Potential Gains:** +20-30% table accuracy → +8-12 composite points

### 2. Style Accuracy (35.6% - High Impact)

**Weight:** 40% of composite score
**Issues:**

- ccn_2512.21804v1 only 7% style accuracy
- Bold/italic detection may be weak
- Not a heading detection problem

**Potential Gains:** +15-20% style accuracy → +6-8 composite points

### 3. Performance (90% - Low Impact)

**Weight:** 10% of composite score
**Current:** Already good at 90%
**Potential Gains:** Limited (+1-2 composite points max)

### 4. Robustness (100% - No Impact)

**Weight:** 10% of composite score
**Current:** Perfect score
**Action:** Maintain, don't break

## Next Steps (Loops 19-30)

### Priority 1: Table Accuracy (Loops 19-24)

**Target:** 27.2% → 50%+ (gain ~10 composite points)

Approaches:

1. Improve column boundary detection
2. Fix cell content extraction (currently extracting only column 0)
3. Better table vs non-table discrimination
4. Handle tables without grid lines

### Priority 2: Style Accuracy (Loops 25-28)

**Target:** 35.6% → 55%+ (gain ~8 composite points)

Approaches:

1. Improve bold/italic detection at extraction level
2. Check font weight thresholds
3. Validate span-level style preservation
4. Handle font name based heuristics

### Priority 3: Final Optimization (Loops 29-30)

**Target:** Overall polish and regression prevention

## Predicted Final Score

With focused improvements:

- Table: 27% → 50% (+9 points)
- Style: 36% → 55% (+8 points)
- **Total: 44 → 61 points**

Stretch goal with excellent execution:

- Table: 27% → 60% (+13 points)
- Style: 36% → 60% (+10 points)
- **Total: 44 → 67 points**

## Session Files

- Session directory: `sessions/improve_pdf/`
- Iteration logs: `001-iteration/` through `018-iteration/`
- Scratchpad: `scratchpad_append_log.md`
- Metrics: `metrics_loop_*.json`

## Tests Status

✅ All 103 tests passing
✅ No compilation errors
✅ No crashes on real dataset
✅ All generated markdown pandoc-valid
