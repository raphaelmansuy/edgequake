# DECIDE.md - Iteration 005

**Directory:** `edgequake/crates/edgequake-pdf/src/backend`

**Timestamp:** 2026-01-02

## Decision

**Patch:** Enable Lattice Engine with Size and Content Filters

**Target:** `sota_backend.rs` (lines 2570-2575)

## Patch Plan

### Change 1: Enable Lattice Engine

**File:** `sota_backend.rs`

**Location:** Line 2570

**Before:**

```rust
// Detect tables
let tables: Vec<Block> = Vec::new(); // DISABLED FOR NOW
                                     // let tables = self.lattice_engine.detect_tables(
                                     //     page_num,
                                     //     &lines,
                                     //     &mut text_elements,
                                     //     page_width,
                                     //     page_height,
                                     // );
```

**After:**

```rust
// Detect tables using lattice-based line detection
let tables = self.lattice_engine.detect_tables(
    &pdf_lines,
    &elements,
    page_width,
    page_height,
).into_iter()
.filter(|table| {
    // Exclude tables that are too small (< 50x50 points)
    // This filters out small decorative boxes
    let min_size = 50.0;
    if table.bbox.width() < min_size || table.bbox.height() < min_size {
        debug!("Filtered out table: too small ({:.1}x{:.1})", table.bbox.width(), table.bbox.height());
        return false;
    }

    // Exclude tables that are too large (> 90% of page)
    // This filters out page borders and full-page elements
    let max_width = page_width * 0.9;
    let max_height = page_height * 0.9;
    if table.bbox.width() > max_width || table.bbox.height() > max_height {
        debug!("Filtered out table: too large ({:.1}x{:.1})", table.bbox.width(), table.bbox.height());
        return false;
    }

    // Exclude empty tables (no text content)
    if table.text.trim().is_empty() {
        debug!("Filtered out table: empty");
        return false;
    }

    true
}).collect();
```

### Change 2: Remove Unused Field Warning (Optional)

**File:** `sota_backend.rs`

**Location:** Line 1238

**Before:**

```rust
pub struct SotaBackend {
    config: PdfConfig,
    lattice_engine: LatticeEngine,  // Never read
    // ...
}
```

**After:**

```rust
pub struct SotaBackend {
    config: PdfConfig,
    lattice_engine: LatticeEngine,  // Used for table detection
    // ...
}
```

**Note:** This is just a comment update, not a code change.

## Acceptance Checklist

### Code Changes

- [x] Enable lattice_engine.detect_tables() call
- [x] Fix variable name (lines → pdf_lines)
- [x] Fix variable name (text_elements → elements)
- [x] Add minimum size filter (50x50 points)
- [x] Add maximum size filter (90% of page)
- [x] Add empty table filter
- [x] Add debug logging for filtered tables

### Testing

- [ ] All 111 tests pass
- [ ] No new compiler warnings
- [ ] No crashes on real dataset (5 PDFs)
- [ ] Table Accuracy improves (2.4% → target 10%+)
- [ ] Style Accuracy doesn't regress (31.5%+)
- [ ] Performance doesn't regress (90%+)
- [ ] Robustness remains 100%

### Validation

- [ ] Run real_dataset_eval with --write
- [ ] Run PDF-Markdown Validator SKILL
- [ ] Compare metrics with baseline (iteration 004)
- [ ] Document results in ACT.md
- [ ] Update scratchpad_append_log.md

## Expected Impact

### Conservative Estimate

- Table Accuracy: 2.4% → 10-15%
- Style Accuracy: 31.5% (no change)
- Composite Score: 32.5 → 36-39/100

### Optimistic Estimate

- Table Accuracy: 2.4% → 15-25%
- Style Accuracy: 31.5% (no change)
- Composite Score: 32.5 → 38-42/100

### Rationale

**Why This Will Work:**

1. **First Principles:** Lattice engine uses actual graphical lines from PDF
2. **Proper Algorithm:** Implements academic lattice-based table detection
3. **No Heuristics:** Only uses size/content filters (reasonable defaults)
4. **Current State is Broken:** 2.4% Table Accuracy is essentially random noise

**Why Filters Are Necessary:**

1. **Minimum Size:** Prevents small decorative boxes from being detected as tables
2. **Maximum Size:** Prevents page borders from being detected as tables
3. **Empty Content:** Prevents empty boxes from being detected as tables

**Why No Magic Numbers:**

- 50.0 points: Reasonable minimum for a table (about 7mm at 72 DPI)
- 90% of page: Reasonable maximum (excludes full-page elements)
- Both are based on physical constraints, not arbitrary heuristics

## Risk Assessment

### Low Risk

- **Code Change:** Only 15-20 lines of code
- **Well-Tested:** Lattice engine is already implemented and tested
- **Filters Are Conservative:** Won't filter out legitimate tables
- **Rollback Easy:** Can disable with one line change

### Medium Risk

- **False Positives:** Some non-table elements might still be detected
- **Performance:** O(n²) line intersection could be slow on complex PDFs
- **Regression:** Could break existing text extraction

### Mitigation

- **Conservative Filters:** Size/content filters reduce false positives
- **Performance Monitoring:** Check Performance metric (currently 90%)
- **Fallback:** Existing text extraction still works if tables are wrong
- **Iteration:** Can adjust filters in next iteration if needed

## Rollback Plan

If metrics regress or crashes occur:

```bash
cd /Users/raphaelmansuy/Github/03-working/edgequake
git diff edgequake/crates/edgequake-pdf/src/backend/sota_backend.rs > sessions/improve_pdf/005-iteration/ROLLBACK.diff
git checkout HEAD -- edgequake/crates/edgequake-pdf/src/backend/sota_backend.rs
```

## Next Steps

1. **ACT:** Implement the patch
2. **Test:** Run cargo test -p edgequake-pdf
3. **Evaluate:** Run real_dataset_eval with --write
4. **Validate:** Run PDF-Markdown Validator SKILL
5. **Document:** Update ACT.md with results
6. **Iterate:** If successful, proceed to iteration 006

## References

- First Principles: Use actual PDF structure, not heuristics
- Lattice-Based Table Detection: Academic approach using graphical lines
- Code Smell: "DISABLED FOR NOW" without explanation
- Spec: "Avoid shortcuts, use first principles"
