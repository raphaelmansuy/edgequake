# OODA Iteration 04 - Act

## Date: 2026-02-04

## Actions Taken

### 1. Fixed Smart Sort Key Algorithm Call

**Bug Found**: The `sort_by_smart_key()` function was defined but NEVER CALLED!

**Location**: `edgequake/crates/edgequake-pdf/src/layout/reading_order.rs:179`

**Before**:

```rust
for col_blocks in &mut column_blocks {
    self.sort_by_position(col_blocks, blocks);  // Wrong function!
}
```

**After**:

```rust
for col_blocks in &mut column_blocks {
    self.sort_by_smart_key(col_blocks, blocks);  // OODA-04 FIX
}
```

### 2. Changed Line Tolerance to 3pt

**Files Modified**:

- `reading_order.rs:62`: Changed `line_tolerance: 5.0` → `line_tolerance: 3.0`
- `pymupdf_grouper.rs:34`: Changed `line_tolerance: 5.0` → `line_tolerance: 3.0`

**Why**: pymupdf4llm uses 3pt as the default. The 5pt value was a workaround causing lines to incorrectly merge.

---

## Results

### Before OODA-04:

| Metric    | Value |
| --------- | ----- |
| Quality   | 0.731 |
| ROUGE-L   | 0.698 |
| Word F1   | 0.893 |
| Structure | 0.595 |
| Format    | 0.573 |

### After OODA-04:

| Metric    | Value | Change |
| --------- | ----- | ------ |
| Quality   | 0.752 | +0.021 |
| ROUGE-L   | 0.711 | +0.013 |
| Word F1   | 0.915 | +0.022 |
| Structure | 0.650 | +0.055 |
| Format    | 0.572 | -0.001 |

### Per-File Improvements:

| File                  | Before | After | Change |
| --------------------- | ------ | ----- | ------ |
| v2_2512.25072v1       | 0.591  | 0.692 | +0.101 |
| 01_2512.25075v1       | 0.622  | 0.654 | +0.032 |
| one_tool_2512.20957v2 | 0.667  | 0.698 | +0.031 |
| ccn_2512.21804v1      | 0.702  | 0.737 | +0.035 |

---

## Commit Message

```
OODA-04: Fix smart sort key call, change line_tolerance to 3pt

BUG FIX: sort_by_smart_key() was defined but never called!
- reading_order.rs:179 was calling sort_by_position instead
- Now correctly calls sort_by_smart_key for column sorting

LINE TOLERANCE:
- Changed from 5pt to 3pt (pymupdf4llm default)
- Affects reading_order.rs and pymupdf_grouper.rs

Quality impact: 0.731 → 0.752 (+0.021)
ROUGE-L impact: 0.698 → 0.711 (+0.013)
```

---

## Next Steps

Main gaps remaining:

1. **Format Score (0.572)**: Need better bold/italic detection
2. **Structure Score (0.650)**: Need better heading hierarchy
3. **ROUGE-L (0.711)**: Reading order still needs work for complex layouts
