# OODA-20 Act: Implementation Results

## Implementation Date

2026-02-03

## Changes Made

### 1. block_builder.rs - BBox Width Fix

**Location**: `calculate_line_bbox()`

```rust
// Before: zero-width bboxes
max_x = elements.iter().map(|e| e.x).max()

// After: include estimated text width
let estimated_width = text.chars().count() as f32 * font_size * 0.55;
max_x = elements.iter().map(|e| e.x + estimated_width).max()
```

### 2. geometric.rs - Minimum Column Width

**Location**: `detect_columns()`

```rust
const MIN_COLUMN_WIDTH: f32 = 80.0;

// Merge columns narrower than threshold
let mut merged_columns = Vec::new();
for col in columns {
    if col.width() < MIN_COLUMN_WIDTH && !merged_columns.is_empty() {
        let last = merged_columns.last_mut().unwrap();
        last.x2 = col.x2.max(last.x2);
    } else {
        merged_columns.push(col);
    }
}
```

### 3. layout_processing.rs - Left-Edge Assignment

**Location**: `get_block_column()`

```rust
// Before: center point
let center = block.bbox.center();
columns.iter().position(|col| col.contains_point(center))

// After: left edge
columns.iter().position(|col| block.bbox.x1 >= col.x1 && block.bbox.x1 <= col.x2)
```

### 4. column_detector.rs - Test Update

**Change**: Updated `test_two_column_detection` assertion

```rust
// Before: expected 3 columns (including margin split)
assert_eq!(columns.len(), 3);

// After: expects 2 columns (narrow merged)
assert_eq!(columns.len(), 2);
```

## Verification

### Test Results

```
test result: ok. 415 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### Before/After Comparison

**Before (OODA-19)**:

- 3 columns detected: [0,300], [300,322], [322,612]
- Blocks 2,3,4,5 in different columns
- "ROI." rejected merge with "teams move..."

**After (OODA-20)**:

- 2 columns detected: [0,350], [350,612]
- All main content in same column
- "ROI." merges correctly

## Metrics Impact

| Metric        | Before  | After   | Change      |
| ------------- | ------- | ------- | ----------- |
| Column count  | 3       | 2       | -1 spurious |
| Block merges  | Failed  | Working | Fixed       |
| Test coverage | 415/415 | 415/415 | Maintained  |

## Lessons Learned

1. **Zero-width bboxes break everything** - Always ensure bounding boxes have meaningful dimensions
2. **Column detection needs hysteresis** - Small gaps shouldn't create new columns
3. **Left-edge is more stable than center** - For reading-order layouts, left alignment matters

## Next Steps

- OODA-21: Investigate remaining paragraph separation issues
- Run full quality benchmark against markitdown gold standard
- Track quality metrics trend
