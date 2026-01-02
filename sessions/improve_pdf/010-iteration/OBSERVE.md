# OBSERVE Phase - Loop 010

## Target Identification

**File:** `crates/edgequake-pdf/src/layout/xy_cut.rs`  
**Scope:** XYCutParams deprecated methods (single_column, multi_column)  
**Line Range:** 43-67  
**Date:** 2024-01-XX

## Current Implementation

### Deprecated Method: single_column() (lines 48-55)

```rust
#[deprecated(note = "Use segment_adaptive() instead for first-principles approach")]
pub fn single_column() -> Self {
    Self {
        min_vertical_gap: 100.0, // Large gap to avoid column splits  [MAGIC NUMBER]
        min_horizontal_gap: 8.0,                                      [MAGIC NUMBER]
        ..Default::default()
    }
}
```

**Magic Numbers Identified:**

1. `min_vertical_gap: 100.0` - Fixed threshold to prevent column detection
2. `min_horizontal_gap: 8.0` - Fixed threshold for block separation

### Deprecated Method: multi_column() (lines 61-67)

```rust
#[deprecated(note = "Use segment_adaptive() instead for first-principles approach")]
pub fn multi_column() -> Self {
    Self {
        min_vertical_gap: 15.0, // Smaller gap to detect columns       [MAGIC NUMBER]
        min_horizontal_gap: 10.0,                                      [MAGIC NUMBER]
        ..Default::default()
    }
}
```

**Magic Numbers Identified:** 3. `min_vertical_gap: 15.0` - Fixed threshold to enable column detection 4. `min_horizontal_gap: 10.0` - Fixed threshold for block separation

### Total Magic Numbers: 4

## Problem Analysis

### Issue: Fixed Thresholds Don't Scale

These deprecated methods use fixed pixel values that don't adapt to:

- **Font Size:** 8pt vs 24pt documents need different thresholds
- **Page Size:** Letter (612×792) vs A4 (595×842) vs Legal (612×1008)
- **Line Spacing:** Single-spaced vs double-spaced layouts
- **Column Width:** Narrow (1.5in) vs wide (3in) columns

### Concrete Examples

#### Example 1: Small Font Document (8pt)

- **Line spacing:** ~11pt
- **single_column():** min_horizontal_gap = 8.0pt (0.7× line spacing) ❌ TOO SMALL
- **Expected:** ~16.5pt (1.5× line spacing)
- **Result:** Adjacent lines may be incorrectly split into separate blocks

#### Example 2: Large Font Document (24pt)

- **Line spacing:** ~34pt
- **multi_column():** min_horizontal_gap = 10.0pt (0.3× line spacing) ❌ TOO SMALL
- **Expected:** ~51pt (1.5× line spacing)
- **Result:** Vertically adjacent paragraphs may be incorrectly split

#### Example 3: Wide Column Spacing (2-column, 100pt gap)

- **Column gap:** 100pt
- **multi_column():** min_vertical_gap = 15.0pt ❌ TOO SMALL (15% of actual gap)
- **Expected:** ~50-70pt (0.5-0.7× actual column gap)
- **Result:** May fail to detect columns, treat as single-column layout

#### Example 4: Narrow Column Spacing (3-column, 40pt gap)

- **Column gap:** 40pt
- **single_column():** min_vertical_gap = 100.0pt ❌ TOO LARGE (2.5× actual gap)
- **Expected:** ~20-28pt (0.5-0.7× actual column gap)
- **Result:** Correctly avoids false column splits (but for wrong reasons)

### Current Adaptive Methods

The file already contains adaptive calculation functions:

- `calculate_adaptive_vertical_gap()` (lines 151-173) - 15th percentile of horizontal distances
- `calculate_adaptive_horizontal_gap()` (lines 185-207) - 15th percentile of vertical distances

**These functions exist but the deprecated methods aren't using them!**

## Usage Analysis

### Test Usage (xy_cut.rs:621-624)

```rust
#[test]
fn test_xy_cut_params() {
    let single = XYCutParams::single_column();  // ⚠️ DEPRECATED
    assert!(single.min_vertical_gap > 50.0);

    let multi = XYCutParams::multi_column();    // ⚠️ DEPRECATED
    assert!(multi.min_vertical_gap < 30.0);
}
```

**Status:** Test validates deprecated behavior, should be updated

### Production Usage

No production code currently calls these deprecated methods. They're marked deprecated with guidance to use `segment_adaptive()` instead.

## Compiler Warnings

```
warning: use of deprecated associated function `layout::xy_cut::XYCutParams::single_column`:
Use segment_adaptive() instead for first-principles approach
   --> crates/edgequake-pdf/src/layout/xy_cut.rs:621:35
    |
621 |         let single = XYCutParams::single_column();
    |                                   ^^^^^^^^^^^^^

warning: use of deprecated associated function `layout::xy_cut::XYCutParams::multi_column`:
Use segment_adaptive() instead for first-principles approach
   --> crates/edgequake-pdf/src/layout/xy_cut.rs:624:34
    |
624 |         let multi = XYCutParams::multi_column();
    |                                  ^^^^^^^^^^^^
```

**Impact:** 2 compiler warnings from test code

## Adaptive Functions Available

### calculate_adaptive_vertical_gap() (lines 151-173)

```rust
fn calculate_adaptive_vertical_gap(items: &[BoundingBox]) -> f32 {
    if items.len() < 2 {
        return 20.0; // Default fallback
    }
    // Calculate horizontal distances between adjacent items
    let mut distances = Vec::new();
    for i in 0..items.len() {
        for j in (i + 1)..items.len() {
            let dist = (items[i].x1 - items[j].x1).abs();
            distances.push(dist);
        }
    }
    // Sort and use 15th percentile to capture typical column gaps
    distances.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let percentile_idx = (distances.len() as f32 * 0.15) as usize;
    let gap = distances.get(percentile_idx).copied().unwrap_or(20.0);
    // Clamp to reasonable range
    gap.max(10.0).min(100.0)
}
```

**Purpose:** Calculate column gap threshold from actual layout
**Method:** 15th percentile of horizontal distances (robust against outliers)
**Clamping:** 10.0-100.0pt range

### calculate_adaptive_horizontal_gap() (lines 185-207)

```rust
fn calculate_adaptive_horizontal_gap(items: &[BoundingBox]) -> f32 {
    if items.len() < 2 {
        return 10.0; // Default fallback
    }
    // Calculate vertical distances between adjacent items
    let mut distances = Vec::new();
    for i in 0..items.len() {
        for j in (i + 1)..items.len() {
            let dist = (items[i].y1 - items[j].y1).abs();
            distances.push(dist);
        }
    }
    // Similar logic...
}
```

**Purpose:** Calculate block gap threshold from actual layout
**Method:** 15th percentile of vertical distances (robust against outliers)

## First Principles Assessment

### Current State: ❌ Violates First Principles

- Fixed pixel values (100.0, 8.0, 15.0, 10.0) don't scale
- Heuristic-based (guessing column vs non-column layouts)
- Deprecated with recommendation to use adaptive approach

### Adaptive Functions: ✅ Follow First Principles

- Measure actual spacing from document layout
- Use percentiles (robust against outliers)
- Clamp to reasonable ranges (prevent extreme values)

### Opportunity

**Adaptive functions already exist!** They're just not used by the deprecated methods. We can:

1. Keep deprecated methods for backward compatibility
2. Update them to call adaptive functions internally
3. Update test to verify adaptive behavior instead of fixed values

## Impact Assessment

### Code Quality

- **Compiler Warnings:** 2 (test usage of deprecated methods)
- **Dead Code:** Deprecated methods kept for backward compatibility but not used
- **Test Coverage:** Test validates deprecated behavior, needs update

### User Impact

- **Low:** Methods already deprecated, no production usage
- **Documentation:** Deprecation message guides users to segment_adaptive()
- **Migration Path:** Clear (use segment_adaptive())

### Complexity

- **Simple Fix:** Update deprecated methods to use adaptive functions
- **Test Update:** Minimal (change assertions to check adaptive behavior)
- **Risk:** Very low (methods already deprecated, limited usage)

## Decision Points

### Option A: Remove Deprecated Methods (Aggressive)

- **Pros:** Eliminates dead code, removes compiler warnings
- **Cons:** Breaking change for external code, removes backward compatibility

### Option B: Update Deprecated Methods to Use Adaptive Functions (Recommended)

- **Pros:** Maintains backward compatibility, eliminates magic numbers, keeps deprecation
- **Cons:** Keeps deprecated methods in codebase (but with better implementation)

### Option C: Leave As-Is (Status Quo)

- **Pros:** No risk, maintains exact behavior
- **Cons:** Keeps magic numbers, compiler warnings persist, doesn't advance mission

## Recommendation

**Option B: Update deprecated methods to use adaptive functions**

Rationale:

1. **Backward Compatible:** External code using deprecated methods continues to work
2. **Better Behavior:** Adaptive thresholds improve quality even for deprecated paths
3. **Consistent:** Aligns with First Principles mission without breaking changes
4. **Progressive:** Keeps deprecation warnings, encourages migration
5. **Simple:** Minimal code changes, easy to verify

## Next Steps

1. **ORIENT:** Design how to integrate adaptive functions into deprecated methods
2. **DECIDE:** Create implementation plan with before/after code
3. **ACT:** Update deprecated methods, update test, verify all tests pass
4. **DOCUMENT:** Create ACT.md with lessons learned

---

**Loop 010 Status:** OBSERVE Complete  
**Magic Numbers Identified:** 4  
**Recommended Approach:** Update deprecated methods to use adaptive functions  
**Risk Level:** Low (deprecated, limited usage)  
**Next Phase:** ORIENT
