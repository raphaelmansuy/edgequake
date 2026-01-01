# EdgeQuake PDF Test Suite

## Overview
This directory contains test PDF documents with increasing complexity levels to validate the EdgeQuake PDF to Markdown conversion tool.

## Test Documents

### Level 1: Basic Text
- **001_basic_single_column_text.pdf**
  - Single column, plain text only
  - No formatting or special elements
  - Expected output: Clean paragraphs in Markdown

### Level 2: Formatted Text
- **002_formatted_text_bold_italic.pdf**
  - Headings (H1, H2)
  - Bold and italic inline formatting
  - Expected output: Proper Markdown formatting (* for italic, ** for bold)

- **003_lists_bullets_numbered.pdf**
  - Unordered bullet lists
  - Ordered numbered lists
  - Expected output: Proper list syntax (-, 1., 2., etc.)

### Level 3: Structure
- **006_multi_column_layout.pdf**
  - Two-column layout
  - Tests column detection and reading order
  - Expected output: Left column first, then right column

### Level 4: Tables
- **004_simple_table_2x3.pdf**
  - Simple 2-column table with header
  - 3 data rows
  - Expected output: Markdown table syntax with alignment

- **005_complex_table_merged_cells.pdf**
  - Multi-column table (6 columns)
  - Multiple rows with formatting
  - Total row with special styling
  - Expected output: Complete table in Markdown

### Level 6: Mixed Content
- **007_mixed_content_complex.pdf**
  - Realistic mix: text + lists + tables
  - Multiple sections with headings
  - Tests integration of all features
  - Expected output: Well-structured Markdown with all elements

- **008_multi_page_5_pages.pdf**
  - 5-page document
  - Tests page boundary handling
  - Tests --page-numbers flag
  - Expected output: Continuous or page-delimited Markdown

## Testing Protocol (ODAA Loop)

For each test document:

1. **OBSERVE**: Examine the input PDF (use `info` command)
2. **ORIENT**: Define expected Markdown output
3. **DECIDE**: Run conversion, identify issues
4. **ACT**: Fix code if needed
5. **ASSESS**: Verify improvement, iterate

## Test Commands

```bash
# Get PDF info
cargo run --bin edgequake-pdf -- info -i test-data/001_basic_single_column_text.pdf

# Convert to markdown (default output)
cargo run --bin edgequake-pdf -- convert -i test-data/001_basic_single_column_text.pdf

# Convert with custom output
cargo run --bin edgequake-pdf -- convert -i test-data/001_basic_single_column_text.pdf -o output/001.md

# Convert with page numbers
cargo run --bin edgequake-pdf -- convert -i test-data/008_multi_page_5_pages.pdf --page-numbers

# Convert first 3 pages only
cargo run --bin edgequake-pdf -- convert -i test-data/008_multi_page_5_pages.pdf --max-pages 3

# Vision mode (if/when implemented with real LLM)
cargo run --bin edgequake-pdf -- convert -i test-data/007_mixed_content_complex.pdf --vision
```

## Success Criteria (SOTA)

A SOTA PDF converter should:

✅ Extract text accurately (100% for clean PDFs)
✅ Preserve document structure (headings, paragraphs)
✅ Detect and format lists correctly
✅ Handle tables (simple and complex)
✅ Maintain reading order (single/multi-column)
✅ Process multi-page documents efficiently
✅ Generate clean, valid Markdown
✅ Handle edge cases gracefully

## Current Status

See `scratchpad_raw_log.md` for detailed test results and iteration log.
