# OODA Iteration 04 – Orient

**Date:** 2026-02-06
**Theme:** Superscript Detection via Position Analysis

## Analysis

- **Font-size ratio approach:** if `span.font_size / line.dominant_font_size < 0.7`
  AND text length < 5 characters, classify as superscript.
- Size difference is the primary signal; vertical position is secondary.
- No new enum required — add `Superscript` variant to existing `StyleType`.
- `line.dominant_font_size()` serves as the reference baseline for comparison.

## First Principles

1. Superscripts are always smaller than surrounding body text.
2. They are short — rarely more than a few characters.
3. A 70% size threshold captures most real-world cases without false positives.

## Risk

- Edge case: small-caps text could be misclassified. Mitigate with length check.

**Mission Re-read:** Confirmed.
