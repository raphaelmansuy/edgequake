# DECIDE Phase - Loop 010

## Implementation Decision

**Chosen Strategy:** Option 5 - Remove deprecated methods + update test  
**Rationale:** Complete elimination > partial improvement, methods already deprecated and unused  
**Date:** 2024-01-XX

## Final Implementation Plan

### Change 1: Remove single_column() Method

**File:** `crates/edgequake-pdf/src/layout/xy_cut.rs`  
**Lines:** 43-55 (13 lines including doc comments)  
**Action:** DELETE entire method

**Before (lines 43-55):**

```rust
impl XYCutParams {
    /// Create parameters for single-column documents.
    ///
    /// Uses larger vertical gap threshold to avoid false column splits.
    /// This is a heuristic that should be replaced with adaptive calculation.
    #[deprecated(note = "Use segment_adaptive() instead for first-principles approach")]
    pub fn single_column() -> Self {
        Self {
            min_vertical_gap: 100.0, // Large gap to avoid column splits
            min_horizontal_gap: 8.0,
            ..Default::default()
        }
    }

    /// Create parameters for multi-column documents.
```

**After (lines 43):**

```rust
impl XYCutParams {
    // Removed deprecated single_column() and multi_column() methods.
    // Use XYCut::with_defaults() or segment_adaptive() for adaptive thresholds.
}
```

### Change 2: Remove multi_column() Method

**File:** `crates/edgequake-pdf/src/layout/xy_cut.rs`  
**Lines:** 57-67 (11 lines including doc comments)  
**Action:** DELETE entire method (same operation as Change 1)

**Before (lines 57-67):**

```rust
    /// Create parameters for multi-column documents.
    ///
    /// Uses smaller vertical gap threshold to detect columns.
    /// This is a heuristic that should be replaced with adaptive calculation.
    #[deprecated(note = "Use segment_adaptive() instead for first-principles approach")]
    pub fn multi_column() -> Self {
        Self {
            min_vertical_gap: 15.0, // Smaller gap to detect columns
            min_horizontal_gap: 10.0,
            ..Default::default()
        }
    }
}
```

**After:** (deleted, covered by Change 1)

### Combined Change 1+2: Update impl Block

**File:** `crates/edgequake-pdf/src/layout/xy_cut.rs`  
**Lines:** 42-68  
**Action:** Replace entire impl block with empty implementation

**Before (lines 42-68):**

```rust
impl XYCutParams {
    /// Create parameters for single-column documents.
    ///
    /// Uses larger vertical gap threshold to avoid false column splits.
    /// This is a heuristic that should be replaced with adaptive calculation.
    #[deprecated(note = "Use segment_adaptive() instead for first-principles approach")]
    pub fn single_column() -> Self {
        Self {
            min_vertical_gap: 100.0, // Large gap to avoid column splits
            min_horizontal_gap: 8.0,
            ..Default::default()
        }
    }

    /// Create parameters for multi-column documents.
    ///
    /// Uses smaller vertical gap threshold to detect columns.
    /// This is a heuristic that should be replaced with adaptive calculation.
    #[deprecated(note = "Use segment_adaptive() instead for first-principles approach")]
    pub fn multi_column() -> Self {
        Self {
            min_vertical_gap: 15.0, // Smaller gap to detect columns
            min_horizontal_gap: 10.0,
            ..Default::default()
        }
    }
}
```

**After (lines 42-45):**

```rust
impl XYCutParams {
    // Removed deprecated single_column() and multi_column() methods.
    // Use XYCut::with_defaults() or segment_adaptive() for adaptive thresholds.
}
```

### Change 3: Replace test_xy_cut_params with test_adaptive_gap_calculation

**File:** `crates/edgequake-pdf/src/layout/xy_cut.rs`  
**Lines:** 620-628  
**Action:** REPLACE entire test

**Before (lines 620-628):**

```rust
    #[test]
    fn test_xy_cut_params() {
        let single = XYCutParams::single_column();
        assert!(single.min_vertical_gap > 50.0);

        let multi = XYCutParams::multi_column();
        assert!(multi.min_vertical_gap < 30.0);
    }
```

**After (lines 620-643):**

```rust
    #[test]
    fn test_adaptive_gap_calculation() {
        // Test adaptive vertical gap (column detection)
        // Wide column spacing should result in larger gap threshold
        let wide_columns = vec![
            make_bbox(50.0, 50.0, 250.0, 150.0),   // Left column
            make_bbox(350.0, 50.0, 550.0, 150.0),  // Right column (100pt horizontal gap)
        ];
        let vertical_gap = calculate_adaptive_vertical_gap(&wide_columns);
        assert!(
            vertical_gap >= 10.0 && vertical_gap <= 100.0,
            "Adaptive vertical gap {} should be in range [10, 100]",
            vertical_gap
        );

        // Test adaptive horizontal gap (block separation)
        // Vertically spaced blocks should result in smaller gap threshold
        let vertical_blocks = vec![
            make_bbox(50.0, 50.0, 250.0, 100.0),   // Top block
            make_bbox(50.0, 120.0, 250.0, 170.0),  // Bottom block (20pt vertical gap)
        ];
        let horizontal_gap = calculate_adaptive_horizontal_gap(&vertical_blocks);
        assert!(
            horizontal_gap >= 5.0 && horizontal_gap <= 50.0,
            "Adaptive horizontal gap {} should be in range [5, 50]",
            horizontal_gap
        );
    }
```

## Implementation Steps

### Step 1: Remove Deprecated Methods

**Tool:** `replace_string_in_file`  
**Target:** xy_cut.rs lines 42-68  
**Complexity:** Simple (delete block, add comment)  
**Time:** 1 minute

### Step 2: Update Test

**Tool:** `replace_string_in_file`  
**Target:** xy_cut.rs lines 620-628  
**Complexity:** Medium (replace with new test)  
**Time:** 2 minutes

### Step 3: Verify Compilation

**Tool:** `run_in_terminal`  
**Command:** `cargo test --package edgequake-pdf | cat`  
**Expected:** 113/113 tests passing, 2 warnings eliminated  
**Time:** 30 seconds

### Step 4: Document Results

**Tool:** `create_file`  
**Target:** sessions/improve_pdf/010-iteration/ACT.md  
**Content:** Implementation summary, test results, lessons learned  
**Time:** 2 minutes

**Total Estimated Time:** 5-6 minutes

## Expected Outcomes

### Code Changes

- **Lines Deleted:** 26 (deprecated methods + old test)
- **Lines Added:** 26 (new test + comment)
- **Net Change:** ~0 lines (replacement)
- **Magic Numbers Eliminated:** 4
- **Compiler Warnings Eliminated:** 2

### Test Results

- **Before:** 113 tests passing, 2 deprecation warnings
- **After:** 113 tests passing, 0 warnings
- **New Test:** `test_adaptive_gap_calculation` validates adaptive functions

### Code Quality Improvements

1. ✅ Eliminated 4 magic numbers (100.0, 8.0, 15.0, 10.0)
2. ✅ Removed deprecated dead code
3. ✅ Improved test coverage (tests actual adaptive functions)
4. ✅ Eliminated compiler warnings
5. ✅ Simpler codebase (less code = less maintenance)

## Risk Assessment

### Risk 1: External Code Dependency

**Likelihood:** Very Low  
**Check:** Grep search shows only test usage  
**Mitigation:** If discovered, can revert or apply Option 4 (modern defaults)  
**Recovery Time:** 2 minutes (git revert)

### Risk 2: Test Failures

**Likelihood:** Very Low  
**Check:** New test uses existing helper functions (make_bbox)  
**Mitigation:** New test has clear assertions with debug output  
**Recovery Time:** 5 minutes (debug and fix assertions)

### Risk 3: Adaptive Functions Have Bugs

**Likelihood:** Very Low (functions already exist and used)  
**Check:** New test will validate them directly  
**Mitigation:** Tests provide concrete examples for debugging  
**Recovery Time:** 10 minutes (investigate and fix if needed)

## Validation Criteria

### Compilation Success ✅

```bash
$ cargo test --package edgequake-pdf | cat
Compiling edgequake-pdf v0.1.0
```

**Expected:** No errors, 2 deprecation warnings eliminated

### Test Suite Success ✅

```bash
test result: ok. 113 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

**Expected:** All tests passing, including new `test_adaptive_gap_calculation`

### Warning Elimination ✅

**Before:**

```
warning: use of deprecated associated function `...::single_column`
warning: use of deprecated associated function `...::multi_column`
```

**After:** No deprecation warnings (0 warnings from xy_cut.rs)

### Test Coverage ✅

**New Test Validates:**

- `calculate_adaptive_vertical_gap()` returns values in reasonable range [10, 100]
- `calculate_adaptive_horizontal_gap()` returns values in reasonable range [5, 50]
- Adaptive functions handle typical layouts (columns, vertical blocks)

## Rollback Plan

**If Problems Occur:**

1. **Immediate:** `git revert` last commit
2. **Alternative:** Apply Option 4 (modern defaults) from ORIENT phase
3. **Investigation:** Debug issue, create new plan
4. **Resume:** Continue with corrected approach

**Recovery Time:** 2-10 minutes depending on issue

## Post-Implementation Actions

1. ✅ Create ACT.md documentation
2. ✅ Update Loop 010 status to COMPLETE
3. ✅ Verify cumulative magic numbers eliminated: 15 (11 previous + 4 this loop)
4. ✅ Begin Loop 011 OBSERVE phase
5. ⚠️ Consider validation run after 2-3 more loops

## Implementation Authorization

**Status:** ✅ APPROVED for implementation  
**Confidence Level:** High (95%)  
**Risk Level:** Low  
**Estimated Time:** 5-6 minutes  
**Reversibility:** High (easy git revert)

**Proceed to ACT Phase:** YES

---

**Loop 010 Status:** DECIDE Complete  
**Next Phase:** ACT (implementation)  
**Implementation Method:** multi_replace_string_in_file (2 replacements)  
**Expected Duration:** 5-6 minutes  
**Go/No-Go Decision:** ✅ GO
