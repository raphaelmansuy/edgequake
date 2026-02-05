# OODA-06: Character Width Estimation and Word Gap Threshold Fixes

## Act Summary

Fixed word spacing issues in the **lopdf backend** by:

1. Correcting the character width estimation factor
2. Expanding the author name rescue zone
3. Removing an over-aggressive threshold doubling rule

**Quality Impact (lopdf backend):** Author names now properly spaced in output  
**Eval Impact:** No change to metrics (eval uses pdfium backend, not lopdf)

## Problem Statement

Author names were being merged without spaces:

```
"Zhening Huang Hyeonho JeongXuelin ChenYulia Gryaditskaya"
                       ↑     ↑        ↑
                       Missing spaces
```

This was traced to three related issues in the lopdf backend.

## Root Cause Analysis

### Issue 1: Width Estimation Factor Too High

**Location:** `element_processing.rs`, `content_parser.rs`

The character width factor was set to 0.55, meaning:

```
estimated_char_width = font_size × 0.55
```

**Empirical data from PyMuPDF:**

- "Zhening Huang" (13 chars, 12pt font): actual width = 74.7pt, ratio = 0.48
- "Yulia Gryaditskaya" (18 chars, 12pt font): actual width = 92.0pt, ratio = 0.43

The 0.55 factor overestimated widths by ~15%, causing:

- Overestimated element end positions
- Underestimated gaps between elements
- Missed word boundary detection

**Fix:** Changed from 0.55 to 0.48

### Issue 2: Author Zone Y-Threshold Too Restrictive

**Location:** `text_grouping.rs`, OODA-07 rescue logic

The rescue logic for author names had `Y < 60.0` constraint.  
Author names in 01_2512.25075v1.pdf were at Y=61.4 (just outside threshold).

**Fix:** Increased threshold from 60 to 80:

```rust
// Before
let in_author_zone = elem.y > 15.0 && elem.y < 60.0;

// After
let in_author_zone = elem.y > 15.0 && elem.y < 80.0;
```

### Issue 3: prev_has_space Threshold Doubling

**Location:** `text_grouping.rs`, merge_line function

Original logic doubled the word gap threshold if the previous element contained ANY space:

```rust
let effective_threshold = if prev_has_space {
    word_gap_threshold * 2.0  // Threshold doubled!
} else {
    word_gap_threshold
};
```

This was intended to prevent double-spacing, but caused:

- "Hyeonho Jeong" → "Xuelin Chen": gap=19.8pt, threshold=21.6pt (doubled from 10.8)
- 19.8 < 21.6, so NO space inserted → "JeongXuelin"

**Fix:** Removed the doubling logic entirely:

```rust
let effective_threshold = word_gap_threshold;
```

The `prev_ends_with_space` check already handles the double-space case properly.

## Files Changed

| File                    | Change                                            |
| ----------------------- | ------------------------------------------------- |
| `element_processing.rs` | `char_width_factor: 0.55 → 0.48`                  |
| `content_parser.rs`     | Width estimation ratio: 0.55 → 0.48 (4 locations) |
| `text_grouping.rs`      | Author zone: `Y < 60 → Y < 80`                    |
| `text_grouping.rs`      | Removed `prev_has_space` threshold doubling       |

## Verification

### Before (lopdf backend)

```
## Zhening Huang Hyeonho JeongXuelin ChenYulia Gryaditskaya
```

### After (lopdf backend)

```
## Zhening Huang Hyeonho Jeong Xuelin Chen Yulia Gryaditskaya
```

All author names now properly separated with spaces.

## Quality Metrics

**Eval uses pdfium backend, not lopdf, so metrics unchanged:**

| Metric  | Before | After | Delta  |
| ------- | ------ | ----- | ------ |
| Quality | 0.752  | 0.752 | +0.000 |
| ROUGE-L | 0.711  | 0.711 | +0.000 |
| Word F1 | 0.915  | 0.915 | +0.000 |

**Why no metric change:** The evaluation script uses `--features pdfium` which invokes `PymupdfPipeline` (pdfium-render backend), not the lopdf backend where these fixes were applied.

## Key Learnings

1. **Always trace the data path**: Understanding which backend is used for evaluation is critical
2. **Width estimation matters**: A 15% error (0.55 vs 0.48) cascades into word gap detection failures
3. **Threshold doubling is dangerous**: Well-intentioned protections (prev_has_space) can cause worse issues

## Next Steps

The pdfium backend has different issues causing low ROUGE-L:

- Image descriptions mixed into main text flow
- Figure content interleaved with body text
- Chinese characters from OCR artifacts

OODA-07 should focus on the pdfium pipeline's content filtering.
