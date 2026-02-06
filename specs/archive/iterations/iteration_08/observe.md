# OODA-08 Observe: Word Splitting Issue

## Current State

- Quality: 0.724 (target >= 0.95)
- ROUGE-L: 0.700 (target >= 0.90)
- Word F1: 0.897 (target >= 0.95)

## Observation: Failing Test

`test_qwen_reading_order` was failing with word splitting:

- "Push ing" instead of "Pushing"
- "Thin king" instead of "Thinking"
- "re inforcement" instead of "reinforcement"

## Root Cause Discovery

### Trace Analysis

Using `RUST_LOG=edgequake_pdf=info`, traced the merge_line output:

```
MERGE-GAP: prev='Push' x=183.5 w=33.0 | curr='ing Qwen3-Max-Th' x=284.7 | gap=68.2 thresh=54.0
```

The issue: `prev.width=33.0` for "Push" (4 chars _ 60pt _ 0.55 = 132 expected, not 33!)

### Width Field Bug

In `element_processing.rs`, the `merge()` function combines adjacent text elements:

1. At line 203: `current.text.push_str(&next.text);` - text grows
2. At line 206: `current_end_x = current_end_x.max(next_end_x);` - tracking var updated
3. **BUG**: `current.width` is NEVER updated!

When the merged element reaches `merge_line()`, it still has the original single-character width.

### Gap Calculation Error

With incorrect width (before fix):

- prev.x=183.5, prev.width=33 → ends at 216.5 (WRONG)
- curr.x=284.7
- gap = 284.7 - 216.5 = **68.2** (false positive large gap)
- threshold = 54.0
- Since gap > threshold, space inserted → "Push ing"

With correct width (after fix):

- prev.x=183.5, prev.width=132 → ends at 315.5
- curr.x=284.7
- gap = 284.7 - 315.5 = **-30.8** (small overlap due to width overestimate)

## Key Insight

The 0.55 char_width_factor overestimates width for tightly-kerned fonts (actual ~0.42 for font_size 60). This creates small negative gaps when elements overlap slightly, which is normal for character-by-character PDFs.

The OODA-42 check `gap < -(avg_font_size * 0.5)` was too aggressive (-30pt threshold for 60pt font), inserting spaces for width estimation errors.

## Files Examined

1. `backend/element_processing.rs` - ElementProcessor::merge() - **ROOT CAUSE**
2. `backend/text_grouping.rs` - TextGrouper::merge_line() - OODA-42 threshold
3. `backend/elements.rs` - TextElement struct (width field)
