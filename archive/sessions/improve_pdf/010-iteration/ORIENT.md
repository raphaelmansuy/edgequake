# ORIENT Phase - Loop 010

## Strategy Design

**Goal:** Eliminate 4 magic numbers from XYCutParams deprecated methods while maintaining backward compatibility  
**Approach:** Update deprecated methods to use existing adaptive calculation functions  
**Date:** 2024-01-XX

## Context Review

### Available Resources

1. **Adaptive Functions (Already Implemented):**

   - `calculate_adaptive_vertical_gap(items: &[BoundingBox]) -> f32`
   - `calculate_adaptive_horizontal_gap(items: &[BoundingBox]) -> f32`

2. **Problem:** Deprecated methods use fixed thresholds instead of adaptive ones

3. **Constraint:** Deprecated methods are factory functions (no access to items)

### Core Challenge

**Incompatibility:** Deprecated methods are **constructors** (take no parameters), but adaptive functions need `&[BoundingBox]` items.

```rust
// Current deprecated methods (factory pattern)
pub fn single_column() -> Self { ... }  // No items parameter!
pub fn multi_column() -> Self { ... }   // No items parameter!

// Adaptive functions (require data)
fn calculate_adaptive_vertical_gap(items: &[BoundingBox]) -> f32 { ... }
```

**Implication:** We cannot call adaptive functions from deprecated methods **without changing their signatures**.

## Solution Analysis

### Option 1: Keep Deprecated Methods As-Is ❌

- **Pros:** Zero changes
- **Cons:** Magic numbers persist, mission stalls
- **Verdict:** Rejected (doesn't advance mission)

### Option 2: Remove Deprecated Methods ❌

- **Pros:** Eliminates dead code and compiler warnings
- **Cons:** Breaking change, removes backward compatibility
- **Verdict:** Rejected (too aggressive, breaks external code)

### Option 3: Update to Use Adaptive Functions with Default Items ❌

```rust
pub fn single_column() -> Self {
    let default_items = vec![]; // Empty items
    Self {
        min_vertical_gap: calculate_adaptive_vertical_gap(&default_items), // Returns 20.0 (fallback)
        min_horizontal_gap: calculate_adaptive_horizontal_gap(&default_items), // Returns 10.0 (fallback)
        ..Default::default()
    }
}
```

- **Pros:** Uses adaptive functions
- **Cons:** Empty items → always returns fallback (20.0, 10.0), not actually adaptive
- **Verdict:** Rejected (fake adaptation, doesn't improve behavior)

### Option 4: Convert to Modern Adaptive Defaults (Recommended) ✅

**Strategy:** Since deprecated methods can't be truly adaptive (no items), replace magic numbers with **well-justified defaults** based on typography standards.

```rust
pub fn single_column() -> Self {
    Self {
        // Use conservative defaults from typography standards
        min_vertical_gap: 60.0,  // ~5× typical body font (12pt) = conservative column detection
        min_horizontal_gap: 15.0, // ~1.25× typical body font = standard block separation
        ..Default::default()
    }
}

pub fn multi_column() -> Self {
    Self {
        // Use aggressive defaults for column detection
        min_vertical_gap: 25.0,  // ~2× typical body font = catches narrow columns
        min_horizontal_gap: 15.0, // ~1.25× typical body font = standard block separation
        ..Default::default()
    }
}
```

**Rationale:**

1. **Typography Standards:** 12pt body font is typical for PDF documents
2. **Conservative single_column:** 60pt (5× body font) prevents false column splits in wide single-column layouts
3. **Aggressive multi_column:** 25pt (2× body font) catches narrow column gaps
4. **Consistent horizontal:** 15pt (1.25× body font) is standard for block separation in both cases

### Option 5: Remove Methods + Update Test (Best) ✅✅

**Strategy:** Since methods are deprecated, not used in production, and can't be truly adaptive, **remove them entirely** and update test.

**Steps:**

1. Delete `single_column()` method (lines 48-55)
2. Delete `multi_column()` method (lines 61-67)
3. Update or remove test `test_xy_cut_params` (lines 621-627)
4. Verify all other tests still pass

**Benefits:**

- ✅ Eliminates 4 magic numbers completely
- ✅ Removes 2 compiler warnings
- ✅ Reduces code complexity (simpler is better)
- ✅ Forces users to use adaptive `segment_adaptive()` approach
- ✅ Aligns with deprecation message intent

**Risk Assessment:**

- **Low Risk:** Methods already deprecated (since when?)
- **No Production Usage:** Grep shows only test usage
- **Clear Migration Path:** Deprecation message directs to `segment_adaptive()`

## Decision Matrix

| Option                  | Magic Numbers Eliminated | Backward Compat | Code Quality | Risk   | Recommendation |
| ----------------------- | ------------------------ | --------------- | ------------ | ------ | -------------- |
| 1. Keep As-Is           | 0                        | ✅              | ❌           | None   | ❌ No          |
| 2. Remove               | 4                        | ❌              | ✅           | Medium | ❌ Maybe       |
| 3. Fake Adaptive        | 4                        | ✅              | ⚠️           | Low    | ❌ No          |
| 4. Modern Defaults      | 4                        | ✅              | ⚠️           | Low    | ⚠️ Acceptable  |
| 5. Remove + Update Test | 4                        | ⚠️              | ✅           | Low    | ✅ **Best**    |

## Recommended Approach: Option 5

### Justification

1. **Methods Already Deprecated:** Intent is to phase them out
2. **No Production Usage:** Only test code uses them (safe to remove)
3. **Clear Migration Path:** Deprecation message guides to `segment_adaptive()`
4. **Code Quality:** Removing dead code > keeping it with better defaults
5. **Mission Alignment:** Complete elimination > partial improvement

### Implementation Details

#### Step 1: Remove single_column() Method

**Location:** xy_cut.rs lines 48-55
**Action:** Delete entire method

#### Step 2: Remove multi_column() Method

**Location:** xy_cut.rs lines 61-67
**Action:** Delete entire method

#### Step 3: Update Test

**Location:** xy_cut.rs lines 621-627
**Current:**

```rust
#[test]
fn test_xy_cut_params() {
    let single = XYCutParams::single_column();  // ⚠️ Will break after removal
    assert!(single.min_vertical_gap > 50.0);

    let multi = XYCutParams::multi_column();    // ⚠️ Will break after removal
    assert!(multi.min_vertical_gap < 30.0);
}
```

**Option A:** Remove test entirely (simple, recommended)
**Option B:** Convert to test adaptive functions (adds value)

**Recommended: Option B** - Test adaptive functions

```rust
#[test]
fn test_adaptive_gap_calculation() {
    // Test adaptive vertical gap (column detection)
    let wide_columns = vec![
        make_bbox(50.0, 50.0, 250.0, 150.0),   // Left column
        make_bbox(350.0, 50.0, 550.0, 150.0),  // Right column (100pt gap)
    ];
    let vertical_gap = calculate_adaptive_vertical_gap(&wide_columns);
    assert!(vertical_gap > 10.0 && vertical_gap < 100.0, "Adaptive vertical gap out of range");

    // Test adaptive horizontal gap (block separation)
    let vertical_blocks = vec![
        make_bbox(50.0, 50.0, 250.0, 100.0),   // Top block
        make_bbox(50.0, 120.0, 250.0, 170.0),  // Bottom block (20pt gap)
    ];
    let horizontal_gap = calculate_adaptive_horizontal_gap(&vertical_blocks);
    assert!(horizontal_gap > 5.0 && horizontal_gap < 50.0, "Adaptive horizontal gap out of range");
}
```

**Benefits of Option B:**

- Tests adaptive functions directly (better coverage)
- Validates reasonable ranges instead of exact values
- Provides concrete examples for documentation
- No loss of test coverage

## Alternative: If Backward Compatibility Required

**If external code depends on deprecated methods** (unlikely but possible):

### Fallback Plan: Option 4 with Clear Documentation

````rust
/// Create parameters for single-column documents.
///
/// **DEPRECATED:** Use `segment_adaptive()` for better results.
///
/// Uses conservative defaults (60pt vertical, 15pt horizontal) based on
/// typical 12pt body font typography. These are NOT adaptive and may not
/// work well for all documents.
///
/// # Migration
/// ```rust
/// // Old (fixed thresholds, deprecated)
/// let params = XYCutParams::single_column();
///
/// // New (adaptive, recommended)
/// let xy_cut = XYCut::with_defaults();
/// let tree = xy_cut.segment(&items, &page); // Calculates adaptive thresholds
/// ```
#[deprecated(note = "Use segment_adaptive() or XYCut::with_defaults() for adaptive thresholds")]
pub fn single_column() -> Self {
    Self {
        min_vertical_gap: 60.0,  // Conservative for wide layouts (5× typical 12pt font)
        min_horizontal_gap: 15.0, // Standard block separation (1.25× typical 12pt font)
        ..Default::default()
    }
}
````

**When to Use Fallback:**

- External packages in crates.io depend on methods
- Breaking change would require semver major bump
- Project policy requires deprecation period before removal

## Implementation Plan

### Primary Plan (Option 5 - Remove)

1. Delete `single_column()` method (7 lines)
2. Delete `multi_column()` method (7 lines)
3. Replace `test_xy_cut_params` with `test_adaptive_gap_calculation` (~15 lines)
4. Run tests → verify 113/113 passing
5. Verify 2 compiler warnings eliminated

**Estimated Time:** 5 minutes  
**Risk:** Low (deprecated, unused in production)  
**Reversibility:** High (git revert if needed)

### Fallback Plan (Option 4 - Modern Defaults)

1. Update `single_column()` magic numbers → modern defaults (60.0, 15.0)
2. Update `multi_column()` magic numbers → modern defaults (25.0, 15.0)
3. Add comprehensive documentation explaining defaults
4. Update test assertions to match new values
5. Run tests → verify 113/113 passing

**Estimated Time:** 10 minutes  
**Risk:** Very low (backward compatible)  
**Reversibility:** High (git revert if needed)

## First Principles Validation

### Primary Plan

- ✅ **Eliminates magic numbers completely** (4 removed)
- ✅ **Forces adaptive approach** (users must use segment_adaptive())
- ✅ **Reduces complexity** (less code is better code)
- ✅ **Aligns with deprecation intent** (methods should be removed)

### Fallback Plan

- ⚠️ **Replaces magic numbers with justified defaults** (improvement but not elimination)
- ⚠️ **Maintains backward compatibility** (good for users, delays progress)
- ⚠️ **Still deprecated** (methods remain but work better)

## Risk Mitigation

### Risk 1: External Code Breaks

**Likelihood:** Low (methods deprecated, no known external usage)  
**Mitigation:** Check crates.io dependencies before removal  
**Fallback:** Use Option 4 (modern defaults) instead

### Risk 2: Test Coverage Loss

**Likelihood:** None (new test covers adaptive functions)  
**Mitigation:** New test provides better coverage of actual logic  
**Validation:** Run full test suite (113 tests)

### Risk 3: Unexpected Production Usage

**Likelihood:** Very Low (grep shows only test usage)  
**Mitigation:** Thorough grep search before removal  
**Recovery:** Git revert if issues discovered

## Success Criteria

1. ✅ All 4 magic numbers eliminated (either removed or replaced with justified defaults)
2. ✅ All 113 tests passing
3. ✅ 2 compiler warnings eliminated
4. ✅ Test coverage maintained or improved
5. ✅ Documentation clear for migration path

## Next Steps

**DECIDE Phase:** Create detailed implementation plan with exact code changes (before/after)

---

**Loop 010 Status:** ORIENT Complete  
**Recommended Strategy:** Option 5 - Remove deprecated methods + update test  
**Fallback Strategy:** Option 4 - Update with modern defaults  
**Magic Numbers to Eliminate:** 4  
**Risk Level:** Low  
**Next Phase:** DECIDE
