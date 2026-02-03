# OODA-24 Decide: Page Number Filtering

## Decision

Focus on **Page Number Filtering** as it's low effort and high impact:

1. Page numbers ("1", "2", "3") appear as standalone blocks in output
2. Easy to detect: numeric-only blocks, typically at page edges
3. Should be filtered out during block processing

## Implementation Plan

### Option A: Header/Footer Detection (Preferred)
- Detect blocks at page top/bottom margin
- Filter out numeric-only content in these regions
- WHY: Generic approach, works for any document

### Option B: Block Type Classification
- Add `BlockType::PageNumber` 
- Skip during markdown rendering
- WHY: More explicit but adds complexity

### Chosen: Option A

Filter numeric-only blocks at page edges as running headers/footers.

### Location
- `src/backend/block_builder.rs` or `src/processors/layout_processing.rs`
- Look for `is_running_header` or similar logic

### Changes Required
1. Find existing running header/footer detection
2. Add page number detection (numeric-only, at page edge)
3. Filter from output

### Expected Impact
- Remove noise lines like "2", "1" from output
- Improve structural fidelity by ~1-2%
- No risk of filtering real content (page numbers are always standalone numbers)

### Test Validation
- one_tool_2512.20957v2.pdf should not have "2" appearing on line 6, 8
- All existing tests should pass
