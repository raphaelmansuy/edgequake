# OODA-24 Act: Superscript Digit Filtering

## Implementation Summary

### Changes Made

**layout_processing.rs** - Added superscript digit filter in `is_margin_content()`:

```rust
// OODA-24: Filter standalone single digits that are likely superscripts
// WHY: Author affiliation markers (¹, ², ³) are rendered as separate text elements
// at superscript positions. They appear as standalone "1", "2", "3" blocks.
// Detection: Single digit, very small bbox (superscript size), not at page edges
// Heuristic: bbox height < 8pt indicates superscript positioning
if text.len() == 1 && text.chars().all(|c| c.is_ascii_digit()) && bbox_height < 8.0 {
    return true; // Filter as margin content
}
```

### Problem Solved

Before:

```
5:
6: 2
7:
8: 2
9:
10: Zhaoxi ZhangYitong DuanYanzhi Zhang
```

After:

```
5:
6: Zhaoxi ZhangYitong DuanYanzhi Zhang
```

### Root Cause

arXiv papers use superscript numbers (¹, ², ³) for author affiliations. The PDF renders these as separate text elements with:

- Small font size (~7pt)
- Small bounding box height (~5pt)
- Positioned at superscript baseline

The extraction correctly captures these as separate blocks, but they should be filtered as noise.

### First Principles

Superscripts in academic papers serve as visual markers linking authors to affiliations. In markdown, these should be:

1. Either merged with author names (hard: requires position matching)
2. Or filtered out (simple: they don't add semantic value in plain text)

Chose option 2 because:

- Superscripts are typically explained in footnotes
- The affiliation list appears separately
- Filtering is robust and simple

### Validation

- 415 lib tests pass
- Word count reduced by 19 words (filtered superscripts)
- Output cleaner without standalone digits
