# OODA Improvement Session Summary

## Session: PDF → Markdown Quality Improvement

**Date:** 2025-01-15
**Spec:** specs/27-improve-pdf.md

---

## Iterations Completed

### Iteration 001: Reading Order Fix ✅

**Problem:** Content appeared bottom-to-top instead of top-to-bottom.

**Root Cause:** `ReadingOrderDetector` sorted by ascending Y coordinate, but PDF Y=0 is at the bottom of the page, so ascending Y gives bottom-to-top order.

**Solution:** Changed Y comparisons from `y_a.partial_cmp(&y_b)` to `y_b.partial_cmp(&y_a)` in:

- `single_column_order()`
- `sort_by_position()`
- `merge_column_orders()`

**Files Changed:** `edgequake/crates/edgequake-pdf/src/layout/reading_order.rs`

---

### Iteration 002: Ligature Handling Fix ✅

**Problem:** Words containing ligatures like "fi", "fl", "ff", "ffi", "ffl" were being corrupted:

- "first" → "frst"
- "specific" → "specic"
- "classification" → "classifcation"

**Root Cause:** Two issues:

1. `WIN_ANSI_ENCODING` has `None` at bytes 0x1B-0x1F (ligature positions), causing silent byte drops
2. Some PDFs have corrupted ToUnicode CMaps that map ligature bytes to just 'f' instead of 'fi'

**Solution:** Added `get_ligature_expansion()` function that handles:

- PostScript Type 1 positions: 0x02=fi, 0x03=fl, 0x04=ff, 0x05=ffi, 0x06=ffl
- Windows/Adobe positions: 0x1B=ffl, 0x1C=ffi, 0x1D=ff, 0x1E=fl, 0x1F=fi

Added fallback in:

- `Encoding::decode()` for `OneByteEncoding`
- `ToUnicodeMap::decode()` with corrupted CMap detection

**Files Changed:** `edgequake/crates/edgequake-pdf/src/backend/sota_backend.rs`

---

## Validation Results

### Before (Baseline)

| Metric                        | Value |
| ----------------------------- | ----- |
| Broken ligature words         | 12    |
| "first" in Goyal PDF          | 0     |
| "classification" in Goyal PDF | 0     |
| Validator crashes             | 3     |
| Documents processed           | 2/5   |

### After (Post-Iterations)

| Metric                        | Value |
| ----------------------------- | ----- |
| Broken ligature words         | 0     |
| "first" in Goyal PDF          | 4     |
| "classification" in Goyal PDF | 8     |
| Validator crashes             | 0     |
| Documents processed           | 5/5   |

### Composite Score

| Metric         | Score        |
| -------------- | ------------ |
| Table Accuracy | 3.5%         |
| Style Accuracy | 16.9%        |
| Robustness     | 100.0%       |
| Performance    | 90.0%        |
| **Composite**  | **27.2/100** |

---

## Test Status

- All 102 tests pass
- `cargo clippy` passes with warnings only
- `cargo fmt` passes

---

## Known Remaining Issues

1. **Style Detection Limited**: Bold/italic detection based only on font name (e.g., "Bold" substring)
2. **Table Accuracy Low**: Table cell matching needs improvement
3. **Heading Levels**: Some heading levels are detected incorrectly

---

## Session Artifacts

```
sessions/improve_pdf/
├── 001-iteration/
│   ├── OBSERVE.md
│   ├── ORIENT.md
│   ├── DECIDE.md
│   ├── ACT.md
│   └── PATCH.diff
└── 002-iteration/
    ├── OBSERVE.md
    ├── ORIENT.md
    ├── DECIDE.md
    ├── ACT.md
    └── PATCH.diff
```

---

## Recommendations for Future Iterations

1. **Font Weight Detection**: Parse `/FontDescriptor` for `/FontWeight` field instead of name-based detection
2. **Table Structure**: Improve cell grouping algorithm for multi-column tables
3. **Heading Hierarchy**: Use document-level font analysis to determine heading levels
4. **Post-Processing**: Add pattern-based text cleanup for common PDF artifacts
