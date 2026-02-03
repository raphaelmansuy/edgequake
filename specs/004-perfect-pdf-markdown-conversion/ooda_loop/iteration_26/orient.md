# OODA-26 Orient: Root Cause of Column Interleaving

## Key Finding: Y-Sort After Cross-Column Merge

The `merge_cross_column_hyphenation()` function in `layout_processing.rs` destroys the column-aware reading order.

### Current Processing Flow

```
┌───────────────────────────────────────┐
│ Extraction Engine                      │
│ → Establishes column-first reading     │
│ → Left col blocks, then right col      │
│ → OODA-12 skips Y-sort for multi-col  │
└───────────────────────────────────────┘
           │
           ▼
┌───────────────────────────────────────┐
│ LayoutProcessor                        │
│ → Skips re-sort if columns present     │
│ → Column-first order preserved ✓       │
└───────────────────────────────────────┘
           │
           ▼
┌───────────────────────────────────────┐
│ BlockMergeProcessor                    │
│ → Merges adjacent blocks               │
│ → Calls merge_cross_column_hyphenation │
│ → *** BUG: Re-sorts by Y at end ***   │
│ → Column order DESTROYED               │
└───────────────────────────────────────┘
           │
           ▼
┌───────────────────────────────────────┐
│ Output                                 │
│ → Columns interleaved by Y position    │
│ → "reposito" + "which is realized..."  │
└───────────────────────────────────────┘
```

### Problem Code (lines 634-641)

```rust
// Sort by position (Y then X) to maintain reading order
final_blocks.sort_by(|a, b| {
    let y_cmp = a.bbox.y1.partial_cmp(&b.bbox.y1).unwrap();
    if y_cmp == std::cmp::Ordering::Equal {
        a.bbox.x1.partial_cmp(&b.bbox.x1).unwrap()
    } else {
        y_cmp
    }
});
```

This Y-sort:

- Intended to maintain reading order after merging
- Actually DESTROYS the column-first order established earlier
- Interleaves left and right column content by vertical position

### Why This Matters

The extraction engine carefully establishes reading order:

1. Processes left column completely
2. Then processes right column
3. OODA-12 explicitly skips Y-sort for multi-column pages

But the BlockMergeProcessor undoes this work by re-sorting all blocks by Y.

## Solution

**Option A: Remove the Y-sort entirely** ✅ PREFERRED

- The reading order is already correct from the extraction engine
- Cross-column merges don't need to change block order
- Just insert merged blocks in place of their constituents

**Option B: Use column-aware sorting**

- After merge, sort by column first, then Y within column
- More complex, but preserves column structure

I'll implement Option A - simply preserve the original order.
