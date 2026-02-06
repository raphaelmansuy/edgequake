# IT40 — Orient: Font-Aware Word Boundary Detection

## Root Cause

The OODA-IT32 threshold increase (25% → 33%) was a **monospace-specific fix applied globally**.

### Why IT32 Increased the Threshold

Monospace fonts (Courier, Inconsolata, Consolas) have:
- **Fixed character widths** — every character takes the same horizontal space
- **Inter-character gaps** can reach 25-28% of font size
- **Space width** is only ~26% of font size

With 25% threshold, normal inter-character gaps were being detected as word boundaries, splitting words like "function" into "func tion".

### Why This Broke Proportional Fonts

Proportional fonts (Arial, Helvetica, Times) have:
- **Variable character widths** — 'i' is narrow, 'm' is wide
- **Inter-character gaps** of 5-15% (tight kerning)
- **Space width** of 20-25% of font size

With 33% threshold, word gaps of 20-32% are NOT detected, merging "Executive summary" into "Executivesummary".

## The Span Already Has Monospace Information

The `Span` struct tracks `font_is_monospace: Option<bool>`:

```rust
pub struct Span {
    // ...
    pub font_is_monospace: Option<bool>,  // From RawChar.is_monospace
}
```

This comes from PDFium's `font_is_fixed_pitch()` which reads the FixedPitch bit (bit 1) from the font descriptor flags. This is **reliable** — it's how PDF viewers determine fixed-width fonts.

## Solution: Font-Aware Thresholds

Instead of one threshold for all fonts:

| Font Type     | Threshold | Rationale                                     |
|---------------|-----------|-----------------------------------------------|
| Monospace     | 33%       | Inter-char ~28%, space ~26% → need high bar   |
| Proportional  | 22%       | Inter-char ~15%, space ~25% → catch gaps >22% |

### Why 22% for Proportional?

- Kerning pairs (AV, To, We) can have gaps up to 15-18%
- Word spaces are 20-25%
- 22% threshold catches most word boundaries while avoiding false splits on kerned pairs
- This is slightly higher than the minimum space width to provide margin

## Alternative Approaches Considered

### 1. Add Intra-Span Space Detection

**Approach**: Track character positions within spans, detect large gaps, insert spaces.

**Rejected** because:
- Span doesn't store individual char positions (only bounding box)
- Would require redesigning the Span data model
- More invasive change for unclear benefit

### 2. Use Character Width for Dynamic Threshold

**Approach**: Calculate threshold based on average character width, not font size.

**Rejected** because:
- Average char width varies wildly (e.g., "iii" vs "mmm")
- Font size is a more stable metric
- Would add complexity without clear benefit

### 3. Single Threshold with Post-Processing

**Approach**: Keep 33%, add a processor to detect and fix merged words.

**Rejected** because:
- Would require dictionary lookups (slow, language-dependent)
- Can't reliably detect "Executivesummary" without knowing both words
- Treating symptoms rather than root cause

## Selected Solution

**Font-aware thresholds** in `can_append()`:
- Simple change (5 lines)
- Uses existing reliable data (`font_is_monospace`)
- Fixes root cause
- No regression for monospace fonts (keeps 33%)
- Improves proportional fonts (22%)
