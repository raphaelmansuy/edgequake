# OODA-09 Orient: First Principles Analysis of Text Merging

## First Principles: How PDF Text Should Be Read

According to Donald Knuth's principles of typography and typesetting:

1. **Word Separation**: Words are separated by horizontal whitespace (space characters or gaps)
2. **Line Separation**: Lines are separated by vertical displacement (different Y-coordinates)
3. **Paragraph Separation**: Paragraphs have larger vertical gaps than line spacing
4. **Column Separation**: Columns are separated by horizontal gaps AND different reading sequences

### The Core Problem

```
┌─────────────────────────────────────────────────────────────┐
│  PDF Two-Column Layout                                       │
│  ┌───────────────────┐     ┌───────────────────┐            │
│  │ Left Column       │     │ Right Column      │            │
│  │ ───────────────── │     │ ───────────────── │            │
│  │ Text A continues  │     │ manipulate objects│            │
│  │ for oper-         │     │ during the task.  │            │
│  │ ───────────────── │     │ ───────────────── │            │
│  │ ating in human... │     │ This requires...  │            │
│  └───────────────────┘     └───────────────────┘            │
│                                                              │
│  CORRECT Reading Order:                                      │
│  1. "Text A continues for oper-"                             │
│  2. "ating in human..."                                      │
│  3. "manipulate objects"                                     │
│  4. "during the task."                                       │
│                                                              │
│  INCORRECT (Current Bug):                                    │
│  1. "Text A continues for oper-manipulate objects"           │
│  2. "ating in human...during the task."                      │
└─────────────────────────────────────────────────────────────┘
```

## Root Cause: X-Gap Detection in merge_line()

Looking at `text_grouping.rs` merge_line() function:

```rust
// Calculate typical letter spacing from NON-WHITESPACE elements only
let typical_spacing = /* average spacing between adjacent elements */

// word_gap_threshold = typical_spacing * 1.5
let word_gap_threshold = typical_spacing * 1.5;

// If spacing > threshold, insert space
if spacing > effective_threshold && !starts_with_punct {
    text.push(' ');
}
```

**Problem**: The threshold is calculated globally across the entire line. When elements from **different columns** (X-gap > 200pt) are in the same line group, the average spacing is skewed high, making the threshold too permissive.

### Evidence from Real Data

From logs:

```
Column stats: left avg_len=55.4 X=[54.0,298.5], right avg_len=53.0 X=[306.0,379.6]
```

The column boundary is at ~300pt. Elements in left column have X range [54, 298.5], right column [306, 379.6].

Gap between columns: 306 - 298.5 = 7.5pt (visually small but semantically a column break)

This small gap doesn't trigger a space insertion because the X-gap threshold is calculated from all elements including the within-column letter spacing (~6pt for 12pt font).

## Root Cause: Line Grouping Y-Tolerance

In `group_single_column_layout()`:

```rust
let y_tolerance = elem.font_size * 0.5;  // 6pt for 12pt font
if y_diff > y_tolerance {
    // New line
}
```

**Problem**: Elements from left column (Y=386) and right column (Y=374) might have Y-diff of only 12pt. If font size is 24pt, tolerance is 12pt, so they might be grouped together!

But looking at the code, left and right columns ARE processed separately:

```rust
// Process each column into lines
let left_lines = self.group_single_column_layout(left_column);
let right_lines = self.group_single_column_layout(right_column);
```

So the column separation is happening correctly at the line grouping stage.

## The REAL Problem: Block Merging Cross-Column

Looking at `block_builder.rs` logs:

```
BLOCK-XRANGE: pos=10 bbox=[54.0,298.5] range=244.5 text='results for cases where...'
```

This block has X-range of 244.5pt spanning almost the entire left column. But wait - the text includes content from BOTH columns!

**Root Cause Found**: The issue is NOT in merge_line() but in how elements are assigned to columns in `group_two_column_layout()`:

```rust
// Margin around column boundary for classification
let margin = 15.0;

if elem.x < column_boundary - margin {
    left_column.push(elem);
} else if elem.x > column_boundary + margin {
    right_column.push(elem);
} else {
    // Element is in the GAP between columns
    if elem.x < column_boundary {
        left_column.push(elem);
    } else {
        right_column.push(elem);
    }
}
```

Elements with X in range [285, 315] go to the GAP handling. The gap handling uses `column_boundary` (300) as the decision point, which should work correctly.

## True Root Cause: Hyphen Continuation Not Working

Looking at the markdown output again:

```
Abstract- Humanoid robots hold great promise for oper-manipulate objects
```

The text "oper-" ends with a hyphen - this is a **hyphenated word continuation**. The next word should be "ating" (completing "oper-ating"), but instead it's "manipulate" from the right column!

**This means**: The hyphenated word from left column is being joined with the WRONG text. The issue is:

1. Left column ends with: "for oper-"
2. Left column next line starts with: "ating in human..."
3. Right column has: "manipulate objects"

Somehow, "oper-" is being merged with "manipulate" instead of "ating".

## Hypothesis Refined

The issue is in the **reading order after column processing**:

```rust
// WHY: Academic papers are read column-by-column
result.extend(left_main);  // All left column content
result.extend(right_main); // All right column content
```

This is CORRECT for reading order. But then in `block_builder.rs` or later processors:

1. Lines are converted to blocks
2. `BlockMergeProcessor` might be merging blocks from the same Y-band
3. `HyphenContinuationProcessor` might be incorrectly joining hyphenated words

Let me check the HyphenContinuation processor:

## Action Plan

1. **Verify hyphen handling**: Check if `HyphenContinuationProcessor` is active and working correctly
2. **Check block merge order**: Ensure blocks are processed in correct reading order
3. **Add column boundary awareness**: The hyphen continuation should only join words within the same column
