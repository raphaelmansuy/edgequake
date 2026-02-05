# OODA Iteration 02 - Orient

**Mission**: Improve PDF-to-Markdown Conversion Quality
**Date**: 2026-02-05

---

## Root Cause Analysis

### Primary Issue

The `compute_smart_sort_key()` function has a bug in the left-block finder logic:

**Current (WRONG)**:

```rust
// Uses max_by(x2) - finds RIGHT-MOST left block
left_blocks.iter()
    .max_by(|(_, a), (_, b)| a.bbox.x2.partial_cmp(&b.bbox.x2).unwrap())
```

**Should be (per PyMuPDF4LLM)**:

```rust
// Uses min_by(x1) - finds LEFT-MOST left block
left_blocks.iter()
    .min_by(|(_, a), (_, b)| a.bbox.x1.partial_cmp(&b.bbox.x1).unwrap())
```

### Secondary Issue

The smart sort key is only applied WITHIN columns via `sort_by_smart_key()`, not to the final merged result.

However, for academic two-column papers, the sequential column merge (all of column 1, then all of column 2) is actually correct behavior. The issue is the wrong left-block finder.

### Impact Assessment

- **Severity**: Medium - affects documents with blocks at similar vertical positions
- **Scope**: Reading order within columns
- **Risk**: Low - change is isolated to one function

---

## Options Analysis

### Option A: Fix left-block finder only (RECOMMENDED)

- **Effort**: 5 min
- **Risk**: Very low
- **Impact**: Corrects smart sort key computation to match PyMuPDF4LLM

### Option B: Apply smart sort to final merged result

- **Effort**: 30 min
- **Risk**: Medium - could disrupt working academic paper ordering
- **Impact**: Might improve some edge cases, might break others

### Option C: Remove smart sort entirely, use pure sequential

- **Effort**: 10 min
- **Risk**: Medium - loses the benefit of smart within-column ordering
- **Impact**: Simplifies code but may regress quality

---

## Decision Rationale

**Choose Option A**: Fix the left-block finder bug.

1. **First Principles**: PyMuPDF4LLM uses left-most block for computing sort key
2. **Minimal change**: Single line fix with clear correctness
3. **Preserves working behavior**: Sequential column merge is correct for academic papers
4. **Low risk**: Easy to verify with existing tests

---

## Expected Outcome

After fix:

- Smart sort key will use left-most overlapping block's Y coordinate
- Right-column blocks at same vertical level will sort after left-column blocks
- All 497 tests should continue passing
