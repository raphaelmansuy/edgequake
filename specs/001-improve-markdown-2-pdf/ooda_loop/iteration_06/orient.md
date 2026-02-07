# OODA Iteration 06 – Orient

**Date:** 2026-02-06
**Theme:** Bold-Only Header Detection for Academic Papers

## Analysis

- Bold alone is not enough — many PDFs bold entire paragraphs or emphasis spans.
- Pattern alone is not enough — body text may contain Roman numerals or numbers.
- **First Principles:** bold + recognized pattern = high-confidence header signal.
- Requiring BOTH conditions eliminates false positives from either signal alone.
- Pattern categories ranked by specificity:
  1. Roman numeral prefix (`I.`, `IV.`) — very high specificity.
  2. Numeric section prefix (`3.`, `3.1`) — high specificity.
  3. All-caps single line (`REFERENCES`) — medium specificity, needs bold gate.
- Subsection numbering (e.g., `3.1`) maps naturally to H3; top-level to H2.
- All-caps keywords without numbering map to H2 (section-level dividers).

## Hypothesis

Re-enabling pattern detection behind a bold-all gate will capture academic headers with zero regressions on non-academic PDFs.

**Mission Re-read:** Confirmed.
