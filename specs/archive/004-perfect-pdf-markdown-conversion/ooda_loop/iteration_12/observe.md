# OODA-12: Observe

## Problem Statement

v2_2512.25072v1.pdf has the lowest structural fidelity score (53.6%) among all test PDFs. The REFERENCES section is particularly broken - references from left and right columns are interleaved on the same lines.

## Symptoms

### Gold File (Expected)

```
* [10] T. E. Truong... "Beyondmimic:..."
* [11] X. Huang Y. Chi... "Diffuseloco:..."
* [12] K. Black, N. Brown, ... L. Groom, K. Hausman... "πο:..."
* [13] A. Guzman-Rivera... "Multiple choice learning:..."
```

### Generated File (Actual)

```
- [1] L. Sentis... "A whole-body control framework for
humanoids..." Teleoperation with immersive active visual feedback," in CoRL  <- [2]'s content!
, 2024.
- [3] X. Gu... "Humanoid-gym:..."
locomotion through lipschitz-constrained policies," in IROS, 2025.  <- [4]'s content!
```

### Reference Counts

- Gold file: 44 references (properly formatted as `* [N] ...`)
- Generated file: 0 references in `* [N]` format, references interleaved within blocks

## Root Cause Discovery

### Pipeline Trace

```
PDF → elements → text_grouping.rs → lines → block_builder.rs → blocks → extraction_engine.rs → Page → render
                       ↓                           ↓                         ↓
                 [L1, L2, ..., R1, R2, ...]  [B1, B2, ..., Bn]       SORT BY Y!
                 (left then right)           (preserves order)       ↓
                                                              [B_interleaved]
                                                              (destroys order!)
```

### The Bug: Line 634 in extraction_engine.rs

```rust
// Sort blocks by Y coordinate for correct reading order
blocks.sort_by(|a, b| {
    a.bbox
        .y1
        .partial_cmp(&b.bbox.y1)
        .unwrap_or(std::cmp::Ordering::Equal)
});
```

**What happens:**

1. `text_grouping.rs` correctly separates left and right columns
2. Lines are returned in order: `[left_col_1, left_col_2, ..., right_col_1, right_col_2, ...]`
3. `block_builder.rs` converts lines to blocks, preserving this order
4. **BUG**: `extraction_engine.rs` line 634 sorts ALL blocks by Y-coordinate
5. Blocks at similar Y from both columns get interleaved!

### Visual Example

```
Two-column REFERENCES page layout:
┌─────────────────────┬─────────────────────┐
│  [1] Author1...     │  [2] Author2...     │  Y=600
│  [3] Author3...     │  [4] Author4...     │  Y=620
│  [5] Author5...     │  [6] Author6...     │  Y=640
└─────────────────────┴─────────────────────┘

After text_grouping (CORRECT order):
  blocks = [ref1, ref3, ref5, ..., ref2, ref4, ref6, ...]
           └── left column ───┘   └── right column ──┘

After blocks.sort_by(Y) (WRONG order):
  blocks = [ref1, ref2, ref3, ref4, ref5, ref6, ...]
           └─ sorted by Y, ignoring columns! ─┘
```

### Comment vs Reality

The code comment claims:

> "For multi-column layouts, the content is already organized by text_grouping (left column first, then right column), and the Y values within each column section will naturally sort correctly."

But the **actual behavior** sorts ALL blocks by Y, destroying the column organization.

## Evidence Collection

### Quality Metrics

```
📄 v2_2512.25072v1
   Text:  85.3% | Structure:  53.6% | Overall:  69.4%  ← Worst structure score
```

### Reference Content Missing

- "K. Black" (author of ref [12]): 0 occurrences in output
- "L. Groom" (co-author of ref [12]): 0 occurrences in output
- "K. Hausman" (co-author of ref [12]): 0 occurrences in output
- Entire reference [12] author list is missing/corrupted

## Files Involved

| File                               | Line    | Role                                       |
| ---------------------------------- | ------- | ------------------------------------------ |
| `src/backend/text_grouping.rs`     | 479-486 | Correctly orders left-then-right columns   |
| `src/backend/block_builder.rs`     | 59-205  | Converts lines→blocks, preserves order     |
| `src/backend/extraction_engine.rs` | **634** | **BUG**: sorts by Y, destroys column order |

## Verification Steps

1. Run test: `cargo test -p edgequake-pdf --test comprehensive_quality --features comprehensive-tests`
2. Check v2 structure score: Should be 53.6%
3. Check generated file: `grep -c "^\* \[" v2_2512.25072v1.md` returns 0
4. Check gold file: `grep -c "^\* \[" v2_2512.25072v1.gold.md` returns 44
