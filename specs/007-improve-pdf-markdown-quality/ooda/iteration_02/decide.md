# OODA Iteration 02 - Decide

**Mission**: Improve PDF-to-Markdown Conversion Quality
**Date**: 2026-02-05

---

## Selected Option

**Option A: Fix left-block finder in compute_smart_sort_key()**

---

## Implementation Plan

### Step 1: Modify compute_smart_sort_key()

**File**: `edgequake/crates/edgequake-pdf/src/layout/reading_order.rs`
**Location**: ~line 354

Change from:

```rust
if let Some((_, left_block)) = left_blocks
    .iter()
    .max_by(|(_, a), (_, b)| a.bbox.x2.partial_cmp(&b.bbox.x2).unwrap())
```

To:

```rust
if let Some((_, left_block)) = left_blocks
    .iter()
    .min_by(|(_, a), (_, b)| a.bbox.x1.partial_cmp(&b.bbox.x1).unwrap())
```

### Step 2: Update WHY comment

Explain why left-most (not right-most) is correct per PyMuPDF4LLM algorithm.

### Step 3: Run tests

```bash
cargo test --lib reading_order
cargo test --lib
```

### Step 4: Add regression test

Test case verifying that right-column blocks use left-column Y when computing sort key.

---

## Acceptance Criteria

- [x] Change max_by(x2) to min_by(x1)
- [x] Add WHY comment explaining the fix
- [x] All 497 tests pass
- [x] New regression test for left-block finder

---

## Rollback Plan

If tests fail, revert to max_by(x2) and investigate further.
