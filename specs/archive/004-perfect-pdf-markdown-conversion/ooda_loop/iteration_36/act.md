# OODA Iteration 36 - Act Phase

## Date: 2026-02-04

## Actions Taken

### 1. Added Figure/Table Caption Protection (OODA-36)

Modified [text_grouping.rs](edgequake/crates/edgequake-pdf/src/backend/text_grouping.rs) to protect figure/table captions from being classified as footer/header/affiliation.

**Changes:**

- Added `is_figure_or_table_caption` detection using pattern "Figure N." or "Table N."
- Modified `is_footer` to exclude figure/table captions
- Modified `is_header` to exclude figure/table captions
- Modified `is_affiliation` to exclude figure/table captions

### 2. Verified Changes Compile

- Build: ✅ Success
- Smoke tests: ✅ 4/4 passed (0.01s)

### 3. Investigated Figure 4/7 Missing Issue

**Finding:** Debug logs show Figure 4 caption IS being detected:

```
FIGURE->GAP-RIGHT: X=317.2 boundary=305.0 center=306.0 text='Figure 4. Cam×Time dataset visualization. (Top) A space-tim'
```

The caption is:

1. ✅ Being extracted from PDF
2. ✅ Being assigned to right column (GAP-RIGHT)
3. ✅ NOT being classified as footer (our protection works)
4. ❌ NOT appearing in final output file

### 4. Root Cause Identification

The issue is NOT in text_grouping.rs. The Figure 4 caption is correctly assigned to the right column.

**Hypothesis:** The issue is downstream in:

1. Block building (elements → blocks conversion)
2. Block rendering (blocks → markdown conversion)
3. Final output writing

### 5. Quality Metrics (Unchanged)

| PDF      | F1    | Change |
| -------- | ----- | ------ |
| 01_2512  | 0.853 | 0.000  |
| one_tool | 0.753 | 0.000  |

The protection was not the cause - the issue is elsewhere in the pipeline.

## Next Steps for OODA-37

1. Trace the Figure 4 caption through block_builder.rs
2. Check if it's being merged incorrectly with other blocks
3. Look for any filtering in the render phase

## Code Changes Made

```rust
// OODA-36 FIX: Protect figure/table captions from being classified as footer/header.
let is_figure_or_table_caption = {
    let trimmed = elem.text.trim();
    (trimmed.starts_with("Figure ") || trimmed.starts_with("Table "))
        && trimmed.len() > 8
        && {
            // Check for "Figure N." or "Table N." pattern
            let after_prefix = if trimmed.starts_with("Figure ") {
                &trimmed[7..]
            } else {
                &trimmed[6..]
            };
            let digit_count = after_prefix
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .count();
            digit_count >= 1 && after_prefix.chars().nth(digit_count) == Some('.')
        }
};
```

## Learnings

1. Caption protection was a valid defensive measure
2. The root cause is NOT footer/header classification
3. The issue is further downstream in the pipeline
4. Debug logging (eprintln!) is essential for tracing text flow
