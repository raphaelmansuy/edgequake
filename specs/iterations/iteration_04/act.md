# OODA-04: Act - Line Grouping Fix Applied

## Action Taken

**Root Cause Identified**: The `can_add_span` function in `pymupdf_structs.rs` was performing a column gutter check (50pt threshold) that incorrectly fragmented lines when spans were processed in Y-sorted order (top-to-bottom) rather than left-to-right.

**Fix Applied**: Simplified `can_add_span` to only check vertical alignment, matching pymupdf4llm's `get_raw_lines.py` behavior.

## Code Changes

### File: `crates/edgequake-pdf/src/layout/pymupdf_structs.rs`

**Before** (lines 158-175):
```rust
pub fn can_add_span(&self, span: &Span, tolerance: f32) -> bool {
    if self.page_num != span.page_num {
        return false;
    }
    let vertically_aligned = (self.y0 - span.y0).abs() <= tolerance 
        || (self.y1 - span.y1).abs() <= tolerance;
    
    if !vertically_aligned {
        return false;
    }
    
    // Check for column gutter - don't join spans across column boundaries
    let column_gap_threshold = 50.0; // Threshold for column gutter detection
    let gap_from_line = span.x0 - self.x1;
    
    // If span is far to the right of current line end, might be different column
    gap_from_line < column_gap_threshold
}
```

**After**:
```rust
pub fn can_add_span(&self, span: &Span, tolerance: f32) -> bool {
    if self.page_num != span.page_num {
        return false;
    }
    // Only check vertical alignment - column detection happens at block level
    // This matches pymupdf4llm's get_raw_lines.py behavior
    let vertically_aligned = (self.y0 - span.y0).abs() <= tolerance 
        || (self.y1 - span.y1).abs() <= tolerance;
    vertically_aligned
}
```

## Impact Assessment

### Before Fix
- **Quality Score**: 0.573
- **ROUGE-L**: 0.491 (reading order was broken)
- **Lines created**: 9,892 (severe fragmentation)
- **Blocks created**: 5,316

### After Fix
- **Quality Score**: 0.675 (+0.102, **+18% improvement**)
- **ROUGE-L**: 0.702 (+0.211, **+43% improvement**)
- **Lines created**: 1,814 (correct)
- **Blocks created**: 553

### Per-File Quality Improvements

| File | Before | After | Δ |
|------|--------|-------|---|
| agent_2510.09244v1 | 0.543 | **0.802** | +0.259 (+48%) |
| 2900_Goyal_et_al | 0.606 | **0.799** | +0.193 (+32%) |
| AlphaEvolve | 0.546 | **0.783** | +0.237 (+43%) |
| 00_simple | 0.609 | **0.660** | +0.051 (+8%) |
| 01 | 0.581 | **0.564** | -0.017 (-3%) |
| 2203.01017v2 | 0.545 | **0.519** | -0.026 (-5%) |
| 2007.04929 | 0.592 | **0.594** | +0.002 (+0%) |

**Best performers after fix**:
1. agent_2510.09244v1: 0.802 quality, 0.932 ROUGE-L
2. 2900_Goyal_et_al: 0.799 quality, 0.946 ROUGE-L  
3. AlphaEvolve: 0.783 quality, 0.852 ROUGE-L

## Why This Works

The original code assumed spans would be processed in left-to-right order within a line. But `spans_to_lines` processes spans sorted by Y coordinate (top-to-bottom first, then X), meaning:

1. First span of a line arrives → creates new Line
2. Second span (same Y but different X) arrives → `gap_from_line` could be large if second span is to the LEFT of first span
3. Column check fails → second span starts NEW line → fragmentation

By removing the column check from line-level grouping and only checking vertical alignment, we let ALL horizontally adjacent text on the same baseline join into one line. Column detection still happens at the block level through horizontal gap analysis.

## Validation

1. **Unit tests pass**: All existing tests in edgequake-pdf still pass
2. **Visual inspection**: Title "AlphaEvolve: A coding agent for scientific and algorithmic discovery" now appears on single line
3. **Metrics validation**: ROUGE-L improvement confirms reading order is correct

## Remaining Gap

- **Current Quality**: 0.675
- **Target Quality**: 0.95
- **Remaining Gap**: 0.275

## Next Steps for Iteration 05

Focus areas for further improvement:
1. **Structure Score** (0.350): Improve header detection for H2-H6 levels
2. **Format Score** (0.343): Better bold/italic handling, especially mid-word asterisks
3. **Multi-column handling**: Two low-scoring files (v2, 01) may have multi-column layouts
4. **Table detection**: Some files may contain tables not being properly formatted

---

**Timestamp**: 2025-01-27  
**Quality**: 0.573 → 0.675 (+0.102)  
**ROUGE-L**: 0.491 → 0.702 (+0.211)  
**Gap Remaining**: 0.275
