# OODA-56: Bold Span Accuracy

## Date: 2026-02-05 (Planned)

## Observe

Format score is 0.659 (target: 0.95).

### Current State

- `Span::is_bold()` checks font name for "Bold"
- PDFium provides `font_is_bold` flag
- Flag not always used

### Issues

- Font name "ArialMT" vs "Arial-Bold" matching unreliable
- PDFium flag more accurate but not universally used

## Orient

PDFium font descriptor flags are more reliable than name matching.

## Decide

Prioritize `font_is_bold` flag over name matching.

## Act

**Status:** PLANNED

Changes to make:

1. Update `Span::is_bold()` to check `font_is_bold` first
2. Fall back to name matching if flag is None
3. Validate with test PDFs with known bold text
4. Update quality metrics

**Expected Impact:** Format 0.659 → 0.70
