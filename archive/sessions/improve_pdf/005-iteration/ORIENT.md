# ORIENT.md - Iteration 005

**Directory:** `edgequake/crates/edgequake-pdf/src/backend`

**Timestamp:** 2026-01-02

## Root Cause Analysis

### Why is Lattice Engine Disabled?

#### Hypothesis 1: False Positives (Page Borders)

**Evidence:**

- Lattice engine detects connected components of intersecting lines
- Page borders form a rectangle (4 intersecting lines)
- Minimum component size is 4 lines (a box)

**Likelihood:** HIGH

**Impact:** Page borders would be detected as tables, filtering out all text.

#### Hypothesis 2: Performance Issues

**Evidence:**

- Line intersection check is O(n²) for n lines
- Complex PDFs could have hundreds of lines
- No early termination or optimization

**Likelihood:** MEDIUM

**Impact:** Slow extraction on complex documents.

#### Hypothesis 3: Incomplete Testing

**Evidence:**

- Comment says "DISABLED FOR NOW" with no explanation
- No unit tests for lattice_engine integration
- No integration tests with real PDFs

**Likelihood:** HIGH

**Impact:** Unknown behavior on real documents.

## First Principles Analysis

### What is Table Detection?

**Definition:** Identifying regions of a document that present data in a tabular format.

**First Principles:**

1. Tables have structure (rows and columns)
2. Tables contain data (text/numbers)
3. Tables are visually distinct from surrounding text

### Lattice-Based Table Detection

**Core Idea:** Use graphical lines (borders, separators) to identify table structure.

**Algorithm:**

1. Extract horizontal and vertical lines from PDF
2. Find intersecting lines (connected components)
3. Each connected component is a potential table
4. Extract text from grid cells
5. Format as Markdown table

**Advantages:**

- Uses actual PDF structure (not heuristics)
- Works for any table with visible borders
- No magic numbers (except reasonable defaults)
- Composable and testable

**Disadvantages:**

- Requires visible lines (doesn't work for borderless tables)
- O(n²) complexity for line intersection
- Can detect page borders as tables

### Alternative Approaches

#### 1. Whitespace-Based Detection

**Idea:** Use gaps in text to infer table structure.

**Pros:**

- Works for borderless tables
- Faster than line-based

**Cons:**

- Heuristic-based (magic numbers for gap thresholds)
- Can fail on dense text
- Not first principles

#### 2. Hybrid Approach

**Idea:** Combine line-based and whitespace-based detection.

**Pros:**

- Works for both bordered and borderless tables
- More robust

**Cons:**

- More complex
- Still has heuristics

## Code Analysis

### Lattice Engine Implementation

**Strengths:**

1. **Pure Functions:** `detect_tables()` has clear inputs/outputs
2. **Modular:** Separate methods for filtering, intersection, cell extraction
3. **Fallback:** Handles tables without vertical lines
4. **Markdown Output:** Formats tables correctly

**Weaknesses:**

1. **No Size Filtering:** Accepts any connected component with >= 4 lines
2. **No Content Validation:** Doesn't check if table contains text
3. **No Position Filtering:** Doesn't exclude page margins/borders
4. **O(n²) Complexity:** Line intersection is quadratic

### Integration Point

**Current Code (Disabled):**

```rust
let tables = self.lattice_engine.detect_tables(
    page_num,
    &lines,           // pdf_lines from extract_page_elements
    &mut text_elements, // elements from extract_page_elements
    page_width,
    page_height,
);
```

**Issues:**

1. `lines` variable doesn't exist (should be `pdf_lines`)
2. `text_elements` is mutable but not modified
3. No safeguards against false positives

## Proposed Solutions

### Solution 1: Minimal Enable with Size Filters (RECOMMENDED)

**Changes:**

1. Fix variable name (`lines` → `pdf_lines`)
2. Add minimum table size check (exclude small boxes)
3. Add maximum table size check (exclude page borders)
4. Add content validation (exclude empty tables)

**Code:**

```rust
let tables = self.lattice_engine.detect_tables(
    &pdf_lines,
    &elements,
    page_width,
    page_height,
).into_iter()
.filter(|table| {
    // Exclude tables that are too small (< 50x50 points)
    let min_size = 50.0;
    if table.bbox.width() < min_size || table.bbox.height() < min_size {
        return false;
    }

    // Exclude tables that are too large (> 90% of page)
    let max_width = page_width * 0.9;
    let max_height = page_height * 0.9;
    if table.bbox.width() > max_width || table.bbox.height() > max_height {
        return false;
    }

    // Exclude empty tables
    if table.text.trim().is_empty() {
        return false;
    }

    true
}).collect();
```

**Advantages:**

- Minimal code change
- First principles (uses actual size/content)
- No magic numbers (percentages are reasonable)
- Addresses false positives

**Expected Impact:**

- Table Accuracy: 2.4% → 10-20%
- Composite Score: 32.5 → 36-40/100

### Solution 2: Add Performance Optimization

**Changes:**

1. Implement Solution 1
2. Add spatial indexing for line intersection (R-tree or grid)
3. Early termination for large line counts

**Advantages:**

- Better performance on complex PDFs
- Scales to larger documents

**Disadvantages:**

- More complex
- Not necessary for initial enablement

### Solution 3: Add Unit Tests

**Changes:**

1. Implement Solution 1
2. Add unit tests for lattice_engine integration
3. Add integration tests with real PDFs

**Advantages:**

- Prevents regressions
- Documents expected behavior

**Disadvantages:**

- Takes more time
- Can be done in separate iteration

## Decision

**Selected Solution:** Solution 1 (Minimal Enable with Size Filters)

**Rationale:**

1. **First Principles:** Uses actual table size/content (not heuristics)
2. **Minimal Change:** Only 10-15 lines of code
3. **High Impact:** Expected 8-15 point improvement in Table Accuracy
4. **Low Risk:** Filters are conservative and reasonable
5. **Testable:** Can verify with real_dataset_eval

**Acceptance Criteria:**

- [ ] Enable lattice_engine with size/content filters
- [ ] All 111 tests still pass
- [ ] Table Accuracy improves (2.4% → target 10%+)
- [ ] No performance regression (Performance >= 90%)
- [ ] No crashes on real dataset
- [ ] Update OBSERVE.md with actual results

## References

- Lattice-based table detection: Academic approach using graphical lines
- First principles: Use actual PDF structure, not heuristics
- Code smell: "DISABLED FOR NOW" without explanation
