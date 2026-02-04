# OODA Loop Iteration 37 - Figure Caption Preservation

## Date

2025-02-04

## Observe

- Figure 4 ("Cam×Time dataset visualization") and Figure 7 ("Temporal compression ablation") were missing from 01_2512 output
- These captions exist in the gold file but were not appearing in generated markdown
- F1 score for 01_2512 was 0.853 (before fix)

## Orient

Investigation traced the issue through the entire pipeline:

1. **text_grouping.rs**: Figure 4 and Figure 7 ARE detected and properly assigned to columns ✓
2. **block_builder.rs**: Figures detected with correct block types ✓
3. **extraction_engine.rs**: Figures present in blocks after block building ✓
4. **ProcessorChain**: Figures present BEFORE processors, MISSING AFTER processors ⚠️
5. **TextTableReconstructionProcessor**: ROOT CAUSE FOUND

### Root Cause Analysis

In `table_detection.rs`, the `scan_for_table()` function scans forward from a Table caption to collect table content. The scan STOPPED when it hit:

- Empty text
- Hard breaks (section headers)
- Another Table caption (`Table N.` pattern)

**The bug**: Figure captions (`Figure N.`) did NOT trigger the break condition, so they were consumed as part of the preceding table's content and discarded.

Evidence from debug tracing:

```
TTRP-SKIP-FIG: page=4 caption_i=60 consumed=65 skipped_i=63 text='Figure 4...'
TTRP-SKIP-FIG: page=7 caption_i=60 consumed=65 skipped_i=42 text='Figure 7...'
```

## Decide

Add `is_figure_caption` detection to the stopping condition in `scan_for_table()` so Figure captions are preserved instead of being consumed as table content.

## Act

### Fix Implementation in `table_detection.rs` (line ~930):

```rust
// OODA-37 FIX: Stop scanning when hitting Figure captions
let is_figure_caption = t.starts_with("Figure ")
    && t.len() > 7
    && t.chars().nth(7).map_or(false, |c| c.is_ascii_digit());

if t.is_empty() || Self::is_hard_break(b) || Self::looks_like_table_caption(t) || is_figure_caption {
    break;
}
```

### Cleanup

Removed all OODA-37 debug statements (eprintln!) from:

- text_grouping.rs
- block_builder.rs
- extraction_engine.rs
- processor.rs
- markdown.rs
- table_detection.rs
- layout_processing.rs

Also cleaned up older debug statements that were polluting output:

- FIGURE->LEFT/RIGHT/FOOTER tracking
- PAGE1-BLOCKS debug
- OODA-12 Y-sort debug
- BMP-PAGE debug
- LAYOUT-SKIP debug

## Results

### Before Fix

- 01_2512: F1 = 0.853
- Figure 4 and Figure 7 missing from output

### After Fix

- 01_2512: F1 = 0.854 (+0.001)
- Figure 4 and Figure 7 now appear in output
- All 7 documents maintain their F1 scores
- Average F1: 91.1%

### Quality Metrics (Post-Fix)

| Document              | F1 Score  | Precision | Recall |
| --------------------- | --------- | --------- | ------ |
| 01_2512.25075v1       | 0.854     | 0.955     | 0.772  |
| 2900_Goyal_et_al      | 0.943     | 0.921     | 0.967  |
| AlphaEvolve           | 1.000     | 1.000     | 1.000  |
| agent_2510.09244v1    | 0.957     | 0.987     | 0.928  |
| ccn_2512.21804v1      | 0.931     | 0.986     | 0.883  |
| one_tool_2512.20957v2 | 0.753     | 0.670     | 0.861  |
| v2_2512.25072v1       | 0.939     | 0.969     | 0.911  |
| **Average**           | **0.911** |           |        |

## Files Modified

1. `src/processors/table_detection.rs` - Added figure caption detection (FIX)
2. `src/backend/text_grouping.rs` - Removed debug statements
3. `src/backend/block_builder.rs` - Removed debug statements
4. `src/backend/extraction_engine.rs` - Removed debug statements
5. `src/processors/processor.rs` - Removed debug statements
6. `src/renderers/markdown.rs` - Removed debug statements
7. `src/processors/layout_processing.rs` - Removed debug statements

## Lessons Learned

1. **Table reconstruction is greedy**: The scan_for_table() function consumes all blocks until a stopping condition. This can accidentally consume unrelated content.
2. **Caption patterns need explicit protection**: Both Table and Figure captions have similar patterns (`Type N.`). When adding stopping conditions for one type, consider others.
3. **Debug tracing through the pipeline is essential**: The issue was only visible by tracing block flow from text_grouping through processors to the final renderer.

## Next Steps

- Investigate why one_tool_2512 has the lowest F1 (0.753)
- Continue with OODA-38
