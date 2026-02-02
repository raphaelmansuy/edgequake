# OODA Iteration 02 - Orient

## Analysis of Flipped Coordinate Detection Issue

### The Coordinate System Problem

PDFs can use two coordinate systems:
1. **Standard**: Origin at bottom-left, Y increases upward
2. **Flipped**: Origin at top-left, Y increases downward (negative CTM d-component)

### Current Code Flow (BROKEN)

```
1. Parse all text elements → original Y coordinates
2. OCR layer detection → filter based on Y distribution
3. AFTER FILTERING → detect if coordinates are flipped
4. Normalize Y coordinates
```

**Problem**: Flip detection at step 3 uses filtered Y range, which is much smaller.

### Why the Logic Failed

| Stage | Y min | Y max | Span | > 1.5×792? | Flip? |
|-------|-------|-------|------|------------|-------|
| Original | 265.6 | 2452.5 | 2186.9 | YES | ✅ |
| Filtered | 1700.2 | 2452.5 | 752.2 | NO | ❌ |

The filtered range (752.2) doesn't exceed 1.5× page height (1188), so flip wasn't detected.

### Correct Solution

Move flip detection BEFORE OCR filtering:
1. Parse all text elements → original Y coordinates
2. **DETECT FLIP HERE** → using original Y range (2186.9 > 1188 → flipped)
3. OCR layer detection → filter based on Y distribution
4. Normalize Y coordinates → use is_flipped from step 2

### Code Location
- File: `edgequake/crates/edgequake-pdf/src/extraction/extraction_engine.rs`
- Function: `extract_page_content`
- Lines: ~255-395
