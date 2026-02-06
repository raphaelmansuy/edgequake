# OODA-08 Act: Word Splitting Fix Implementation

## Date: 2026-02-04

## Changes Made

### Fix 1: Update width during merge

**File**: `backend/element_processing.rs`
**Lines**: 200-210

```rust
current.text.push_str(&next.text);
// OODA-08: Update width to reflect merged text length
// WHY: Without this, merge_line() uses stale width from original element
// which causes incorrect gap calculations and spurious space insertions
// (e.g., "Push ing" instead of "Pushing")
current.width =
    current.text.chars().count() as f32 * current.font_size * self.char_width_factor;
```

### Fix 2: Relax OODA-42 threshold

**File**: `backend/text_grouping.rs`
**Lines**: 1140-1170

Changed threshold from `-(avg_font_size * 0.5)` to `-avg_font_size`:

```rust
// OODA-08 refinement: Distinguish between:
// 1. Moderate negative gap (width estimation error): Just merge without space
// 2. Extreme negative gap (line grouping error): Insert space as separator
//
// The width estimation uses 0.55 * font_size per character, but actual
// character widths vary. For tightly-kerned fonts, actual width can be
// 0.4-0.45 * font_size. This can create ~25% overestimate.
// For a 4-char element at font_size 60: estimated width = 132, actual ~100
// Max gap error = ~32 points.
//
// We use font_size * 1.0 (100%) as the threshold for "real line break":
// - gap in (-font_size, 0): Width estimation error → merge without space
// - gap < -font_size: True line break → insert space
let significant_overlap = gap < -avg_font_size;
```

## Test Results

| Test                          | Result  |
| ----------------------------- | ------- |
| `test_qwen_reading_order`     | ✅ PASS |
| `test_arxiv_paper_extraction` | ✅ PASS |

## Output Verification

Before:

```
# Push ing Qwen3-Max-Th
```

After:

```
# Pushing Qwen3-Max-Th
```

## Quality Metrics

| Metric  | Before | After | Change |
| ------- | ------ | ----- | ------ |
| Quality | 0.724  | 0.724 | ±0.000 |
| ROUGE-L | 0.700  | 0.700 | ±0.000 |
| Word F1 | 0.897  | 0.897 | ±0.000 |

Note: Metrics unchanged because the fix is targeted at specific word splitting cases in Qwen.pdf which isn't in the gold standard test set.

## Commit

```bash
git add -A
git commit -m "OODA-08: Fix word splitting (Push ing → Pushing)"
```
