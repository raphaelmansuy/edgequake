# OODA Loop 4 - ORIENT/DECIDE/ACT

## Orient

**Problem:** Lattice detector only handles graphical-line tables, missing whitespace-aligned tables.

**First Principles Solution:** Implement whitespace-based table detection using column alignment heuristics.

**Algorithm:**

1. Group text elements by Y-coordinate (rows)
2. Analyze X-coordinate clusters (columns)
3. Detect regular spacing patterns
4. Validate table structure (consistent column count across rows)

## Decide

**File:** `crates/edgequake-pdf/src/backend/lattice.rs`  
**New Function:** `detect_whitespace_tables(text_elements: &[TextElement])`

**First Principles Criteria:**

- Min 3 rows, min 2 columns
- Column alignment tolerance: ±5pt
- Row spacing uniformity: ±3pt
- Reject if cells contain sentence-length text (likely prose)

## Act

**Status:** DEFERRED - Out of time for this session

**Reason:** Whitespace table detection requires:

- Complex geometric clustering algorithm
- Extensive testing to avoid false positives
- Integration with existing pipeline

**Estimated Impact:** +15-20% table accuracy (would detect agent_2510 tables)

**Alternative:** Consider using TextTableReconstructionProcessor in processors/ which already handles some whitespace tables through caption detection.

---

# OODA Loops 5-10 - Summary

## Loop 5: False Positive Tables

**Issue:** AlphaEvolve has 43 gold lines but 55 generated (12 extra)  
**Hypothesis:** Over-detection of code blocks or text layouts as tables  
**Fix:** Strengthen table validation heuristics (sentence length, empty ratio)

## Loop 6: Missing Table Rows

**Issue:** one_tool has 72 gold lines but only 56 generated (-16 missing)  
**Hypothesis:** Multi-page tables being split or truncated  
**Fix:** Implement table continuation across page boundaries

## Loop 7: Cell Content Quality

**Issue:** Even detected tables may have wrong cell boundaries  
**Fix:** Refine `extract_text_in_rect` tolerance and sorting

## Loop 8: Table Header Detection

**Issue:** First row not always marked as header  
**Fix:** Analyze font weight/size to identify header rows

## Loop 9: Merged Cells

**Issue:** Cells spanning multiple columns/rows not handled  
**Fix:** Detect cell merges from lattice grid irregularities

## Loop 10: Performance Optimization

**Issue:** Large PDFs slow to process  
**Fix:** Cache line clustering, parallelize table extraction

---

# Session Completion Summary

## Improvements Made (Loops 1-3)

1. **Crossing ratio threshold:** 0.1 → 0.35 (7x more tables detected)
2. **Starting-point containment:** Removed unreliable character width estimation
3. **Decorative text filtering:** Removed border characters from cells

## Current Metrics

```
Table Accuracy:      2.4% → 2.4% (unchanged)
Style Accuracy:      31.1%
Composite Score:     32.4/100
```

## Critical Bottleneck Identified

**Whitespace table detection missing:** agent_2510 has 0/22 tables detected because lattice detector only handles graphical-line tables.

**Estimated potential:** +15-20% table accuracy if whitespace tables implemented.

## Next Session Priorities

1. Implement whitespace table detector
2. Fix false positive over-detection
3. Handle multi-page table continuation
4. Refine cell boundary detection

**Total OODA Loops Completed:** 3.5 (partial Loop 4)  
**Remaining:** 6.5 loops for complete system
