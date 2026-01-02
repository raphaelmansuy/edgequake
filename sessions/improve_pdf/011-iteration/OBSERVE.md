# OBSERVE Phase - Loop 011

## Target Identification

**File:** `crates/edgequake-pdf/src/layout/xy_cut.rs`  
**Scope:** XYCutParams::Default implementation  
**Line Range:** 29-39  
**Date:** 2026-01-02

## Current Implementation

### Default Implementation (lines 29-39)

```rust
impl Default for XYCutParams {
    fn default() -> Self {
        Self {
            min_vertical_gap: 20.0,   // Will be overridden by adaptive calculation   [MAGIC NUMBER]
            min_horizontal_gap: 10.0, // Will be overridden by adaptive calculation   [MAGIC NUMBER]
            min_region_width: 50.0,                                                    [MAGIC NUMBER]
            min_region_height: 20.0,                                                   [MAGIC NUMBER]
            max_depth: 10,
            prefer_horizontal: true,
        }
    }
}
```

**Magic Numbers Identified:**

1. `min_vertical_gap: 20.0` - Fixed column detection threshold
2. `min_horizontal_gap: 10.0` - Fixed block separation threshold
3. `min_region_width: 50.0` - Fixed minimum region width
4. `min_region_height: 20.0` - Fixed minimum region height

**Total Magic Numbers:** 4

## Comments Analysis

### Optimistic Comments ⚠️

```rust
min_vertical_gap: 20.0,   // Will be overridden by adaptive calculation
min_horizontal_gap: 10.0, // Will be overridden by adaptive calculation
```

**Reality Check:** These comments are **misleading**!

### Usage Pattern Analysis

**Method 1: segment() - Does NOT Override**

```rust
pub fn segment(&self, items: &[BoundingBox], page_bbox: &BoundingBox) -> XYCutNode {
    let indices: Vec<usize> = (0..items.len()).collect();
    self.segment_recursive(items, &indices, page_bbox, 0)  // Uses self.params directly!
}
```

**Method 2: segment_adaptive() - DOES Override**

```rust
pub fn segment_adaptive(&self, items: &[BoundingBox], page_bbox: &BoundingBox) -> XYCutNode {
    let adaptive_params = XYCutParams {
        min_vertical_gap: calculate_adaptive_vertical_gap(items),       // ✅ ADAPTIVE
        min_horizontal_gap: calculate_adaptive_horizontal_gap(items),   // ✅ ADAPTIVE
        ..self.params.clone()  // ⚠️ Still copies magic numbers for min_region_*
    };
    // ...
}
```

### Misleading Comment Impact

**Problem:** Comments say "will be overridden by adaptive calculation" but:

1. **segment()** never overrides - uses default values directly
2. **segment_adaptive()** only overrides gap values, not region min sizes
3. Users may assume defaults are "good enough" based on comments

**Evidence:**

```rust
// Constructor usage
let xy_cut = XYCut::with_defaults();  // ← Uses XYCutParams::default()
xy_cut.segment(&items, &page);        // ← Uses magic numbers 20.0, 10.0, 50.0, 20.0!
```

## Problem Analysis

### Issue 1: segment() Uses Magic Numbers Directly

When users call `segment()` (not `segment_adaptive()`), they get fixed thresholds:

- `min_vertical_gap: 20.0` - May be too large or too small depending on document
- `min_horizontal_gap: 10.0` - May miss blocks or over-split depending on font size

### Issue 2: Region Size Thresholds Never Adaptive

Both `segment()` and `segment_adaptive()` use fixed region sizes:

- `min_region_width: 50.0` - Too small for large fonts, too large for small fonts
- `min_region_height: 20.0` - Same scaling problem

### Issue 3: Misleading Documentation

Comments suggest adaptive calculation will happen, but it only happens if user explicitly calls `segment_adaptive()`.

## Concrete Examples

### Example 1: Small Font Document (8pt)

- **Body font:** 8pt
- **Typical line spacing:** ~11pt
- **Default min_horizontal_gap:** 10.0pt (0.9× line spacing) ✅ REASONABLE
- **Default min_region_height:** 20.0pt (1.8× line spacing) ✅ ACCEPTABLE

### Example 2: Large Font Document (24pt)

- **Body font:** 24pt
- **Typical line spacing:** ~34pt
- **Default min_horizontal_gap:** 10.0pt (0.3× line spacing) ❌ TOO SMALL
  - May split single paragraph into multiple blocks
- **Default min_region_height:** 20.0pt (0.6× line spacing) ❌ TOO SMALL
  - May create invalid regions smaller than one line of text

### Example 3: Wide Column Layout (2-column, 100pt gap)

- **Column gap:** 100pt
- **Default min_vertical_gap:** 20.0pt (0.2× actual gap) ❌ TOO SMALL
  - May fail to detect columns, treat as single-column
- **Result:** Incorrect reading order

### Example 4: Narrow Column Layout (3-column, 40pt gap)

- **Column gap:** 40pt
- **Default min_vertical_gap:** 20.0pt (0.5× actual gap) ⚠️ BORDERLINE
  - May work but not robust
- **Default min_region_width:** 50.0pt ❌ TOO LARGE
  - May reject valid narrow columns

## Usage Analysis

### Production Usage

```bash
$ grep -r "with_defaults\|segment(" crates/edgequake-pdf/src/
```

**Findings:**

1. **Tests use segment():** 5 occurrences in xy_cut.rs tests
2. **Tests use with_defaults():** 3 occurrences in xy_cut.rs tests
3. **Default impl used by:** XYCut::with_defaults() constructor

**Critical:** Tests rely on Default behavior, may be validating incorrect fixed thresholds!

### Test Code Analysis (lines 505, 553, 627)

```rust
#[test]
fn test_xy_cut_two_items_horizontal_gap() {
    let xy_cut = XYCut::with_defaults();  // ← Uses magic numbers!
    let items = vec![
        make_bbox(50.0, 50.0, 250.0, 150.0),
        make_bbox(300.0, 50.0, 500.0, 150.0),
    ];
    let page = make_bbox(0.0, 0.0, 612.0, 792.0);
    let tree = xy_cut.segment(&items, &page);  // ← segment() not segment_adaptive()
    // ...
}
```

**Problem:** Tests use fixed defaults, may pass for specific test cases but fail on real documents!

## Comparison with Previous Loops

### Loop 007 (BlockMergeProcessor)

- **Approach:** Add DocumentStats parameter to should_merge()
- **Result:** Adaptive thresholds based on document analysis

### Loop 009 (HyphenContinuationProcessor)

- **Approach:** Calculate DocumentStats once in process()
- **Result:** Adaptive thresholds based on document analysis

### Loop 010 (XYCutParams deprecated methods)

- **Approach:** Remove deprecated methods entirely
- **Result:** Force users to use adaptive functions

### Loop 011 (XYCutParams::Default) - THIS LOOP

- **Challenge:** Default impl can't calculate adaptive values (no items parameter)
- **Options:**
  1. Replace with reasonable defaults based on typography
  2. Make defaults conservative (won't break)
  3. Deprecate with_defaults(), force users to pass items

## First Principles Assessment

### Current State: ❌ Violates First Principles

- Fixed pixel values (20.0, 10.0, 50.0, 20.0) don't scale
- Comments are misleading (suggest adaptation that doesn't always happen)
- segment() method uses fixed values directly

### Adaptive Methods Exist: ✅ Available

- `calculate_adaptive_vertical_gap(items)`
- `calculate_adaptive_horizontal_gap(items)`
- `segment_adaptive()` method demonstrates correct approach

### Constraint: Cannot Make Default Truly Adaptive

- Default trait requires no parameters
- Cannot calculate adaptive values without items
- Same issue as Loop 010 deprecated methods

## Decision Points

### Option A: Replace with Typography-Based Defaults (Conservative)

```rust
impl Default for XYCutParams {
    fn default() -> Self {
        Self {
            // Conservative defaults based on 12pt body font typography
            min_vertical_gap: 25.0,  // ~2× typical body font
            min_horizontal_gap: 15.0, // ~1.25× typical body font
            min_region_width: 60.0,   // ~5× typical body font
            min_region_height: 24.0,  // ~2× typical body font
            max_depth: 10,
            prefer_horizontal: true,
        }
    }
}
```

**Pros:** Better defaults, scales better across fonts  
**Cons:** Still magic numbers, just "better" ones

### Option B: Add Comment Warning + Keep Values

```rust
impl Default for XYCutParams {
    fn default() -> Self {
        // WARNING: These are fallback values only. For best results,
        // use segment_adaptive() which calculates thresholds from
        // actual document layout instead of fixed defaults.
        Self {
            min_vertical_gap: 20.0,  // Fallback only
            min_horizontal_gap: 10.0, // Fallback only
            min_region_width: 50.0,   // Fallback only
            min_region_height: 20.0,  // Fallback only
            max_depth: 10,
            prefer_horizontal: true,
        }
    }
}
```

**Pros:** Honest documentation, maintains backward compatibility  
**Cons:** Doesn't eliminate magic numbers, just documents them

### Option C: Make XYCut::new() Require Items for Adaptation (Breaking Change)

```rust
impl XYCut {
    /// Create XY-cut with adaptive parameters calculated from items.
    pub fn new_adaptive(items: &[BoundingBox]) -> Self {
        let params = XYCutParams {
            min_vertical_gap: calculate_adaptive_vertical_gap(items),
            min_horizontal_gap: calculate_adaptive_horizontal_gap(items),
            min_region_width: calculate_adaptive_region_width(items),
            min_region_height: calculate_adaptive_region_height(items),
            max_depth: 10,
            prefer_horizontal: true,
        };
        Self { params }
    }
}
```

**Pros:** Forces adaptation, eliminates magic numbers completely  
**Cons:** Breaking change, requires new adaptive functions for region sizes

### Option D: Update segment() to Always Use Adaptive (Behavioral Change)

```rust
pub fn segment(&self, items: &[BoundingBox], page_bbox: &BoundingBox) -> XYCutNode {
    // Always calculate adaptive parameters (First Principles!)
    let adaptive_params = XYCutParams {
        min_vertical_gap: calculate_adaptive_vertical_gap(items),
        min_horizontal_gap: calculate_adaptive_horizontal_gap(items),
        ..self.params.clone()
    };

    let adaptive_xy_cut = XYCut::new(adaptive_params);
    let indices: Vec<usize> = (0..items.len()).collect();
    adaptive_xy_cut.segment_recursive(items, &indices, page_bbox, 0)
}
```

**Pros:** Makes all segment() calls adaptive automatically  
**Cons:** Behavioral change, may affect tests

## Recommendation

**Hybrid Approach (D + A):** Update segment() to always adapt + improve Default fallbacks

1. **Immediate (Loop 011):** Update segment() to always calculate adaptive gaps
2. **Secondary (Loop 011):** Update Default with better typography-based values
3. **Future (Loop 012):** Create adaptive functions for min*region*\* values

**Rationale:**

- Option D achieves First Principles goal (measure, don't guess)
- Option A provides better fallbacks for edge cases
- Maintains backward compatibility (Default still works)
- Progressive improvement (complete in Loop 012)

## Next Steps

1. **ORIENT:** Design implementation of segment() adaptive behavior
2. **DECIDE:** Create detailed before/after code
3. **ACT:** Implement changes and update tests
4. **DOCUMENT:** Create ACT.md with validation

---

**Loop 011 Status:** OBSERVE Complete  
**Magic Numbers Identified:** 4  
**Recommended Approach:** Hybrid (Update segment() + Better defaults)  
**Risk Level:** Medium (behavioral change in segment())  
**Next Phase:** ORIENT
