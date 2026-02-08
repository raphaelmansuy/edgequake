# OODA Iteration 05 – Orient

**Date:** 2026-02-06
**Theme:** Hyphenation Resolution Across Line Breaks

## Analysis

- **Soft hyphens (U+00AD):** always resolve — remove hyphen, join fragments.
- **ASCII hyphens (U+002D):** resolve only when next line starts lowercase.
- Hard hyphens preserved: next line starts uppercase, digit, or list marker.
- Processing belongs at the renderer level, inside `render_lines_inline()`,
  after individual lines are rendered but before final join.

## First Principles

1. Soft hyphens exist solely for layout — always safe to remove.
2. Lowercase continuation is a reliable mid-word signal without a dictionary.
3. Conservative resolution avoids false positives on compound words.

## Risk

- Rare edge case: proper nouns hyphenated at line end. Acceptable trade-off.

**Mission Re-read:** Confirmed.
