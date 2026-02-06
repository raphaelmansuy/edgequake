# IT10 Decide: Enhance TextTableReconstructionProcessor

## Decision

Enhance `TextTableReconstructionProcessor` to detect tables **without captions** using text alignment patterns.

## Rationale

```
┌─────────────────────────────────────────────────────────────┐
│                    DECISION MATRIX                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Option A: Wire is_likely_table() (layout/column_detector)  │
│  - Risk: HIGH - Different module, BoundingBox vs TextElement│
│  - Impact: MEDIUM - Still needs reconstruction logic        │
│  - Effort: MEDIUM                                           │
│                                                             │
│  Option B: Enable TableDetectionProcessor for multi-column  │
│  - Risk: HIGH - OODA-34 disabled it for good reason         │
│  - Impact: HIGH - Could break reading order again           │
│  - Effort: HIGH - Needs careful integration                 │
│                                                             │
│  Option C: Enhance TextTableReconstructionProcessor ← CHOSEN│
│  - Risk: LOW - Already in pipeline, well-tested             │
│  - Impact: MEDIUM - Tables without captions will be found   │
│  - Effort: LOW - Add new detection mode                     │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Implementation Plan

### 1. Add Column Alignment Detection

Add a new method to detect vertically aligned text blocks:

```rust
/// Detect blocks that are vertically aligned (potential table columns)
fn detect_column_aligned_blocks(&self, page: &Page) -> Vec<Vec<usize>> {
    // Group blocks by X-coordinate (column alignment)
    // Blocks with similar X-start are in the same column
}
```

### 2. Add Row Detection

Add a method to detect horizontally aligned blocks (table rows):

```rust
/// Detect blocks that are horizontally aligned (same Y-coord)
fn detect_row_aligned_blocks(&self, page: &Page) -> Vec<Vec<usize>> {
    // Group blocks by Y-coordinate
    // Blocks with similar Y are in the same row
}
```

### 3. Add Grid Pattern Detection

Detect if blocks form a grid pattern (rows × columns):

```rust
/// Check if blocks form a grid pattern (table structure)
fn is_grid_pattern(columns: &[Vec<usize>], rows: &[Vec<usize>]) -> bool {
    // Grid pattern:
    // - Multiple columns (≥2)
    // - Multiple rows (≥2)
    // - Blocks appear at intersections
}
```

### 4. Process Method Enhancement

Modify `process_page` to also scan for grid patterns, not just captions:

```rust
// Existing: Look for "Table N" captions
// NEW: Also scan for grid patterns in blocks
let grid_tables = self.detect_grid_tables(page);
```

## Expected Outcome

Tables like this (from academic papers) will be detected:

```
Dataset      Docs    Tokens
Agriculture  12      2,017,886
CS           10      2,306,535
Legal        20      5,081,069
Mix          61      619,009
```

Even without a "Table 4:" caption nearby.

## Test Plan

1. Add unit test for column alignment detection
2. Add unit test for row alignment detection
3. Add unit test for grid pattern detection
4. Integration test with LightRAG paper (page 13, Table 4)
