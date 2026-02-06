# OODA-08 Orient: Word Splitting Analysis

## Date: 2026-02-04

## Gap Analysis

### Current State (Before Fix)

- Word F1: 0.897 (gap: 0.053 to target)
- Test `test_qwen_reading_order`: FAILING

### Root Cause

1. **Primary Bug**: `element_processing.rs` merge() doesn't update `width` field
2. **Secondary Issue**: OODA-42 threshold too aggressive for width estimation errors

## First Principles Analysis

### Character Widths in PDFs

Actual character width vs. estimated:

- Font size: 60pt
- Char_width_factor: 0.55 (55% of font size)
- Estimated per-char width: 60 \* 0.55 = 33pt

Reality from trace_content:

- 'P' at 183.5 → 'u' at 210.9: gap = 27.4 (~0.46 of font size)
- 'u' at 210.9 → 's' at 236.3: gap = 25.4 (~0.42 of font size)
- Average: ~0.42-0.46, not 0.55

The 0.55 factor overestimates width by ~20%.

### Overlap Detection Threshold

OODA-42 threshold: `gap < -(avg_font_size * 0.5)`

- For font_size 60: threshold = -30
- Width overestimate can create gaps of -30 to -35 (normal merged elements)
- This incorrectly triggers space insertion

### Correct Threshold

For 20% width overestimate on 4-char element:

- Estimated width: 4 _ 60 _ 0.55 = 132
- Actual width: ~100
- Gap error: up to -32 points

New threshold should allow -50 to -60 (100% of font size) to handle this error margin.

## Risk Assessment

### Fix 1: Update width in merge()

- Risk: LOW
- Impact: Correct gap calculation in merge_line()
- Side effects: None expected

### Fix 2: Relax OODA-42 threshold

- Risk: MEDIUM
- Impact: Reduces false positive space insertions
- Side effects: Might miss some legitimate line breaks

## Recommendation

1. Apply both fixes
2. Use `gap < -avg_font_size` instead of `gap < -(avg_font_size * 0.5)`
3. Test both Qwen.pdf and arxiv paper to verify no regressions
