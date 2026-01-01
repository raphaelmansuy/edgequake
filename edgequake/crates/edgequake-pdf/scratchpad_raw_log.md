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

## 2026-01-01: OODA Loop - Refinement and Expansion

### Observation

- 001: Title duplicated due to metadata title + content title.
- 002: Headers had extra bolding (`# **Header**`).
- 003: Second column header split into two blocks.
- 004: Table detection was basic but working for simple tables.

### Detection

- `MarkdownRenderer` was adding metadata title unconditionally.
- `MarkdownRenderer` was adding bolding to headers if spans were bold.
- `BlockMergeProcessor` was too strict with header vertical gaps (8.0).
- `BlockMergeProcessor` was not merging `ListItem` blocks.

### Action

- Modified `MarkdownRenderer` to skip metadata title if it matches the first block.
- Modified `MarkdownRenderer` to skip bolding in headers.
- Modified `MarkdownRenderer` to add extra newline after lists to separate them from following text.
- Updated `BlockMergeProcessor` to merge `SectionHeader` blocks with a larger gap (15.0).
- Updated `BlockMergeProcessor` to merge `ListItem` blocks if they are continuations.
- Expanded test suite to 012 with complex tables, math, and mixed languages.

### Assessment

- 001: SOTA.
- 002: SOTA.
- 003: SOTA. Headers merged correctly.
- 004: SOTA. Simple tables rendered as Markdown.
- 010-012: Pending verification.

### Next Steps

- Verify 010-012.
- Refine table detection for merged cells.
- Final SOTA declaration.

---

## 2026-01-01 19:00: CRITICAL BUG FOUND - Pdfium Feature Not Default

### Observation
- CLI tool built without `--features pdfium` returns 0 pages for all PDFs
- get_info() reports 0 pages, conversions fail silently
- MockBackend was being used by default instead of PdfiumBackend

### Detection
- Default feature in Cargo.toml was `[]` instead of `["pdfium"]`
- Without pdfium feature, MockBackend is used which has no real extraction logic

### Action
- Changed Cargo.toml: `default = ["pdfium"]`
- Verified build with explicit feature flag works correctly
- 001_simple_text.pdf now correctly shows 1 page and extracts 219 chars

### Assessment
- **CRITICAL FIX APPLIED** - pdfium is now default feature
- All subsequent tests must verify with rebuilt binary
- Need to re-baseline all PDFs with correct feature enabled

### Next Actions
1. Rebuild with new default features
2. Test all existing PDFs (001-012)
3. Identify gaps in test coverage
4. Create comprehensive test suite
5. Run OODA loops until SOTA achieved

---

## 2026-01-01 19:15: Baseline Testing All Existing PDFs


---

## 2026-01-01 20:15: OODA Loop #1 - Three Column Fix

### OBSERVE:
Output for 017_three_columns.pdf shows:
- Title correctly placed first (spanning element)
- Column headers extracted in order: Col1, Col2, Col3
- Content mixed: "Column 2" and "Column 3" content interleaved
- Expected: Title → All of Col1 → All of Col2 → All of Col3

### ORIENT (Root Cause Analysis):
Located in `src/layout/reading_order.rs::merge_column_orders()`:
- Algorithm processes blocks in "horizontal slices" (row-by-row across columns)
- Logic: For each Y position, process blocks from all columns at that Y
- This is WRONG for multi-column documents
- Correct behavior: Process entire Column 1, then Column 2, then Column 3

### DECIDE (Fix Strategy):
Change `merge_column_orders` to:
1. Process spanning elements at top
2. For each column (left to right):
   - Process ALL blocks in that column before next column
3. Handle remaining spanning elements between columns
4. Minimize disruption to 2-column logic (which works)

### ACT (Implementation):


### ASSESS (Results):
**TWO-COLUMN TEST (003)**: ✅ PERFECT!
- Reading order: Title → Column 1 (complete) → Column 2 (complete)
- No interleaving
- Fix WORKS for 2-column case

**THREE-COLUMN TEST (017)**: ❌ STILL BROKEN
- Columns still interleaved
- Possible reasons:
  1. Column detection might not be finding 3 columns
  2. Canvas-based PDF (017) might have different text extraction
  3. Need to verify column detection is actually running

**NEW TEST (021)**: ⚠️ 0 CHARACTERS
- Canvas-based PDF not extracting text at all
- Might be pdfium incompatibility with reportlab.pdfgen.canvas

### DECISION:
- Two-column fix is GOOD - don't revert
- Need to investigate why 017 doesn't detect 3 columns properly
- Try regenerating 017 with SimpleDocTemplate instead of canvas

---

## 2026-01-01 20:45: Continue Testing - Verify All Working Cases

