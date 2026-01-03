# Loop 013 - ACT Phase

## Timestamp

Friday, January 3, 2026 01:00:00 HKT

## Implementation Summary

### Changes Made

**File:** `edgequake/crates/edgequake-pdf/src/backend/lattice.rs`
**Function:** `extract_text_in_rect()` (lines 505-547)

**Modifications:**

1. Reduced tolerance from 2.0pt → 1.5pt (via 1.0pt, 0.5pt experiments)
2. Removed 5pt Y-binning, replaced with 1.0pt same-row threshold
3. Improved Y-coordinate sorting (exact coordinates vs binned values)

**Iterations Tested:**

- v1: 0.5pt tolerance + estimated bbox → TOO TIGHT (27478 chars, empty cells)
- v2: 1.0pt tolerance + center point → STILL TOO TIGHT (40631 chars, all text in first column)
- v3: 1.5pt tolerance + center point → NO CHANGE (40950 chars, all text in first column)

### Test Results

**Compilation:** ✅ Success (all 113 tests passing)

**Generated Output Analysis:**

```
Document: one_tool_2512.20957v2.mdf.gen

Gold structure (line 138):
| RepoSearcher | Claude3.7-Sonnet | 66.80 | 28.30 | ... | (10 columns, proper data)

Generated structure (line 136):
| RepoSearcher Claude3.7-Sonnet  66.80  19.90 28.30 17.89 ... | | | | | | | | (8 cols, all data in col 0)
```

**Validator Metrics:**

```
Table Accuracy:      2.4%   (NO CHANGE from baseline)
Style Accuracy:      31.5%  (NO CHANGE)
Composite Score:     32.5/100 (NO CHANGE)
Tests Passing:       113/113 ✅
```

### Root Cause Analysis

#### What We Learned

1. **Tolerance is NOT the problem:**

   - Tried 0.5pt, 1.0pt, 1.5pt, 2.0pt
   - Output character count varied (27k-41k) but table structure unchanged
   - All data still goes to first column regardless of tolerance

2. **Column detection MAY be working:**

   - Correct number of `|` separators in markdown
   - Separator row shows 8-13 columns depending on table
   - This suggests column boundaries ARE being detected

3. **Text assignment is fundamentally broken:**
   - `extract_text_in_rect(elements, left, bottom, right, top)` returns empty for columns 1+
   - Only column 0 gets text (all of it!)
   - This indicates cell boundaries don't match text coordinates

#### Three Possible Root Causes

**Hypothesis 1: TextElement doesn't provide bbox**

- `TextElement` has center point (x, y) and font_size
- No bbox information (x0, y0, x1, y1)
- Center-point containment check is fundamentally flawed for wide text
- **Evidence:** Short text like "66.80" works, but wide text like "Claude3.7-Sonnet" fails

**Hypothesis 2: PDF coordinate system issue**

- Text elements and grid lines might use different coordinate spaces
- Possible Y-axis inversion or scaling mismatch
- **Evidence:** Zero columns get text except first → systematic offset

**Hypothesis 3: Column boundaries from lines don't align with text placement**

- Tables use vertical lines for structure (decorative)
- Text is placed OUTSIDE the line boundaries
- Clustering would work (based on actual text X-coords) but lines don't
- **Evidence:** Loop 012 showed clustering detects 13 cols, but score unchanged

### Critical Discovery

**The tolerance experiments revealed a KEY insight:**

When tolerance changed from 2.0pt → 0.5pt, output went from 41k → 27k chars. This means:

- **At 2.0pt:** Extra text was being included (from outside table, adjacent cells)
- **At 0.5pt:** Most text was excluded (even from correct cells)
- **At 1.0-1.5pt:** Intermediate (40k chars, closer to original)

**But table structure never improved!** All data stayed in first column.

This proves: **Tolerance affects how much junk is included, but doesn't fix the fundamental cell assignment problem.**

### What Needs Investigation

1. **Check if tables use vertical lines or clustering:**

   ```rust
   // In create_table_block(), line 258:
   if unique_y.len() >= 2 && unique_x.len() < 2 {
       // Use clustering
   } else {
       // Use vertical lines
   }
   ```

   Need to know which path is taken for one_tool tables!

2. **Verify TextElement coordinates vs cell boundaries:**

   - Add debug output showing:
     - Cell boundaries: [left, bottom, right, top]
     - Text elements: (x, y, text)
     - Which elements pass the containment check
   - Example for cell [0][1]:
     ```
     Cell [0][1]: [100, 50, 150, 70]
     Checking: (x=105, y=60, "Claude3.7-Sonnet") → PASS or FAIL?
     ```

3. **Test DBSCAN clustering directly:**
   - Force tables to use clustering (even if they have vertical lines)
   - Check if clustering-based boundaries work better than line-based

### Blocked Issues

**Cannot proceed without understanding:**

1. Are tables using line-based or clustering-based column detection?
2. What are the actual cell boundary coordinates?
3. What are the actual text element coordinates?
4. Why does text only appear in column 0?

**Debugging strategy needed:**

- Add tracing/println to extract_text_in_rect
- Log every cell boundary and which text elements match
- Identify the coordinate mismatch pattern

## Attempted Solutions Summary

| Approach                  | Result                       | Lesson                     |
| ------------------------- | ---------------------------- | -------------------------- |
| Reduce tolerance to 0.5pt | Empty cells, 27k chars       | Too restrictive            |
| Use estimated bbox        | Empty cells                  | Bbox estimation inaccurate |
| Tolerance 1.0pt           | All data in col 0, 40k chars | Not tight enough           |
| Tolerance 1.5pt           | All data in col 0, 41k chars | No improvement             |
| Remove Y-binning          | No score change              | Y-axis wasn't the issue    |

**Conclusion:** Tolerance tuning does NOT solve the problem. Need architectural fix.

## Next Actions Required

1. **Add comprehensive debug logging:**

   ```rust
   tracing::debug!("Cell [{}][{}]: [{}, {}, {}, {}]", i, j, left, bottom, right, top);
   tracing::debug!("Found {} text elements", contained.len());
   for elem in &contained {
       tracing::debug!("  - ({}, {}): {}", elem.x, elem.y, elem.text);
   }
   ```

2. **Create minimal reproduction test:**

   - Extract one table from one_tool PDF
   - Create unit test with known cell boundaries
   - Verify extract_text_in_rect() behavior

3. **Compare line-based vs clustering-based column detection:**

   - Temporarily force clustering path
   - Check if scores improve

4. **Consider alternative: Use text X-coordinates directly as column boundaries**

   - Instead of using vertical lines
   - Use DBSCAN clusters of text X-coords
   - Assign text to nearest column boundary

5. **Review gold file generation:**
   - How were gold files created?
   - Did they use different logic for cell-text assignment?
   - Can we replicate their approach?

## Metrics

**Before Loop 013:**

- Composite: 32.5/100
- Table Accuracy: 2.4%
- Tests: 113/113 ✅

**After Loop 013:**

- Composite: 32.5/100 (NO CHANGE)
- Table Accuracy: 2.4% (NO CHANGE)
- Tests: 113/113 ✅

**Verdict:** Loop 013 FAILED to improve metrics. Tolerance tuning was insufficient.

## Lessons Learned

1. **Assumptions must be validated:**

   - Assumed tolerance was the problem → WRONG
   - Should have added debug logging first

2. **Small changes don't always help:**

   - Three tolerance values (0.5, 1.0, 1.5) → same result
   - Need to understand root cause before implementing fixes

3. **TextElement limitations:**

   - No bbox → cannot do proper geometric containment
   - Center point + tolerance is fundamentally limited

4. **Test-driven debugging:**
   - Should create minimal reproduction case
   - Unit test specific to table cell extraction

## Task Log Summary

**Status:** ⚠️ INCOMPLETE - Zero metric improvement despite 3 implementation attempts

**Problem:** Table cell text assignment broken - all text goes to first column

**Blocker:** Cannot diagnose without debug logging of cell boundaries and text coordinates

**Recommendation:** Pause Loop 013, add comprehensive logging, understand coordinate system before next attempt

## Files Modified

- `edgequake/crates/edgequake-pdf/src/backend/lattice.rs` (extract_text_in_rect function)

## Commit Message

```
Loop 013: Failed attempt to fix table cell extraction via tolerance tuning

Attempted 3 tolerance values (0.5pt, 1.0pt, 1.5pt) to fix extract_text_in_rect():
- Removed 5pt Y-binning → use 1pt same-row threshold
- Tightened tolerance → prevent adjacent cell pollution
- Improved Y-coordinate sorting → eliminate row merging

Results: NO IMPROVEMENT
- Table Accuracy: 2.4% (unchanged)
- Composite: 32.5/100 (unchanged)
- All table data still goes to first column

Root cause: Tolerance is NOT the problem. Text element coordinates
don't match cell boundaries. Need debug logging to understand
coordinate system mismatch.

Tests: 113/113 passing ✅
Next: Add tracing, create minimal repro, investigate line vs clustering detection
```
