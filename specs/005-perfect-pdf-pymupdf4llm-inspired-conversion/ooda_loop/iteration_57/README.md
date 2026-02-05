# OODA-57: Italic Span Accuracy

## Date: 2026-02-05 (Planned)

## Observe

Similar to bold, italic detection has issues.

### Current State

- `Span::is_italic()` checks font name for "Italic", "Oblique"
- PDFium provides `font_is_italic` flag
- Flag not always used

### Issues

- Some italic fonts named "ArialMT" not detected
- PDFium flag more reliable

## Orient

Same solution as OODA-56 for italic.

## Decide

Prioritize `font_is_italic` flag over name matching.

## Act

**Status:** PLANNED

Changes to make:

1. Update `Span::is_italic()` to check `font_is_italic` first
2. Fall back to name matching if flag is None
3. Validate with test PDFs with known italic text
4. Update quality metrics

**Expected Impact:** Format 0.70 → 0.75
