# OODA Iteration 02 - Act

**Mission**: Improve PDF-to-Markdown Conversion Quality
**Date**: 2026-02-05

---

## Changes Implemented

### 1. Fixed left-block finder in compute_smart_sort_key()

**File**: `edgequake/crates/edgequake-pdf/src/layout/reading_order.rs`

**Before** (WRONG):
```rust
if let Some((_, left_block)) = left_blocks
    .iter()
    .max_by(|(_, a), (_, b)| a.bbox.x2.partial_cmp(&b.bbox.x2).unwrap())
```

**After** (CORRECT):
```rust
// OODA-02 FIX: Use LEFT-MOST block (min x1), not RIGHT-MOST (max x2)
// WHY (First Principles - per pymupdf4llm multi_column.py lines 290-304):
// When sorting blocks for reading order, the left-most block with vertical
// overlap determines the sort key. This ensures that right-column content
// comes AFTER left-column content at the same vertical level.
if let Some((_, left_block)) = left_blocks
    .iter()
    .min_by(|(_, a), (_, b)| a.bbox.x1.partial_cmp(&b.bbox.x1).unwrap())
```

### 2. Added regression test

**Test**: `test_smart_sort_key_uses_leftmost_block`

Verifies that:
- When multiple left blocks overlap vertically with a right block
- The LEFT-MOST block's Y coordinate is used for sort key
- Not the RIGHT-MOST block's Y

---

## Test Results

```
cargo test --lib
test result: ok. 498 passed; 0 failed; 0 ignored
```

**Test count**: 497 → 498 (+1 regression test)

---

## Commit

```
OODA-IT02: Fix left-block finder in reading order smart sort

Changed max_by(x2) to min_by(x1) to match pymupdf4llm's algorithm:
- Find LEFT-MOST overlapping block (not RIGHT-MOST)
- Ensures correct sort key for right-column blocks

Added regression test: test_smart_sort_key_uses_leftmost_block
```

---

## Impact Assessment

- **Risk**: Very low - isolated change to sort key computation
- **Scope**: Reading order for multi-column documents
- **Quality impact**: Improves sorting accuracy for blocks at similar vertical levels
