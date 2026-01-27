# DECIDE.md - Iteration 001

**Directory:** `crates/edgequake-pdf/src/backend`

## Selected Patch

**Title:** Remove final block sort that destroys reading order

**File:** `sota_backend.rs`

**Change:** Remove lines 2378-2379:

```rust
// Sort blocks by Y (top to bottom) - PDF Y is up, so sort descending by max Y (top)
blocks.sort_by(|a, b| b.bbox.y2.partial_cmp(&a.bbox.y2).unwrap_or(std::cmp::Ordering::Equal));
```

**Rationale:**

1. The `group_into_lines()` function already establishes correct reading order
2. For single-column layouts, `group_single_column_layout()` sorts by Y descending
3. For two-column layouts, `group_two_column_layout()` processes columns sequentially
4. The final sort destroys this carefully-constructed order

**Expected Impact:**

- Content drifts: -400 to -450 (from 468)
- Heading mismatches: -20 (from 27)
- Overall score improvement: +30-40 points

## Alternative Patches Considered

### Alternative A: Conditional Sort

Only sort if columns were not detected:

```rust
if columns.is_empty() {
    blocks.sort_by(|a, b| b.bbox.y2.partial_cmp(&a.bbox.y2).unwrap_or(std::cmp::Ordering::Equal));
}
```

**Rejected:** Single-column layout is already sorted by `group_single_column_layout()`.

### Alternative B: Column-Aware Sort

Sort within columns but preserve column order:

```rust
// Sort respecting column boundaries
```

**Rejected:** More complex, and the existing column processing already handles this.

## Acceptance Checklist

- [ ] `cargo test -p edgequake-pdf` passes (102/102)
- [ ] Run `real_dataset_eval --write` with no crashes
- [ ] Validator SKILL shows content drifts reduced by >50%
- [ ] Reading order in 2900_Goyal_et_al.mdf.gen matches gold file order
- [ ] Composite score improves by >15 points

## Risks

- **Low:** Some edge cases with mixed layouts might need adjustment
- **Mitigation:** The column detection is robust and tested

## Commit Message Template

```
fix(pdf): Remove final block sort that destroys reading order

Directory: crates/edgequake-pdf/src/backend

The sort at line 2379 was re-ordering blocks by Y position AFTER
column-based reading order had been established. This caused content
from different columns to be interleaved and paragraphs to appear
in reverse order.

The reading order is now preserved from group_into_lines() which
correctly handles both single-column and multi-column layouts.

Fixes content mismatch drifts in real_dataset evaluation.
```
