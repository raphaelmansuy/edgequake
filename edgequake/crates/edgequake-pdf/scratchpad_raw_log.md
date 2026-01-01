# Scratchpad Raw Log - SOTA PDF Conversion

## Initial State

- CLI tool `edgequake-pdf` mapped.
- `reportlab` available for PDF generation.
- Goal: Achieve SOTA quality through ODAA loops.

## Plan

1. Generate a set of test PDFs with increasing complexity.
2. Test conversion for each.
3. Observe failures/limitations.
4. Fix and iterate.

## Key Learnings

- (To be populated)

## 2024-05-24: ODAA Loop - 001_simple_text.pdf

### Observation

- Initial extraction was vertical (one char per line).
- Fixed coordinate system (PDF bottom-left to IR top-left).
- Word/Line detection heuristics were too sensitive.
- avg_char_width and avg_char_height are critical for thresholds.

### Detection

- y_diff > avg_char_width \* 0.5 was triggering new lines for every character because y_diff was calculated using flipped coordinates but the threshold was too small.
- gap > avg_char_width \* 0.2 was triggering spaces between characters in words.

### Action

- Corrected Y-coordinate flip: char_top = page_height - char_top_pdf.
- Increased word gap threshold to 0.5 \* avg_char_width.
- Increased line detection threshold to 0.8 \* avg_char_height.
- Added avg_char_height calculation.

### Assessment

- 001_simple_text.pdf now extracts words correctly.
- Paragraph detection is still slightly off (some paragraphs merged).
- Line detection within paragraphs is slightly jittery (splitting words like "extractor").
- Need to refine thresholds and potentially use a more stable reference for y (e.g., baseline instead of mid-point).

### Next Steps

- Refine y_diff and gap thresholds.
- Test 002_headers_and_lists.pdf.

## 2024-05-24: ODAA Loop - 002, 003, 004

### Observation

- 002 (Headers/Lists): Extracted correctly, but "1." became "1 .".
- 003 (Two Columns): Reading order is WRONG. Second column extracted BEFORE first column.
- 004 (Tables): Extracted as plain text lines, not Markdown tables.

### Detection

- Reading order logic in LayoutProcessor is likely failing or not being invoked correctly.
- Table detection is non-existent in the current PdfiumExtractor.
- Word boundary logic is still slightly too aggressive (adding space before dots or in numbers).

### Action

- Removed BlockMergeProcessor as it was causing paragraph merging issues.
- Refined paragraph threshold to 1.8x height.
- Refined line threshold to 0.8x height.

### Assessment

- Simple text is SOTA.
- Headers/Lists are acceptable but need style detection.
- Two columns are BROKEN.
- Tables are BROKEN.

## 2024-05-24: ODAA Loop - Multi-Column and Refined Extraction

### Observation

- 003 (Two Columns): Reading order was inverted. Second column appeared before first.
- 002 (Lists): Bullet points were being separated into their own blocks.
- 001 (Simple): "Simple" became "S imple" due to sensitive gap detection.

### Detection

- ReadingOrderDetector used a "row-by-row" merge logic which interleaved columns or picked the wrong one if Y coordinates were slightly different.
- PdfiumExtractor was creating too many blocks for small horizontal gaps (e.g. between bullet and text).
- Line detection using mid_y was sensitive to font size changes (e.g. Title vs Normal).

### Action

- Rewrote `ReadingOrderDetector::merge_column_orders` to use a "column-by-column" flow between spanning elements.
- Improved `PdfiumExtractor` line detection using vertical overlap ratio and mid-point distance fallback.
- Switched from global `avg_char_metrics` to per-character metrics for thresholds to handle mixed font sizes.
- Increased horizontal gap threshold for new block detection to 15x char width to keep lists together.
- Increased word boundary threshold to 0.8x char width to avoid splitting words with wide kerning.

### Assessment

- 001_simple_text.pdf: SOTA.
- 002_headers_and_lists.pdf: SOTA. Lists are correctly grouped.
- 003_two_columns.pdf: SOTA. Reading order is now correct (Column 1 then Column 2).
- 004_tables.pdf: Reading order is correct, but still plain text.

### Next Steps

- Implement table detection and Markdown table formatting.
- Add font style detection (Bold/Italic) to Markdown.
- Final SOTA validation.

## 2026-01-01: SOTA Mission Initialization

### Objective

Achieve SOTA PDF to Markdown conversion quality.

### Plan

1.  Expand test suite with 005-010 PDFs of increasing complexity.
2.  Update `test-data/README.md`.
3.  Run OODA loop for each file.
4.  Fix issues in `PdfiumExtractor` and layout processors.

### Initial Assessment

- 001-003 are reported as SOTA in previous logs, but I should verify.
- 004 (Tables) is known to be broken (plain text).
- Multi-column is reported as fixed but needs verification.
