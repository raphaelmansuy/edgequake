# IT40 — Observe: Missing Spaces Between Words in Elitizon Output

## Mission Re-Read

Re-read `specs/007-improve-pdf-markdown-quality.md` at session start.

## Current State

- IT39 committed `adaad19b` — false `###` headers fixed
- 462 lib tests passing, 0 clippy warnings
- LightRAG: 57,262 bytes (reduced from 60,421 at IT37 start)

## Bug Found: Missing Spaces Between Words in Elitizon Output

Examining the Elitizon output reveals words concatenated without spaces:

```markdown
**Executivesummary**         → should be "**Executive summary**"
**AIAgentDesign &Building**  → should be "**AI Agent Design & Building**"
**ContextGraph &Powerful**   → should be "**Context Graph & Powerful**"
**Deliveryapproach**         → should be "**Delivery approach**"
**Engagementmodels**         → should be "**Engagement models**"
**Nextstep**                 → should be "**Next step**"
```

All affected words are in **bold** markdown blocks, indicating they're in the same `Span`.

## Root Cause Analysis

### The Word Boundary Detection Pipeline

1. **chars_to_spans()** in `pymupdf_grouper.rs:276`:
   - Iterates through sorted characters
   - Explicit space chars (`ch.is_whitespace()`) break spans (line 358-366)
   - For non-space chars, calls `can_append()` to check if char joins current span

2. **Span::can_append()** in `pymupdf_structs.rs:132`:
   - Returns `false` (new span) if gap > threshold
   - **Critical threshold** (line 196): `self.font_size * 0.33` (33%)

### The 33% Threshold Problem

OODA-IT32 increased the threshold from 25% to 33% with this comment:

```rust
// OODA-IT32: Increased from 0.25 to 0.33 to reduce false word boundaries.
// In monospace fonts (e.g., Inconsolatazi4 at 9pt), inter-character spacing
// can reach 28% of font size, while space width is only ~25.6%.
// Using 33% threshold avoids splitting words while still catching most gaps
```

**The problem**: This was a monospace-specific fix applied globally to ALL fonts.

| Font Type     | Inter-char Gap | Space Width | Previous (25%) | Current (33%) |
|---------------|----------------|-------------|----------------|---------------|
| Monospace     | 25-28%         | ~26%        | ❌ Splits words | ✅ OK        |
| Proportional  | 5-15%          | 20-25%      | ✅ OK          | ❌ Misses spaces |

Proportional fonts like Arial, Helvetica, and Times have:
- Inter-character gaps: 5-15% of font size (kerning)
- Word gaps: 20-25% of font size

With 33% threshold, word gaps of 20-32% are NOT detected, causing words to merge.

## Evidence: The Elitizon PDF Uses Proportional Fonts

The Elitizon document is a business presentation using proportional sans-serif fonts (likely Arial/Helvetica family). These fonts have typical word gaps around 25% of font size.

When the PDF doesn't have explicit space characters (relies on positioning), the 33% threshold fails to detect word boundaries.

## File Locations

- `can_append()` threshold: `src/layout/pymupdf_structs.rs:196`
- `chars_to_spans()`: `src/layout/pymupdf_grouper.rs:276`
- OODA-IT32 change: Same location (threshold increase)

## Quality Impact

This affects all proportional font documents where the PDF relies on positioning rather than explicit space characters for word boundaries. The Elitizon document is a clear example of this regression.
