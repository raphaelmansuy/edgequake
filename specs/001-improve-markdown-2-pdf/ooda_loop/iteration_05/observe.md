# OODA Iteration 05 – Observe

**Date:** 2026-02-06
**Theme:** Hyphenation Resolution Across Line Breaks

## Observations

- PDFs frequently break words across lines with a trailing hyphen.
- `pymupdf4llm` resolves these hyphens, producing clean joined words.
- Our current renderer joins lines with `\n` but does not resolve hyphens.
- Result: broken words like `computa-\ntion` instead of `computation`.
- Two hyphen types appear in PDFs:
  - **Soft hyphens** (U+00AD): inserted by layout engines, always removable.
  - **ASCII hyphens** (U+002D): ambiguous — may be real or layout-inserted.
- Hard hyphens (e.g., `self-contained`) must be preserved unchanged.

## Key Constraint

Resolution must distinguish layout hyphens from real hyphens without a dictionary.

**Mission Re-read:** Confirmed.
