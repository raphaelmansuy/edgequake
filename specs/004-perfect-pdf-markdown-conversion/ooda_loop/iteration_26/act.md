# OODA-26 Act: Y-Sort Removal from Cross-Column Merge

## Changes Implemented

### File: `src/processors/layout_processing.rs`

**Lines:** 622-646

### Implementation

The Y-sort removal was implemented as decided. The key changes:

1. **Replaced Y-sort with position-preserving logic**
   - Original code sorted blocks by Y-coordinate, which interleaved columns
   - New code preserves the extraction engine's column-first order (established in OODA-12)

2. **HashMap for merged block tracking**
   - `new_blocks` now collected as `HashMap<usize, Block>`
   - Key = index of LEFT block, Value = merged block
   - Allows O(1) lookup during reconstruction

3. **Position-preserving reconstruction**
   - Iterate original block indices
   - If index in `merged_indices`: check if it was LEFT block (has entry in HashMap)
   - Insert merged block at LEFT block's position, skip RIGHT block
   - Non-merged blocks pass through unchanged

### Code Change

```rust
// OODA-26 FIX: Build final result PRESERVING original reading order
// WHY: The extraction engine established column-first order (OODA-12).
// Sorting by Y would interleave columns, destroying reading order.
// Instead, insert merged blocks at the position of their LEFT component.
let new_blocks_map: std::collections::HashMap<usize, Block> =
    new_blocks.into_iter().collect();

let mut final_blocks: Vec<Block> = Vec::with_capacity(result.len());

for (idx, block) in result.into_iter().enumerate() {
    if merged_indices.contains(&idx) {
        // Check if this was the LEFT block of a merge
        if let Some(merged_block) = new_blocks_map.get(&idx) {
            // Insert merged block at this position
            final_blocks.push(merged_block.clone());
        }
        // Skip RIGHT blocks (already merged into left)
    } else {
        final_blocks.push(block);
    }
}

// NO Y-SORT! Preserve the column-first reading order from extraction engine.
// WHY: OODA-12 specifically skips Y-sort for multi-column pages to ensure
// left column is read completely before right column.
```

## Test Results

```
cargo test --package edgequake-pdf --test quick_smoke
test result: ok. 4 passed; 0 failed
```

## Expected Outcome

The reading order for two-column PDFs should now be:

1. Complete left column from top to bottom
2. Complete right column from top to bottom

Instead of the previous interleaved order that mixed content from both columns.

## Next Steps (OODA-27)

- Run comprehensive quality tests to measure impact
- Verify reading order accuracy (ROA) improvement
- Address any regressions in single-column documents
