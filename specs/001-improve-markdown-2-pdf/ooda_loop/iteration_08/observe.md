# OODA Iteration 08 – Observe

**Date:** 2026-02-07
**Mission Re-read:** Confirmed (`specs/001-improve-markdown-2-pdf.md`)

## Observations

### 1. Test Timing Failures

All 5 `fast_quality` integration tests were failing due to timing assertions being too tight for debug builds with parallel test execution. Quality metrics were excellent:

- TPS: 98.3% and 92.6%
- Jaccard: 0.978 and 0.859

### 2. Footnote Detection Gap

- `footnote.rs` existed as a stub: detection logic implemented but not integrated into the classification pipeline.
- `BlockType` enum lacked a `Footnote` variant.
- Renderer had no footnote rendering path.
- pymupdf4llm renders footnotes as blockquotes (`> `), edgequake did not.

### 3. Current Block Classification Pipeline

```text
Code → Header(font-size) → Header(bold-only) → ListItem → Paragraph
```

Missing: Footnote step between ListItem and Paragraph.

### 4. Unit Tests: 505 passing, integration: 0 of 7 passing (timing)
