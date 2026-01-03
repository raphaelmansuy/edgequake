# Task Log - Loop 013: Failed Table Cell Extraction Fix

## Date

Friday, January 3, 2026 01:00:00 HKT

## Actions

1. Ran OBSERVE phase: Tests 113/113 pass, metrics baseline (Composite 32.5, Table 2.4%)
2. Analyzed drift patterns: 3052 drifts, 140 table mismatches (5%)
3. Created comprehensive OBSERVE.md documenting baseline state
4. Ran ORIENT phase: Deep-dived into extract_text_in_rect() implementation
5. Diagnosed three critical flaws: ±2pt tolerance, center-point vs bbox, 5pt Y-binning
6. Created ORIENT.md with root cause analysis and first principles breakdown
7. Ran DECIDE phase: Designed improved algorithm with bbox containment
8. Created DECIDE.md with implementation strategy and test cases
9. Attempted fix #1: 0.5pt tolerance + estimated bbox → TOO TIGHT (27k chars, empty cells)
10. Attempted fix #2: 1.0pt tolerance + center point → STILL TOO TIGHT (40k chars, all data in col 0)
11. Attempted fix #3: 1.5pt tolerance + center point → NO CHANGE (41k chars, all data in col 0)
12. Ran validator after each attempt: Table Accuracy 2.4% (UNCHANGED)
13. Created comprehensive ACT.md documenting failure and lessons learned
14. Committed all work with honest assessment

## Decisions

- **Three tolerance attempts**: Tried 0.5pt, 1.0pt, 1.5pt to find optimal balance
- **Removed Y-binning**: Changed from 5pt bins to 1pt same-row threshold (good improvement)
- **Kept center-point approach**: Couldn't use bbox (TextElement doesn't provide it)
- **Stopped after 3 attempts**: Recognized tolerance tuning won't solve the problem
- **Documented failure honestly**: Critical for future debugging

## Next Steps

1. **Add comprehensive debug logging**:

   - Log cell boundaries for each cell [i][j]
   - Log which text elements pass containment check
   - Identify coordinate mismatch pattern

2. **Investigate column detection path**:

   - Check if tables use vertical lines or clustering
   - Determine why text only appears in column 0

3. **Create minimal reproduction test**:

   - Extract single table from one_tool PDF
   - Unit test extract_text_in_rect() with known coordinates
   - Isolate the exact failure mode

4. **Consider alternative approaches**:
   - Force clustering-based column detection (even with vertical lines)
   - Use text X-coordinates directly as column boundaries
   - Research how gold files were generated

## Lessons/Insights

- **Debug before implementing**: Should have added logging first, not after 3 failed attempts
- **Tolerance is a symptom, not the disease**: Parameter tuning can't fix architectural issues
- **TextElement limitations are critical**: No bbox info → fundamentally limited containment checks
- **Test-driven debugging**: Need minimal repro case to understand root cause
- **Honest failure documentation**: More valuable than hiding unsuccessful attempts
- **Listen to the data**: Character count changed (27k-41k) but structure unchanged → tolerance affects quantity, not quality

## Metrics Summary

**Before Loop 013:**

- Composite: 32.5/100
- Table Accuracy: 2.4%
- Style Accuracy: 31.5%
- Tests: 113/113 ✅

**After Loop 013:**

- Composite: 32.5/100 (NO CHANGE)
- Table Accuracy: 2.4% (NO CHANGE)
- Style Accuracy: 31.5% (NO CHANGE)
- Tests: 113/113 ✅

**Verdict:** ⚠️ FAILED - Zero metric improvement despite comprehensive analysis and 3 implementation attempts

## Files Modified

- `edgequake/crates/edgequake-pdf/src/backend/lattice.rs` (extract_text_in_rect function, lines 505-547)
- Created: `sessions/improve_pdf/013-iteration/OBSERVE.md` (1500+ lines)
- Created: `sessions/improve_pdf/013-iteration/ORIENT.md` (2000+ lines)
- Created: `sessions/improve_pdf/013-iteration/DECIDE.md` (2000+ lines)
- Created: `sessions/improve_pdf/013-iteration/ACT.md` (700+ lines)

## Problem Statement for Loop 014

**What we know:**

- Column boundaries ARE being detected (correct number of `|` separators)
- Text assignment IS broken (all data in first column)
- Tolerance tuning DOES NOT help (tried 0.5-1.5pt, no difference)
- Y-binning removal DID help conceptually (better row detection)

**What we don't know:**

- Are tables using line-based or clustering-based column detection?
- What are the actual cell boundary coordinates?
- What are the actual text element coordinates?
- Why does containment check only succeed for column 0?

**Hypothesis for Loop 014:**
The cell boundaries from vertical lines are offset from text placement. Clustering-based boundaries (using actual text X-coords) would work better. Need to force clustering path or add text-coordinate-based column detection.

**Action for Loop 014:**
Add comprehensive debug logging → understand coordinate mismatch → implement proper fix (not just parameter tuning).
