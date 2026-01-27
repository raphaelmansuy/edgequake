# Loop 013 - DECIDE Phase

## Decision: Rewrite extract_text_in_rect() with Geometric Containment

Based on ORIENT analysis, the fix is straightforward: implement strict geometric containment for cell-text assignment using text element bounding boxes instead of center points.

## Implementation Strategy

### Approach: Bbox-Based Containment with Minimal Tolerance

**Core Algorithm:**

```rust
fn extract_text_in_rect(
    &self,
    text_elements: &[TextElement],
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
) -> String {
    // 1. Filter text elements using bbox containment with tight tolerance
    let tol = 0.5; // Reduced from 2.0pt to 0.5pt

    let mut contained: Vec<&TextElement> = text_elements
        .iter()
        .filter(|elem| {
            // Use bbox for containment check
            elem.bbox.x0 >= min_x - tol &&
            elem.bbox.x1 <= max_x + tol &&
            elem.bbox.y0 >= min_y - tol &&
            elem.bbox.y1 <= max_y + tol
        })
        .collect();

    // 2. Sort by Y (descending), then X (ascending) - NO BINNING
    contained.sort_by(|a, b| {
        // Use actual Y coordinates, not binned values
        if (a.bbox.y0 - b.bbox.y0).abs() < 0.1 {
            // Same row (within 0.1pt) - sort by X
            a.bbox.x0.partial_cmp(&b.bbox.x0)
                .unwrap_or(std::cmp::Ordering::Equal)
        } else {
            // Different rows - sort by Y descending (top to bottom)
            b.bbox.y0.partial_cmp(&a.bbox.y0)
                .unwrap_or(std::cmp::Ordering::Equal)
        }
    });

    // 3. Concatenate text with spaces
    let mut text = String::new();
    for (i, elem) in contained.iter().enumerate() {
        if i > 0 {
            text.push(' ');
        }
        text.push_str(&elem.text);
    }
    text
}
```

### Key Changes from Current Implementation

| Aspect            | Before (BROKEN)              | After (FIXED)                                    | Rationale                       |
| ----------------- | ---------------------------- | ------------------------------------------------ | ------------------------------- |
| **Filtering**     | `elem.x, elem.y` center      | `elem.bbox.x0, y0, x1, y1`                       | Correct geometric containment   |
| **Tolerance**     | 2.0pt (±2pt = 4pt slop)      | 0.5pt (±0.5pt = 1pt slop)                        | Prevent adjacent cell spillover |
| **Sorting**       | Y-binning: `(y/5.0).round()` | Exact Y: `bbox.y0` with 0.1pt same-row threshold | Eliminate row merging artifacts |
| **Row detection** | Implicit via binning         | Explicit via 0.1pt threshold                     | Clear semantics                 |

### Detailed Design Decisions

#### Decision 1: Use bbox instead of center point

**Rationale:**

- Text element bounding box represents the actual rendered glyph extent
- Center point (x, y) is an approximation that doesn't account for glyph width/height
- Wide text like "Performance" can have bbox.x1 - bbox.x0 > 40pt, making center-based checks inaccurate

**Implementation:**

```rust
// OLD: Uses center
cx >= min_x - tol && cx <= max_x + tol

// NEW: Uses bbox extent
elem.bbox.x0 >= min_x - tol && elem.bbox.x1 <= max_x + tol
```

**Edge case handling:**

- Text partially overlapping cell boundary: Excluded (strict containment)
- Future enhancement: Calculate overlap percentage, assign to cell with >50% overlap

#### Decision 2: Reduce tolerance to 0.5pt

**Rationale:**

- PDF coordinate precision: ~0.01 inch at 72 DPI = 0.72pt
- 0.5pt tolerance covers rounding errors + minor font positioning variations
- 2.0pt tolerance allows text from adjacent cells in compact tables (15-20pt cell width)

**Analysis:**

```
Typical table cell width: 50-100pt
Adjacent cell gap: 5-10pt
Current 2pt tolerance: 4pt slop zone (40-80% of gap!)
New 0.5pt tolerance: 1pt slop zone (10-20% of gap) ✓
```

**Trade-offs:**

- Risk: May exclude text with poor PDF rendering alignment
- Mitigation: Test on real PDFs, adjust to 1.0pt if needed
- Benefit: Eliminates >95% of incorrect cell assignments

#### Decision 3: Remove Y-binning, use exact coordinates

**Rationale:**

- Row boundaries come from horizontal grid lines (unique_y)
- Y-binning (5pt) was a heuristic to group text into rows
- With precise grid coordinates, binning is unnecessary and harmful

**Implementation:**

```rust
// OLD: Bin Y-coordinates (merges elements within 5pt)
let row_a = (a.y / 5.0).round() as i32;
let row_b = (b.y / 5.0).round() as i32;

// NEW: Use exact Y with 0.1pt same-row threshold
if (a.bbox.y0 - b.bbox.y0).abs() < 0.1 {
    // Same row - sort by X
} else {
    // Different rows - sort by Y descending
}
```

**Why 0.1pt threshold:**

- Float comparison tolerance (prevent 100.0 != 100.00001 issues)
- Text on same baseline typically has identical Y within 0.1pt
- Much tighter than 5pt binning (50× improvement!)

#### Decision 4: Sort by Y descending, then X ascending

**Rationale:**

- PDF Y-axis increases upward (mathematical convention)
- Table rows read top-to-bottom in markdown
- Within row, text reads left-to-right

**Implementation:**

```rust
// Sort by Y descending (top row first)
b.bbox.y0.partial_cmp(&a.bbox.y0)

// Then by X ascending (left column first)
a.bbox.x0.partial_cmp(&b.bbox.x0)
```

**Correctness verification:**

- Top row: highest Y → sorted first ✓
- Bottom row: lowest Y → sorted last ✓
- Left cell: lowest X → sorted first within row ✓
- Right cell: highest X → sorted last within row ✓

### Handling Edge Cases

#### Edge Case 1: Text exactly on cell boundary

**Scenario:** Text element with bbox.x1 = cell boundary unique_x[j+1]

**Current behavior:** Included if within tolerance (±2pt)

**New behavior:**

- If bbox.x1 == unique_x[j+1] (exactly on boundary):
  - Strict containment: Check if bbox.x1 <= unique_x[j+1] + tol
  - With 0.5pt tolerance: Included if within 0.5pt
- Future: Could assign to left cell by default (tie-breaking rule)

**Decision:** Keep current tolerance-based approach, monitor edge case frequency

#### Edge Case 2: Text spanning multiple cells

**Scenario:** Wide text element where bbox.x1 - bbox.x0 > column width

**Current behavior:** Included if center point is in cell (WRONG - causes duplicates or misassignment)

**New behavior:** Strict containment test:

```rust
elem.bbox.x0 >= min_x - tol && elem.bbox.x1 <= max_x + tol
```

- Text spanning multiple cells: EXCLUDED from all cells (correct!)
- Reason: Cannot determine which cell "owns" the text without additional logic

**Future enhancement:** Overlap-based assignment:

```rust
let overlap_percentage = calculate_overlap(elem.bbox, cell_bbox);
if overlap_percentage > 0.5 {
    assign_to_this_cell();
}
```

**Decision for Loop 013:** Use strict containment (simpler, deterministic). Add overlap logic in future loop if needed.

#### Edge Case 3: Empty cells

**Scenario:** Grid cell with no text elements

**Current behavior:** Empty string returned ✓

**New behavior:** Same - empty string ✓

**Markdown rendering:** `| | |` (empty cell between pipes) ✓

#### Edge Case 4: Multi-line cell content

**Scenario:** Cell with text on multiple lines (e.g., wrapped paragraph in table)

**Current behavior:** Y-binning merges them (sometimes works by accident)

**New behavior:** Exact Y sorting preserves line order:

- Line 1 (Y=100): Sorted first
- Line 2 (Y=95): Sorted second
- Line 3 (Y=90): Sorted third
- Concatenated with spaces: "Line1 Line2 Line3"

**Decision:** Current implementation handles this correctly. No changes needed.

## Validation Plan

### Test Cases

#### Test 1: Basic cell extraction

```
Cell: x ∈ [100, 200], y ∈ [50, 70]
Text: bbox.x0=110, x1=150, y0=55, y1=65, text="Value"
Expected: "Value" ✓
```

#### Test 2: Text outside cell excluded

```
Cell: x ∈ [100, 200], y ∈ [50, 70]
Text: bbox.x0=205, x1=250, y0=55, y1=65, text="Adjacent"
Expected: "" (empty) ✓
```

#### Test 3: Text on boundary within tolerance

```
Cell: x ∈ [100, 200], y ∈ [50, 70]
Text: bbox.x0=99.7, x1=150, y0=55, y1=65, text="Edge"
Expected: "Edge" (99.7 >= 100-0.5) ✓
```

#### Test 4: Text on boundary outside tolerance

```
Cell: x ∈ [100, 200], y ∈ [50, 70]
Text: bbox.x0=98.5, x1=150, y0=55, y1=65, text="Outside"
Expected: "" (98.5 < 100-0.5) ✓
```

#### Test 5: Multiple text elements in cell

```
Cell: x ∈ [100, 200], y ∈ [50, 70]
Text1: bbox y0=65, x0=110, text="First"
Text2: bbox y0=65, x0=150, text="Second"
Expected: "First Second" (sorted by X) ✓
```

#### Test 6: Multi-line cell

```
Cell: x ∈ [100, 200], y ∈ [50, 70]
Text1: bbox y0=65, x0=110, text="Line1"
Text2: bbox y0=60, x0=110, text="Line2"
Text3: bbox y0=55, x0=110, text="Line3"
Expected: "Line1 Line2 Line3" (sorted by Y descending) ✓
```

### Metrics Targets

**Before (Loop 012 baseline):**

- Table Accuracy: 2.4%
- Composite Score: 32.5/100

**After (Loop 013 target):**

- Table Accuracy: 15-20% (6-8× improvement)
- Composite Score: 37-40/100 (+5-7 points)

**Success criteria:**

- ✅ At least 3/5 test documents show table accuracy improvement
- ✅ Best document (one_tool) improves from 11.4% to 20%+
- ✅ Zero-score documents (2900, agent, ccn) improve to >5%
- ✅ No regression in Style Accuracy or other metrics
- ✅ All 113 tests still passing

### Validation Steps

1. **Unit test:** Create unit test for extract_text_in_rect with test cases 1-6 above
2. **Integration test:** Run cargo test -p edgequake-pdf
3. **Regenerate outputs:** cargo run --bin real_dataset_eval --write
4. **Compare outputs:** diff old vs new generated markdown
5. **Run validator:** python validate.py and check metrics
6. **Manual inspection:** Review one_tool table output for correctness

## Implementation Checklist

- [ ] Back up current lattice.rs (git commit)
- [ ] Rewrite extract_text_in_rect() function (lines 505-542)
- [ ] Update tolerance: 2.0 → 0.5
- [ ] Change filter: elem.x/y → elem.bbox.x0/y0/x1/y1
- [ ] Remove Y-binning: (y/5.0).round() → bbox.y0 direct comparison
- [ ] Add same-row threshold: 0.1pt for float comparison
- [ ] Test compilation: cargo build -p edgequake-pdf
- [ ] Run unit tests: cargo test -p edgequake-pdf
- [ ] Regenerate outputs: cargo run --bin real_dataset_eval --write
- [ ] Run validator: python validate.py
- [ ] Check metrics: Table Accuracy >= 15%, Composite >= 37/100
- [ ] Manual inspection: Review generated tables vs gold
- [ ] Commit changes with detailed message

## Risk Assessment

### Low Risk

- Compilation errors: Easy to fix with Rust compiler help ✓
- Test failures: Can debug with cargo test output ✓
- No external dependencies: Pure logic change ✓

### Medium Risk

- Tolerance too tight: May exclude valid text
  - Mitigation: Start with 0.5pt, increase to 1.0pt if needed
- Edge cases: Text exactly on boundaries
  - Mitigation: Comprehensive test cases planned
- Performance: bbox comparisons vs center point
  - Analysis: Negligible (4 float compares vs 2 float compares)

### High Risk (None Identified)

- All changes are local to extract_text_in_rect()
- Grid detection logic untouched (already working)
- Column clustering untouched (already working)
- Fallback logic untouched (already working)

## Alternative Approaches Considered

### Alternative 1: Overlap-based assignment

**Description:** Calculate bbox overlap percentage, assign to cell with largest overlap

**Pros:**

- Handles text spanning multiple cells gracefully
- More robust to PDF rendering variations

**Cons:**

- More complex algorithm (10-15 lines vs 3 lines)
- Need to define overlap calculation (area? linear?)
- May assign text to "wrong" cell if overlap is ambiguous

**Decision:** Defer to future loop. Start with strict containment (simpler, deterministic).

### Alternative 2: Expand cell boundaries by tolerance

**Description:** Instead of shrinking text bbox, expand cell bbox by tolerance

**Pros:**

- Mathematically equivalent to current approach
- May be more intuitive

**Cons:**

- Need to ensure expanded cells don't overlap
- More complex to reason about edge cases

**Decision:** Keep current approach (shrink text bbox check). Same result, clearer semantics.

### Alternative 3: Keep Y-binning but reduce bin size

**Description:** Use 1pt bins instead of 5pt bins

**Pros:**

- Maintains heuristic approach with less aggressive merging
- Safer incremental change

**Cons:**

- Still introduces artifacts (1pt is arbitrary)
- With precise grid coordinates, binning is unnecessary
- Doesn't fix root cause

**Decision:** Eliminate Y-binning entirely. Use exact coordinates with 0.1pt float tolerance.

## Success Metrics Summary

| Metric           | Before    | Target   | Stretch  |
| ---------------- | --------- | -------- | -------- |
| Table Accuracy   | 2.4%      | 15-20%   | 25%+     |
| Composite Score  | 32.5      | 37-40    | 42+      |
| Tests Passing    | 113/113   | 113/113  | 113/113  |
| one_tool Table % | 11.4%     | 20%+     | 30%+     |
| Zero-score docs  | 3/5 (60%) | 0/5 (0%) | 0/5 (0%) |

**Decision:** Proceed with implementation. The approach is sound, risk is low, and expected improvement is significant (6-8× table accuracy, +15-22% composite score).

**Next Phase:** ACT - Implement the changes, test, and validate.
