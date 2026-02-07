# OODA Iteration 04 – Observe

**Date:** 2026-02-06
**Theme:** Superscript Detection via Position Analysis

## Observations

- PDFium does not expose superscript/subscript flags on text spans.
- Font size and vertical position are the only reliable signals available.
- `pymupdf4llm` uses `flags & 1` to detect superscript, but we lack equivalent flags.
- We need heuristic-based detection instead of flag-based detection.
- Common superscript patterns in PDFs: footnote markers `[1]`, `[*]`, `[†]`.
- Superscript spans are typically shorter text (1-4 chars) with smaller font size.
- Baseline offset relative to the line is a secondary signal.

## Key Constraint

No native API support — detection must be purely heuristic.

**Mission Re-read:** Confirmed.
