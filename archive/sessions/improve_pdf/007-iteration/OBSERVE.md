# OBSERVE.md - Iteration 007

**Directory:** `edgequake/crates/edgequake-pdf/src/processors`

**Timestamp:** 2026-01-02

## Target: Eliminate Magic Number Thresholds

### Current State (After Loop 006)

- ✅ 111 tests passing
- Lattice engine enabled
- SECTION_KEYWORDS eliminated
- Composite Score: ~32-35/100 (estimated)

### Magic Numbers Identified

#### 1. BlockMergeProcessor (CRITICAL)

**Location:** `processor.rs:1415-1416`

```rust
pub fn new() -> Self {
    Self {
        max_vertical_gap: 50.0, // Increased to handle typical line spacing (12pt font = ~12pt gap)
        max_margin_diff: 20.0,
    }
}
```

**Location:** `processor.rs:1530`

```rust
let max_gap = if a.block_type == BlockType::SectionHeader {
    35.0 // Allow more gap for multi-line headers
} else {
    self.max_vertical_gap
};
```

**Location:** `processor.rs:1537`

```rust
let horizontal_zone_threshold = 100.0; // If X positions differ by > 100pt, different columns
```

**Issues:**

- `50.0` - Assumes all documents have same line spacing
- `20.0` - Assumes all documents have same margins
- `35.0` - Hardcoded exception for headers
- `100.0` - Assumes all documents have same column width

**Impact:**

- False positives: Merges blocks from different columns
- False negatives: Doesn't merge multi-line blocks with larger spacing
- Doesn't adapt to document variation

#### 2. MarginFilterProcessor

**Location:** `processor.rs:186-189`

```rust
Self {
    left_margin: 50.0,   // Filter content in first 50pt (line numbers)
    right_margin: 30.0,  // Filter right margin content
    top_margin: 40.0,    // Filter header area
    bottom_margin: 40.0, // Filter footer area
}
```

**Issues:**

- Assumes standard page margins across all PDFs
- Doesn't account for actual page layout
- May filter valid content or miss actual margins

#### 3. HyphenContinuationProcessor

**Location:** `processor.rs:2706`

```rust
if vertical_gap <= 50.0 {
    // Hyphen continuation
}
```

**Issue:** Same 50.0 magic number repeated

#### 4. ListDetectionProcessor

**Location:** `processor.rs:2394`

```rust
let level = (indent / 20.0).round() as i32;
```

**Issue:** Assumes 20pt per indentation level

### Why These are Wrong (First Principles Violations)

1. **Document Variation:**

   - Academic papers: 10-12pt body, ~15-18pt line spacing
   - Technical manuals: 9pt body, ~12-15pt line spacing
   - Presentations: 14-24pt body, ~20-36pt line spacing
   - Different page sizes: Letter, A4, Legal, Custom

2. **No Statistical Basis:**

   - Values aren't derived from document properties
   - No adaptation to actual layout
   - One-size-fits-all approach fails

3. **Arbitrary Choices:**
   - "50.0" appears 5+ times (coincidence? cargo cult?)
   - "20.0" for margins (why not 15 or 25?)
   - "100.0" for columns (what about narrow columns?)

### What First Principles Says

**Spatial relationships should be derived from:**

1. **Font Metrics:**
   - Line spacing = f(body_font_size, leading)
   - Typical: 1.2-1.5x font size
2. **Page Statistics:**
   - Column detection → derive column gap
   - Block distribution → derive typical spacing
3. **Clustering:**
   - Use DBSCAN (like Loop 004 did for columns!)
   - Adaptive epsilon from coordinate distribution
4. **Percentile-Based:**
   - Use 10th/90th percentiles, not hardcoded values
   - Adapts to each document

### Examples of Correct Approach

**Loop 004 - Column Detection (GOOD):**

```rust
// Calculate adaptive epsilon from X-coordinate distribution
let sorted_x: Vec<f32> = x_coords.iter().copied().sorted().collect();
let epsilon = calculate_percentile(&sorted_x, 0.1); // 10th percentile
```

**BlockMergeProcessor (BAD):**

```rust
max_vertical_gap: 50.0  // Fixed value, no adaptation
```

### Solution Strategy

#### Option 1: Statistical Derivation (Recommended)

```rust
pub fn new(document: &Document) -> Self {
    let stats = DocumentStats::from(document);
    Self {
        max_vertical_gap: stats.median_line_spacing * 2.5,
        max_margin_diff: stats.column_alignment_tolerance,
    }
}
```

#### Option 2: DBSCAN Clustering (Most Principled)

```rust
fn should_merge(&self, blocks: &[Block]) -> Vec<(usize, usize)> {
    // Cluster blocks by Y-coordinate
    let y_coords: Vec<f32> = blocks.iter().map(|b| b.bbox.y1).collect();
    let clusters = dbscan(&y_coords, epsilon_from_distribution(&y_coords), 2);
    // Merge within clusters
}
```

#### Option 3: Font-Based (Simple)

```rust
fn should_merge(&self, a: &Block, b: &Block, body_size: f32) -> bool {
    let typical_line_spacing = body_size * 1.5;
    let vertical_gap = (a.bbox.y1 - b.bbox.y2).abs();
    vertical_gap <= typical_line_spacing
}
```

### Impact Analysis

**Estimated Improvement:**

- **Style Accuracy:** +5-10 points (better block boundaries)
- **Table Accuracy:** +2-5 points (less column bleeding)
- **Robustness:** +5-10 points (works on varied documents)

**Risk:**

- May break some edge cases initially
- Need to tune percentile thresholds
- Must maintain backward compatibility with tests

### Files to Analyze

1. `processor.rs` - BlockMergeProcessor (primary target)
2. `processor.rs` - MarginFilterProcessor (secondary)
3. Test files - Understand expected behavior
4. Real PDFs - Measure actual spacing distributions

### Metrics to Collect

Before implementing, need to measure on real documents:

1. Line spacing distribution (median, P10, P90)
2. Margin alignment distribution
3. Column gap statistics
4. Font size distribution

This data will inform the statistical approach.
