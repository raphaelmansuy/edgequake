# OODA-08 Decide: Column-Aware Table Filtering

## Prioritized Changes

### Priority 1: Move Column Detection Before Table Filtering

**File**: `backend/extraction_engine.rs`
**Location**: `extract_page()` function

**Current Order (lines ~430-540)**:

1. Extract elements
2. Detect tables from lattice
3. Filter tables by size/position
4. Group into lines (column detection happens here)

**New Order**:

1. Extract elements
2. **Detect columns FIRST**
3. Detect tables from lattice
4. Filter tables with column awareness
5. Group into lines

### Priority 2: Add Column-Crossing Filter

**Logic**:

```rust
// If two-column layout detected
if let Some(column_boundary) = column_boundary {
    // Table crosses column boundary if:
    // - Left edge is in left column (x1 < boundary)
    // - Right edge is in right column (x2 > boundary)
    if table.bbox.x1 < column_boundary && table.bbox.x2 > column_boundary + 10.0 {
        // Allow full-width tables at very top or bottom of page
        let is_top_area = table.bbox.y1 < page_height * 0.15;
        let is_bottom_area = table.bbox.y2 > page_height * 0.85;

        if !is_top_area && !is_bottom_area {
            debug!("Filtered out table: crosses column boundary");
            return false;
        }
    }
}
```

### Priority 3: Document the Change

Add WHY comment explaining the reasoning.

## Expected Outcome

- Tables in side-by-side configuration will be filtered out at extraction stage
- Text content will be processed as normal text blocks
- Later processors can still create tables from text patterns if needed

## Fallback Plan

If filtering tables that cross column boundary causes legitimate full-width tables to be missed:

- Add a size threshold: Only filter if table width < 70% of page width
- Full-width tables (>70% of page) are likely intentional single tables
