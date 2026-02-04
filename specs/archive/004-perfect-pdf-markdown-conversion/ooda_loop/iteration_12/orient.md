# OODA-12: Orient

## Problem Analysis

The root cause is clear: `blocks.sort_by(Y)` in `extraction_engine.rs` line 634 destroys the column-aware reading order established by `text_grouping.rs`.

## Why Does This Sort Exist?

The comment suggests it's needed because:

> "The text grouping may return lines in an order that reflects the order elements were parsed from the PDF (not necessarily top-to-bottom)."

This may have been true for **single-column** layouts, but for **multi-column** layouts, `text_grouping.rs` already:

1. Separates elements into left and right columns
2. Processes each column top-to-bottom
3. Concatenates left column lines, then right column lines

## Fix Options

### Option A: Remove Sort for Multi-Column Pages

```rust
// Only sort if single-column layout
if columns.len() <= 1 {
    blocks.sort_by(|a, b| {
        a.bbox.y1.partial_cmp(&b.bbox.y1).unwrap_or(std::cmp::Ordering::Equal)
    });
}
```

**Pros:**

- Simple fix
- Preserves text_grouping's column order for multi-column

**Cons:**

- May need Y-sort within each column (if text_grouping doesn't guarantee Y-order)
- Binary behavior might miss edge cases

### Option B: Column-Aware Sort (Stable Sort Within Columns)

```rust
if columns.len() > 1 {
    // Multi-column: Sort within each column, but preserve column order
    let column_boundary = columns.get(0).map(|c| c.x2).unwrap_or(page_width / 2.0);

    // Split blocks by column
    let (mut left, mut right): (Vec<_>, Vec<_>) = blocks.into_iter()
        .partition(|b| b.bbox.x1 < column_boundary);

    // Sort each column by Y
    left.sort_by(|a, b| a.bbox.y1.partial_cmp(&b.bbox.y1).unwrap());
    right.sort_by(|a, b| a.bbox.y1.partial_cmp(&b.bbox.y1).unwrap());

    // Recombine: left column first, then right column
    blocks = left;
    blocks.extend(right);
} else {
    // Single-column: simple Y-sort
    blocks.sort_by(|a, b| a.bbox.y1.partial_cmp(&b.bbox.y1).unwrap());
}
```

**Pros:**

- Guarantees Y-order within each column
- Preserves left-then-right reading order
- Handles cases where text_grouping might not return perfect Y-order

**Cons:**

- More code
- Need to handle column boundary detection

### Option C: No Sort At All

```rust
// Trust text_grouping's order completely
// blocks.sort_by(...); // REMOVED
```

**Pros:**

- Simplest change
- text_grouping already sorts elements by Y within columns

**Cons:**

- If text_grouping has any order issues, they propagate
- May break single-column layouts that rely on Y-sort

## Risk Assessment

| Option | Risk Level | Complexity | Correctness                                |
| ------ | ---------- | ---------- | ------------------------------------------ |
| A      | Low        | Low        | High for multi-col, unknown for edge cases |
| B      | Low        | Medium     | High for all layouts                       |
| C      | Medium     | Minimal    | Depends entirely on text_grouping          |

## First Principles Analysis

**Question**: What is the correct reading order for a two-column page?

**Answer**: Read left column top-to-bottom, then right column top-to-bottom.

**Question**: Does text_grouping already produce this order?

**Answer**: YES. Looking at `text_grouping.rs` lines 479-491:

```rust
result.extend(left_main);      // All left column
result.extend(right_main);     // Then all right column
result.extend(left_footer_lines);
result.extend(right_footer_lines);
```

**Question**: Does block_builder preserve this order?

**Answer**: YES. It iterates through lines sequentially and appends blocks.

**Conclusion**: The Y-sort is unnecessary and harmful for multi-column pages.

## Verification of text_grouping's Y-Order

Looking at `group_single_column_layout` (line 608):

```rust
// Sort by Y ascending (lower Y = top of page after normalization)
elements.sort_by(|a, b| a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal));
```

**text_grouping already sorts by Y within each column!** The Y-sort in extraction_engine is redundant and destructive.

## Recommendation

**Option A** is the correct fix:

1. If `columns.len() <= 1`, sort by Y (single-column behavior unchanged)
2. If `columns.len() > 1`, trust text_grouping's order (column-aware)

This minimizes risk while fixing the multi-column bug.

## Fallback

If Option A introduces regressions, fall back to Option B which re-validates Y-order within each column before recombining.
