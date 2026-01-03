# Task Log: PDF Table Extraction OODA Loops 4-5

**Date:** 2026-01-03  
**Session:** Continue specs/27-improve-pdf.md with first principles approach  
**Focus:** Table extraction accuracy improvement using OODA methodology

---

## Actions Performed

### OODA Loop 4: Text Clustering Approach

**Hypothesis:** Grid lines don't align with logical columns, need text-based column detection.

**Changes Made:**

1. **Fixed DBSCAN epsilon** (lattice.rs ~530):

   - Changed from adaptive (10th percentile) to fixed 15pt
   - Rationale: Adaptive was creating 50-132 columns (one per character)
   - Fixed value creates reasonable column boundaries

2. **Conditional clustering** (lattice.rs ~255-280):
   - Only apply clustering when NO vertical grid lines (`unique_x.len() < 2`)
   - Trust explicit grid lines when present
   - Prevents false crossing_ratio rejections

**Result:** No improvement. Table Accuracy still 2.4%.

**Learning:** Clustering addressed wrong problem - applied at table level when issue is at cell level.

### OODA Loop 5: First Principles Metric Validation

**Action:** Direct comparison of generated vs gold markdown to validate metrics.

**Key Findings:**

1. **one_tool table analysis:**

   - Gold: 72 table lines, Generated: 56 lines
   - Missing 16 lines (22%)
   - Main table starts at wrong row (line 377 vs 135)
   - Started at "CoSIL" (row 18) instead of header

2. **Merged cell problem:**

   ```
   GOLD: | Agentless | Training Free | 25.20 | 14.30 | ... (9 cells)
   GEN:  | Agentless Training Free 25.20 14.30 ... | (1 cell with 9 values)
   ```

3. **False positives:**
   - Mathematical notation detected as tables: `|Y ∩ *Y*ˆ ∗|`
   - Sentence fragments in table format

**Root Cause Identified:**

PDF tables have THREE structural types:

- **Type A:** Well-structured (grid line per boundary) → Works (11.4% success)
- **Type B:** Merged cells (one grid cell, multiple logical cells) → Fails (majority)
- **Type C:** Whitespace only (no grid lines) → Completely missed (agent_2510)

---

## Decisions Made

### Loop 4 Decision: Try Text Clustering

- **Decision:** Use X-position clustering to detect columns when grid insufficient
- **Rationale:** Thought grid lines don't align with data columns
- **Impact:** No improvement - wrong hypothesis

### Loop 5 Decision: Accept Validator Accuracy

- **Decision:** The 2.4% metric is CORRECT, not a measurement error
- **Rationale:** Direct markdown comparison confirms broken output
- **Impact:** Changed focus from "fix metrics" to "fix underlying extraction"

---

## Next Steps

### High Priority: Fix Merged Cell Splitting (Loop 6)

**Problem:** `extract_text_in_rect()` returns one string per grid cell, but grid cells often contain multiple logical cells.

**Solution:**

```rust
// Current:
fn extract_text_in_rect(...) -> String

// Needed:
fn extract_text_in_rect(...) -> Vec<String>  // One per X-position cluster
```

**Implementation plan:**

1. Cluster text within cell by X-position (epsilon ~20pt)
2. Return vector of cluster texts
3. Update table building to handle variable columns per row
4. Match subcells to correct columns

**Expected gain:** one_tool: 11.4% → 40-50%

### Medium Priority: Whitespace Table Detection (Loop 7-8)

**Problem:** agent_2510 has 0/22 tables (100% miss rate) - whitespace tables invisible.

**Solution:** Already partially implemented (lines 255-280), but needs improvement.

### Low Priority: False Positive Reduction (Loop 9)

**Problem:** Math notation and prose detected as tables.

**Solution:** Add content-based validation (equation detection, prose detection).

---

## Lessons Learned

### First Principles Success

**Validated assumption by direct comparison:**

- Didn't trust "2.4%" number blindly
- Compared actual generated vs gold markdown
- Found specific broken patterns (merged cells, missing headers, false positives)
- This validates user's instruction: "always seek truth from direct comparison"

### Why "Heuristics" Failed

**Loops 1-3:** Tweaked thresholds and tolerances

- crossing_ratio: 0.1 → 0.35
- containment logic: center → start point
- decorative filter: added

**Result:** No improvement because these are SYMPTOMS not ROOT CAUSES.

**Loop 4:** Tried smarter approach (clustering)

- Applied at wrong architectural level
- Table-level when problem is cell-level

### True First Principles Approach

**Start with:**

1. What does the validator actually measure? (Token-level F1 in cells)
2. Why are cells wrong? (Direct comparison)
3. What causes that? (PDF structure analysis)
4. How to fix? (Architecture change, not parameter tuning)

**This is what user meant by "Never Cheat or Fake the results"** - don't tweak parameters hoping metrics improve, FIX THE ACTUAL PROBLEM.

---

## Files Modified

### edgequake/crates/edgequake-pdf/src/backend/lattice.rs

**Line ~530:** Fixed DBSCAN epsilon

```rust
- let epsilon = distances[p10_idx]; // Adaptive
+ let epsilon = 15.0; // Fixed 15pt for column boundaries
```

**Lines ~255-280:** Conditional clustering

```rust
if unique_x.len() < 2 {
    // Only cluster when no vertical lines
    let detected_x = self.detect_columns_by_clustering(...);
    ...
}
```

---

## Metrics Summary

### Baseline (Start of Session)

```
Table Accuracy:      2.4%
Style Accuracy:      31.1%
Composite Score:     32.4/100
```

### After Loops 4-5

```
Table Accuracy:      2.4%  (unchanged)
Style Accuracy:      31.1% (unchanged)
Composite Score:     32.4/100 (unchanged)
```

### Expected After Loop 6 (Merged Cell Fix)

```
Table Accuracy:      ~15-20% (est.)
Composite Score:     ~40-45/100 (est.)
```

---

## Session Documentation

Created files:

- `sessions/improve_pdf/004-OBSERVE-TRUTH.md` - First principles validator challenge
- `sessions/improve_pdf/004-ORIENT.md` - Root cause analysis
- `sessions/improve_pdf/004-DECIDE.md` - Decision process (rejected complex solutions)
- `sessions/improve_pdf/004-ACT.md` - Implementation summary
- `sessions/improve_pdf/005-OBSERVE.md` - Direct markdown comparison findings

---

**Session Duration:** ~3 hours  
**OODA Loops Completed:** 4-5 / 10  
**Next Session:** Implement merged cell splitting (Loop 6)  
**Status:** Making progress - identified TRUE root cause through first principles analysis
