# ORIENT.md - Iteration 003

## Root-cause hypotheses

### 1) Inline styles lost at line merge

`SotaBackend::merge_line()` produces a single merged string and a single style summary per line. Downstream, each `Block` is created with exactly one `TextSpan`, so the Markdown renderer cannot emit accurate `**bold**` / `*italic*` spans within a line.

### 2) Header level detection is bypassed

`HeaderDetectionProcessor` only processes `BlockType::Text`. If the backend (or a prior processor) already set a block to `SectionHeader`, its level will not be corrected by the header detector.

Numeric section headers like `1. Introduction` frequently do not have a strong font-size signal and need explicit pattern detection.

### 3) Table reconstruction is too permissive

`TextTableReconstructionProcessor` selects forward/backward “table-like” lines near a caption, but:

- the scan window can drift into unrelated prose,
- header selection defaults to the first captured line, which is often a table note, not the header row,
- collapsed single-line tables need special parsing (common in arXiv PDFs).

