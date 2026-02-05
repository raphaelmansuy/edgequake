# OODA-58: Bullet Normalization

## Date: 2026-02-05 (Planned)

## Observe

PDFs use many different bullet characters.

### Current State

- Detection covers Unicode bullets (U+2022, U+25A0, etc.)
- Original bullet char preserved in output
- Markdown prefers `-` or `*`

### Issues

- Unicode bullets may not render in all markdown viewers
- Inconsistent bullet styles in output

## Orient

Normalize all bullet characters to standard markdown.

## Decide

Map detected bullets to `-` in output.

## Act

**Status:** PLANNED

Changes to make:

1. Add bullet normalization to renderer
2. Convert all recognized bullets to `-`
3. Keep numbered lists as-is (1., 2., etc.)
4. Test with various Unicode bullet PDFs

**Expected Impact:** Format 0.75 → 0.80
