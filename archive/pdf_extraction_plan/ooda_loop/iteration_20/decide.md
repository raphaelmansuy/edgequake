# OODA-20 Decide: Three-Point Fix for Block Merging

## Decision

Implement three coordinated fixes:

### Fix 1: Proper BBox Width Calculation
**File**: `block_builder.rs`
**Function**: `calculate_line_bbox()`

```rust
// Estimate text width from character count and font size
let estimated_char_width = 0.55 * font_size; // empirical average
let text_width = char_count as f32 * estimated_char_width;
max_x = max(e.x + text_width)
```

**Rationale**: Text has non-zero width. Estimating based on character count and font size is simple and effective.

### Fix 2: Minimum Column Width Filter
**File**: `geometric.rs`
**Function**: `detect_columns()`

```rust
const MIN_COLUMN_WIDTH: f32 = 80.0; // ~1 inch minimum

// Merge narrow columns with adjacent
if column.width() < MIN_COLUMN_WIDTH {
    merge_with_neighbor(column);
}
```

**Rationale**: 80pt is narrower than any reasonable text column but wider than indentation gaps.

### Fix 3: Left-Edge Column Assignment
**File**: `layout_processing.rs`
**Function**: `get_block_column()`

```rust
// Use left edge instead of center
let left_edge = Point { x: block.bbox.x1, y: block.bbox.y1 };
columns.iter().position(|col| col.contains_point(left_edge))
```

**Rationale**: Reading starts at left edge. Text continuation should be based on where lines start, not midpoint.

## Alternatives Considered

### Alternative A: Fuzzy Column Matching
Use overlapping column boundaries with tolerance.
**Rejected**: Over-complicated, hard to tune.

### Alternative B: Text-Based Paragraph Detection
Look for sentence endings to detect paragraph breaks.
**Rejected**: Violates "geometric first" principle.

### Alternative C: Ignore Columns for Merge
Just merge vertically adjacent blocks.
**Rejected**: Would merge across multi-column layouts.

## Risk Assessment

| Fix | Risk | Mitigation |
|-----|------|------------|
| BBox width | Could over-estimate width | Use conservative 0.55 factor |
| Min column | Could merge real narrow columns | 80pt is safe threshold |
| Left-edge | Edge case with right-aligned text | Very rare in documents |

## Test Impact

- `test_two_column_detection`: Must update expected columns from 3 → 2
- All 415 existing tests should pass
