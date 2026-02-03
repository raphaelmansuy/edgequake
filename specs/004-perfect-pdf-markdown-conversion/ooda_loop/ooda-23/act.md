# OODA-23 Act: Cross-Column Hyphenation Fix

## Problem Identified

Academic papers sometimes have sentences that span column boundaries. When this happens:

1. Word is hyphenated at end of left column: "reposito-"
2. Continuation appears at start of right column: "ries remains"
3. But other elements (figure captions) may have lower Y and appear first

## Visual Diagram

```
                     PDF Layout (Two Columns)
    ┌─────────────────────────────────────────────────────┐
    │   LEFT COLUMN              RIGHT COLUMN             │
    │                                                     │
    │   ...text...               Figure 1. caption        │ ← Y=171
    │   ...text...               (floating element)       │
    │   reposito-                                         │ ← Y=545 LEFT
    │                            ries remains limited     │ ← Y=232 RIGHT
    │                            (sentence continues)     │
    │                                                     │
    └─────────────────────────────────────────────────────┘

    Current Output Order:
    1. "reposito-"
    2. "Figure 1. caption"  ← WRONG - interrupts sentence
    3. "ries remains limited"

    Expected Output Order:
    1. "reposito-" + "ries remains limited" (merged)
    2. "Figure 1. caption" (after paragraph)
```

## Root Cause

The reading order algorithm processes columns sequentially:

1. All of left column (top to bottom)
2. All of right column (top to bottom)

When a hyphenated word spans columns, the continuation is in the right column but sorted by Y position, causing figure captions (with lower Y) to appear first.

## Solution: Hyphenation-Aware Column Bridging

When the last block of left column ends with hyphen + lowercase continuation pattern:

1. Find the first block in right column that starts with lowercase continuation
2. Move that block to immediately after the left column block
3. Then continue with remaining right column content

## Implementation Location

`src/layout/reading_order.rs` in `merge_column_orders()` or `multi_column_order()`

## Code Change

Add after processing all left column blocks:

```rust
// OODA-23: Detect cross-column hyphenation
// WHY: Sentences may span column boundaries with hyphenation
// e.g., "reposito-" (left column) + "ries remains" (right column)
if let Some(last_left) = column_blocks[0].last() {
    let last_left_block = &blocks[*last_left];
    if last_left_block.text.trim_end().ends_with('-') {
        // Find continuation in right column
        if let Some(first_right_pos) = column_blocks.get(1).and_then(|v| v.first()) {
            let first_right = &blocks[*first_right_pos];
            let starts_lowercase = first_right
                .text
                .trim_start()
                .chars()
                .next()
                .map(|c| c.is_lowercase())
                .unwrap_or(false);

            if starts_lowercase {
                // Move first right block to immediately after left column
                // This ensures hyphenated word is merged properly
                tracing::info!(
                    "OODA-23: Cross-column hyphenation detected: '{}' + '{}'",
                    &last_left_block.text[..last_left_block.text.len().min(30)],
                    &first_right.text[..first_right.text.len().min(30)]
                );
                // Reorder: insert first_right_pos after last_left
            }
        }
    }
}
```

## Expected Outcome

- "reposito-" + "ries remains" properly merged
- Figure captions appear after completed sentences
- TPS improvement: +2-3%

## Risk Assessment

Low risk - only affects edge case where:

1. Block ends with hyphen
2. Next column starts with lowercase
3. Both are in same document

## Testing

Will verify with comprehensive quality tests after implementation.
