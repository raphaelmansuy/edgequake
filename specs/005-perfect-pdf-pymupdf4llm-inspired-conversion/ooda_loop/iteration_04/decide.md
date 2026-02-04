# OODA Iteration 04 - Decide

## Date: 2026-02-04

## Decisions

### Decision 1: Implement Smart Sort Key Algorithm ✅

**What**: Port pymupdf4llm's `join_rects_phase3` sort key algorithm to Rust.

**Where**: `edgequake/crates/edgequake-pdf/src/layout/reading_order.rs`

**Algorithm**:

```rust
fn compute_smart_sort_key(block: &Block, all_blocks: &[Block]) -> (f32, f32) {
    // Find left-most block with vertical overlap
    let left_overlapping: Vec<_> = all_blocks.iter()
        .filter(|other| {
            other.bbox.x1 < block.bbox.x0  // strictly to the left
            && has_vertical_overlap(&other.bbox, &block.bbox)
        })
        .collect();

    if let Some(leftmost) = left_overlapping.iter()
        .max_by(|a, b| a.bbox.x1.partial_cmp(&b.bbox.x1).unwrap())
    {
        (leftmost.bbox.y0, block.bbox.x0)  // Use P's y0, Q's x0
    } else {
        (block.bbox.y0, block.bbox.x0)  // Default key
    }
}
```

**Why**: This ensures blocks in different columns but same visual row are sorted left-to-right.

### Decision 2: Change Line Tolerance to 3pt

**What**: Change `line_tolerance` from 5.0 to 3.0 in `GroupingParams::default()`.

**Where**: `edgequake/crates/edgequake-pdf/src/layout/pymupdf_grouper.rs:34`

**Why**: pymupdf4llm uses 3pt. The 5pt value was a workaround that causes line merging.

### Decision 3: Test Before and After

**What**: Run `scripts/eval_comprehensive.py` before and after changes.

**Why**: Need to measure actual impact on ROUGE-L and Quality score.

---

## Implementation Order

1. Run baseline evaluation (capture current metrics)
2. Implement smart sort key in `reading_order.rs`
3. Change line_tolerance to 3pt
4. Run evaluation (measure improvement)
5. Commit with OODA-04 message

---

## Commit Message Template

```
OODA-04: Implement smart sort key for reading order

- Port pymupdf4llm join_rects_phase3 sort algorithm to Rust
- Smart key: (P.y0, Q.x0) where P is left-most overlapping block
- Fixes multi-column interleaving issues
- Change line_tolerance from 5pt to 3pt (pymupdf4llm default)

Quality impact:
  - Before: Quality=0.573, ROUGE-L=0.491
  - After: Quality=X.XXX, ROUGE-L=X.XXX
```
