# OODA-18 Observe: Multi-Column Reading Order Analysis

## Current State

- Overall Quality: 86.5%
- Target: 95%+
- Gap: 8.5 percentage points

## Key Observation

The `one_tool_2512.20957v2` document has 80.5% text quality due to scrambled reading order in multi-column layouts.

## Root Cause Analysis

### Pipeline Flow

1. **ExtractionEngine** → Extracts blocks from PDF
2. **text_grouping.rs** → Groups text elements, correctly separates left/right columns
3. **LayoutProcessor** → Sorts blocks by reading order (left column first, then right)
4. **TableDetectionProcessor** → Groups blocks by Y-coordinate for table detection
5. **BlockMergeProcessor** → Merges related blocks

### Bug Location

`TableDetectionProcessor.group_blocks_by_row()` sorts blocks by Y-coordinate:

```rust
sorted_indices.sort_by(|&a, &b| {
    page.blocks[a].bbox.y1.partial_cmp(&page.blocks[b].bbox.y1).unwrap()
});
```

This Y-sorting causes blocks from left and right columns to be interleaved because they have similar Y coordinates.

### Evidence

- `OODA-18-AFTER-SORT` logs show correct order after LayoutProcessor (all left column x≈75)
- `OODA-18-BEFORE-MERGE` logs show scrambled order (left/right interleaved)

## Data Points

| Document    | Text Score | Issue               |
| ----------- | ---------- | ------------------- |
| one_tool    | 80.5%      | Column interleaving |
| agent       | 80.1%      | Similar issue       |
| ccn         | 81.1%      | Similar issue       |
| v2          | 92.7%      | Minimal issue       |
| AlphaEvolve | 85.8%      | Has real tables     |
