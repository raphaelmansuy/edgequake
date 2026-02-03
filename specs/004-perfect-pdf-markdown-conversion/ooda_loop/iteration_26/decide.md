# OODA-26 Decide: Remove Y-Sort from Cross-Column Merge

## Decision

Remove the Y-sort at the end of `merge_cross_column_hyphenation()` to preserve column-first reading order.

## Implementation

**File:** `src/processors/layout_processing.rs`
**Location:** Lines 630-645 in `merge_cross_column_hyphenation()`

### Current Code (PROBLEMATIC)

```rust
// Build final result: non-merged blocks + newly merged blocks
let mut final_blocks: Vec<Block> = result
    .into_iter()
    .enumerate()
    .filter(|(idx, _)| !merged_indices.contains(idx))
    .map(|(_, block)| block)
    .collect();

final_blocks.extend(new_blocks);

// Sort by position (Y then X) to maintain reading order  <<< BUG!
final_blocks.sort_by(|a, b| {
    let y_cmp = a.bbox.y1.partial_cmp(&b.bbox.y1).unwrap();
    if y_cmp == std::cmp::Ordering::Equal {
        a.bbox.x1.partial_cmp(&b.bbox.x1).unwrap()
    } else {
        y_cmp
    }
});
```

### New Code (FIXED)

```rust
// Build final result preserving original reading order
// WHY: The extraction engine already established column-first order (OODA-12).
// We must NOT re-sort by Y, as that would interleave columns.
// Instead, insert merged blocks at the position of their left component.

let mut final_blocks: Vec<Block> = Vec::with_capacity(result.len());

for (idx, block) in result.into_iter().enumerate() {
    if merged_indices.contains(&idx) {
        // Check if this was the LEFT block of a merge (not the right)
        // New blocks are keyed by the left block index
        if let Some(pos) = new_blocks.iter().position(|nb|
            // Merged block starts where left block started
            (nb.bbox.x1 - block.bbox.x1).abs() < 1.0 &&
            (nb.bbox.y1 - block.bbox.y1).abs() < 1.0
        ) {
            // Insert merged block at this position
            final_blocks.push(new_blocks.remove(pos));
        }
        // Skip right blocks (already merged into left)
    } else {
        final_blocks.push(block);
    }
}

// No Y-sort! Preserve the column-first reading order.
```

## Expected Outcome

Before (interleaved):

```
...via Reinforcement
> Figure 1...
In the domain of SWE...repositowhich is realized...
ries remains limited...
```

After (column-first):

```
...via Reinforcement Learning (RL)...
(left column continues)
...repositories remains limited...
(left column continues to end)
> Figure 1...
(right column starts)
In the domain of SWE...
```

## Risk Assessment

| Risk                            | Mitigation                                                   |
| ------------------------------- | ------------------------------------------------------------ |
| Merged blocks at wrong position | Track merge positions by left block location                 |
| Breaking single-column PDFs     | Only affects merge_cross_column_hyphenation (multi-col only) |
| Position index misalignment     | Update positions after building final list                   |
