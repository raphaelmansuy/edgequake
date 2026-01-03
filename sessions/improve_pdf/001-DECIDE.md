# OODA Loop 1 - DECIDE

## Directory: `crates/edgequake-pdf/src/backend`

## Selected Patch: Relax crossing_ratio threshold

### Change

**File:** `crates/edgequake-pdf/src/backend/lattice.rs`  
**Line:** 360  
**Current:** `if crossing_ratio > 0.1`  
**New:** `if crossing_ratio > 0.35`

### First Principles Justification

**Fundamental truth:** PDF text extraction produces word-level elements, not cell-level content. Multi-line cells inherently have:

- Words on different Y coordinates
- Variable horizontal positions (due to text flow and alignment)
- Approximate character width calculations

**Mathematical reasoning:**

- A 3-line cell with 10 words = 10 text elements
- If cell spans 2 detected "columns" (due to alignment variation), 40% of words might appear to "cross"
- This is LEGITIMATE table structure, not noise

**Threshold selection:**

- `0.1` (10%) = too strict, rejects most real multi-line cells
- `0.35` (35%) = allows multi-line cells while rejecting severely malformed grids
- `0.5` (50%) = too permissive, might accept non-table layouts

**Choice: 0.35** balances precision (don't accept non-tables) and recall (accept real tables with multi-line cells).

### Predicted Impact

**Table Accuracy:** 2.4% → 15-20% (estimated)  
**Composite Score:** 32.4 → 42-47 (estimated +10-15 points)

**Why:** Based on observation that most table rejections are false positives:

- `crossing_ratio=0.12`: Would now ACCEPT
- `crossing_ratio=0.25`: Would now ACCEPT
- `crossing_ratio=0.40`: Still REJECT (likely malformed)
- `crossing_ratio=0.85`: Still REJECT (definitely noise)

### Acceptance Checklist

- [ ] Patch applied: `lattice.rs` line 360 updated
- [ ] Unit tests pass: `cargo test -p edgequake-pdf`
- [ ] Real dataset eval runs: `cargo run --example real_dataset_eval -- --write`
- [ ] Validator SKILL shows improvement: Table Accuracy > 15%
- [ ] No regressions: Robustness remains 100%, no new crashes
- [ ] Artifacts produced: `PATCH.diff`, updated metrics in `001-ACT.md`

### Risk Assessment

**Low risk:**

- Single-line change
- Only affects table acceptance threshold
- Doesn't change cell extraction logic
- Can be quickly reverted if wrong

**Failure mode:** Might accept some non-table layouts as tables  
**Mitigation:** Monitor cell content accuracy; if it drops, threshold was too high

## Next: ACT phase
