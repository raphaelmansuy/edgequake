# OODA-16: Act Phase

## Implementation Summary

**Problem**: Table detection was completely skipping multi-column pages, preventing detection of tables within 2-column academic papers like AlphaEvolve.

**Solution**: Enable table detection for multi-column pages with stricter criteria to avoid false positives from column text.

## Code Changes

### File: `src/processors/table_detection.rs`

**Change 1**: Removed unconditional skip, added strict_mode flag (lines 62-77)

```rust
// OODA-16: Enable table detection for multi-column pages with stricter criteria
let strict_mode = page.columns.len() > 1;
if strict_mode {
    tracing::info!(
        "  Multi-column page ({} columns) - using strict table detection",
        page.columns.len()
    );
}
```

**Change 2**: Added strict_mode parameter to `group_blocks_by_row` (line 96)

```rust
fn group_blocks_by_row(&self, page: &crate::schema::Page, strict_mode: bool) -> Vec<Vec<usize>>
```

**Change 3**: Tighter Y-tolerance in strict mode (lines 118-121)

```rust
// OODA-16: Stricter Y-tolerance in multi-column mode
// - Normal mode: 10pt tolerance for slight extraction misalignment
// - Strict mode: 2pt tolerance to require precise table alignment
let y_tolerance = if strict_mode { 2.0 } else { 10.0 };
```

**Change 4**: Text length filter in `is_likely_table` (lines 275-310)

```rust
// OODA-16: In strict mode, add text length filter
// Tables have short cells (<100 chars), paragraphs have long sentences (100-300 chars)
if strict_mode {
    let avg_text_length = total_chars as f32 / total_blocks as f32;
    if avg_text_length > 100.0 {
        return false; // Likely column text, not table
    }
}
```

## Quality Metrics

**Before (OODA-15):**

- Text: 85.0%
- Structure: 81.2%
- Overall: 83.1%

**After (OODA-16):**

- Text: 85.7% (+0.7%)
- Structure: 87.2% (+6.0%)
- Overall: 86.5% (+3.4%)

## Per-Document Changes

| Document              | Before | After | Change    |
| --------------------- | ------ | ----- | --------- |
| ccn_2512.21804v1      | 83.3%  | 86.6% | +3.3%     |
| 2900_Goyal_et_al      | 85.7%  | 85.7% | 0.0%      |
| v2_2512.25072v1       | 85.6%  | 89.5% | +3.9%     |
| AlphaEvolve           | 81.2%  | 92.9% | +11.7% 🎉 |
| agent_2510.09244v1    | 80.1%  | 80.1% | 0.0%      |
| 01_2512.25075v1       | 85.6%  | 87.7% | +2.1%     |
| one_tool_2512.20957v2 | 80.2%  | 82.8% | +2.6%     |

## Key Insights

1. **The skip was too aggressive**: Multi-column pages CAN contain tables that should be detected.

2. **Strict criteria prevent false positives**: The combination of:
   - Tighter Y-tolerance (2pt vs 10pt)
   - Text length filter (<100 chars average)
   - Existing 3+ row requirement

   Successfully distinguishes real tables from column text alignment.

3. **AlphaEvolve Table 1 now detected**: The FunSearch vs AlphaEvolve comparison table on page 2 is now properly formatted as a markdown table, improving structure score from 76.2% to 100%.

4. **Ripple effect on other documents**: Several other multi-column papers also benefited (ccn, v2, 01, one_tool) with structure improvements.

## Cumulative Progress

| Iteration | Overall Quality | Change    |
| --------- | --------------- | --------- |
| OODA-14   | 83.0%           | baseline  |
| OODA-15   | 83.1%           | +0.1%     |
| OODA-16   | 86.5%           | +3.4%     |
| **Total** | -               | **+3.5%** |

## Gap to Target

- Current: 86.5%
- Target: 95%
- Remaining gap: 8.5 percentage points

## Commit

```
OODA-16: Enable table detection in multi-column layouts

- Remove unconditional skip for multi-column pages
- Add strict_mode with tighter Y-tolerance (2pt vs 10pt)
- Add text length filter (<100 chars avg) to distinguish tables from columns
- AlphaEvolve: 81.2% → 92.9% (+11.7% - Table 1 now detected!)
- Overall: 83.1% → 86.5% (+3.4%)
```
