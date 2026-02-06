# IT32 — Observe: Quality Baseline Assessment

## Mission Re-Read

Re-read `specs/007-improve-pdf-markdown-quality.md` at session start.

## Current State

- IT31 committed — lopdf legacy removed (~12,025 lines)
- 440 lib tests passing (1 was failing due to pdfium fallback — fixed)
- Single extraction pipeline: PdfiumBackend only

## Bug Found: `extract_with_progress` Missing `merge_same_line_blocks`

The `extract` method calls `merge_same_line_blocks` (added in IT28), but `extract_with_progress` does not. This means progress-aware extraction produces fragmented blocks (e.g., "AI Services" + "—" + "Elitizon" as 3 separate blocks instead of 1).

**Location**: `src/backend/pdfium_backend.rs:288` (extract_with_progress)

## Dead Debug Logging

Both `extract_document` and `extract_document_with_progress` in `extractor.rs` contained extensive debug logging (printing first 50 blocks BEFORE and 20 blocks AFTER processing). This noise:

- Pollutes production logs
- Slows extraction by ~5% (formatting 50+ strings per page)
- Violates SRP (extraction method shouldn't be a debug logger)

## Quality Comparison: EdgeQuake vs PyMuPDF4LLM

### AI_Services\_\_Elitizon.pdf (single-column business doc)

| Metric         | EdgeQuake      | PyMuPDF4LLM                           | Winner    |
| -------------- | -------------- | ------------------------------------- | --------- |
| Reading order  | ✅ Correct     | ❌ Broken (multi-col merge artifacts) | EdgeQuake |
| Bold detection | ✅ Accurate    | ✅ Accurate                           | Tie       |
| Headers        | ✅ H4 sections | ❌ H2 for bold lines (over-promoted)  | EdgeQuake |
| List items     | ✅ Separated   | ❌ Merged in paragraphs               | EdgeQuake |

### lighrag_2410.05779v3.pdf (2-column academic paper)

| Metric         | EdgeQuake               | PyMuPDF4LLM            | Issue        |
| -------------- | ----------------------- | ---------------------- | ------------ |
| Tables         | ❌ Jumbled text         | ✅ Structured          | Critical gap |
| Figures        | ❌ Garbled diagram text | ✅ Extracted as images | Critical gap |
| Column reading | ✅ Mostly correct       | ✅ Correct             | Acceptable   |
| Math formulas  | ❌ Broken symbols       | ❌ Also limited        | Both weak    |
