# OODA-07 Decide: Fix Smart Sort Y-Coordinate

## Decision

Fix the Y-coordinate used in `compute_smart_sort_key` to use `y1` (top) instead of `y0` (bottom).

## Changes

### File: `layout/pymupdf_grouper.rs`

#### Change 1: Line ~536

**Before:**

```rust
left_block.y0 as i32
```

**After:**

```rust
left_block.y1 as i32  // y1 = TOP in PDFium coords
```

#### Change 2: Line ~540

**Before:**

```rust
block.y0 as i32
```

**After:**

```rust
block.y1 as i32  // y1 = TOP in PDFium coords
```

## Rationale

1. PyMuPDF uses origin at top-left, so y0 = top
2. PDFium uses origin at bottom-left, so y1 = top
3. The sort key should use "top of block" for proper reading order
4. Left column blocks should be read before right column blocks with same Y

## Verification

1. Build: `cargo build --release --features pdfium -p edgequake-pdf`
2. Test: `python3 scripts/eval_comprehensive.py`
3. Target: ROUGE-L > 0.75, Quality > 0.75
