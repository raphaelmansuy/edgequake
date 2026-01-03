# EdgeQuake Comprehensive PDF-to-Markdown Test Suite

## 📋 Overview

This directory contains a comprehensive test infrastructure for evaluating PDF-to-Markdown conversion fidelity through rigorous, first-principles evaluation.

**Test Data**: 105 manually-created markdown documents organized into 10 categories by complexity
**Evaluation Method**: Diff-based analysis with quantitative metrics (text preservation, formatting preservation, structural fidelity)
**Documentation**: TEST_PROTOCOL.md defines complete methodology

## 📁 Directory Structure

```
test-data/
├── gold/                          # Original markdown (single source of truth)
│   ├── 01-basics/                 # Plain text, symbols (15 docs)
│   ├── 02-formatting/             # Bold, italic, code (15 docs)
│   ├── 03-headers/                # H1-H6 hierarchy (15 docs)
│   ├── 04-lists/                  # Bullets, numbered, nested (15 docs)
│   ├── 05-tables/                 # Tabular data (15 docs)
│   ├── 06-code/                   # Code blocks (10 docs)
│   ├── 07-multi-column/           # 2-4 column layouts (10 docs)
│   ├── 08-complex/                # Real-world documents (10 docs)
│   ├── 09-edge-cases/             # Unicode, math, footnotes (5 docs)
│   └── 10-adversarial/            # Challenging cases (10 docs)
├── pdfs/                          # PDF versions (generated from gold)
├── extracted/                     # PDF→Markdown extraction results
├── diffs/                         # Diff analysis and reports
├── TEST_PROTOCOL.md               # Evaluation methodology (CRITICAL)
├── generate_pdfs.py               # Markdown → PDF converter
├── eval.sh                        # Evaluation runner
└── report.py                      # Report generator
```

## 🎯 Evaluation Phases

## Test Results Summary (2026-01-01)

**Overall Quality Score: 85/100**

### ✅ WORKING WELL (Score: 80-100)

- Basic text extraction (001 series)
- Formatted text with bold/italic (002 series)
- Two-column layouts (003)
- Simple tables (004)
- Advanced edge cases (022-031) ✅ NEW

### ⚠️ NEEDS IMPROVEMENT (Score: 40-79)

- Nested lists - indentation flattened (013)
- Complex tables - spanning issues (014, 018)
- Superscript/subscript (015)
- Code vs table detection (016)
- Footnotes (019)

### ❌ CRITICAL ISSUES (Score: 0-39)

- Three-column reading order broken (017)
- Canvas-based PDF extraction fails (021)

---

## Complete Test Catalog

### 001_simple_text.pdf ✅ SOTA

**Purpose**: Basic text extraction  
**Content**: Simple paragraphs
**Expected**: Clean paragraph separation  
**Result**: 219 chars, perfect

### 001_basic_single_column_text.pdf ✅ SOTA

**Purpose**: Single column with margins
**Content**: Title + paragraphs
**Expected**: Proper structure
**Result**: 388 chars, perfect

### 002_formatted_text_bold_italic.pdf ✅ SOTA

**Purpose**: Inline formatting
**Content**: Bold, italic, mixed
**Expected**: Markdown formatting (\*, **, \***)
**Result**: 252 chars, formatting preserved

### 002_headers_and_lists.pdf ✅ SOTA

**Purpose**: Document structure
**Content**: Headers + lists
**Expected**: Proper headings and list syntax
**Result**: 187 chars, good

### 003_lists_bullets_numbered.pdf ✅ GOOD

**Purpose**: List extraction
**Content**: Bullets and numbers
**Expected**: List syntax (-, 1., 2.)
**Result**: 226 chars, minor issues

### 003_two_columns.pdf ✅ SOTA

**Purpose**: Two-column reading order
**Content**: Two equal columns
**Expected**: Left column → right column
**Result**: 481 chars, **perfect reading order after fix**

### 004_simple_table_2x3.pdf ✅ SOTA

**Purpose**: Basic tables
**Content**: 2×4 table
**Expected**: Markdown table
**Result**: 162 chars, proper format

### 004_tables.pdf ✅ GOOD

**Purpose**: Table detection
**Content**: Simple table
**Expected**: Table syntax
**Result**: 166 chars

### 005_complex_table_merged_cells.pdf ⚠️ NEEDS WORK

**Purpose**: Advanced tables
**Content**: 6 columns, merged cells
**Expected**: Spanning preserved
**Result**: 264 chars, spanning not preserved

### 005_mixed_styles.pdf ✅ GOOD

**Purpose**: Mixed formatting
**Content**: Various styles
**Expected**: All styles preserved
**Result**: 181 chars

### 006_images_and_captions.pdf ⚠️ PARTIAL

**Purpose**: Image handling
**Content**: Images + captions
**Expected**: Placeholders + captions
**Result**: 142 chars, images not extracted yet

### 006_multi_column_layout.pdf ✅ GOOD

**Purpose**: Multi-column with headers
**Content**: Two columns
**Expected**: Proper flow
**Result**: 402 chars

### 013_nested_lists_deep.pdf ⚠️ NEEDS WORK

**Purpose**: Complex nested lists
**Content**: 3-level nesting
**Expected**: Proper indentation
**Result**: 221 chars, **indentation flattened**
**Issue**: Nested structure lost

### 014_table_spanning_cells.pdf ❌ BROKEN

**Purpose**: Cell merging
**Content**: Table with merged cells
**Expected**: Proper table structure
**Result**: 333 chars, **extracted as individual headers**
**Issue**: Table detection completely failed

### 015_superscript_subscript.pdf ⚠️ ISSUES

**Purpose**: Mathematical notation
**Content**: E=mc², H₂O, footnotes
**Expected**: Super/subscript formatting
**Result**: 192 chars, **spacing issues**
**Examples**: "H O" not "H₂O", "mc2" not "mc²"

### 016_mixed_fonts_sizes.pdf ❌ BROKEN

**Purpose**: Font variation
**Content**: Different fonts including code
**Expected**: Code blocks preserved
**Result**: 195 chars, **code detected as table!**
**Issue**: `def hello_world()` → `| def hello | _ | world() |`

### 017_three_columns.pdf ❌ CRITICAL

**Purpose**: Three-column layout
**Content**: Three equal columns
**Expected**: Col1 → Col2 → Col3
**Result**: 369 chars, **columns interleaved**
**Issue**: Reading order broken (2-col works, 3-col fails)

### 018_table_multiheader.pdf ⚠️ ISSUES

**Purpose**: Multi-level headers
**Content**: Grouped column headers
**Expected**: Header structure preserved
**Result**: 241 chars, **"110" → "1 10"**
**Issue**: Number splitting in cells

### 019_footnotes_references.pdf ⚠️ ISSUES

**Purpose**: Footnote handling
**Content**: Footnotes + references
**Expected**: Proper footnote syntax
**Result**: 486 chars, **extracted as chaotic table**

### 020_unicode_special_chars.pdf ⚠️ PARTIAL

**Purpose**: Unicode support
**Content**: Math, Greek, currencies
**Expected**: All characters preserved
**Result**: 292 chars, **some symbols → ■**
**Issue**: ¥, fractions rendered as ■

### 022_corrupted_xref_table.pdf ✅ PASSED

**Purpose**: Corrupted XRef table (broken cross-reference)
**Content**: Simple text, intentionally broken XRef
**Expected**: Extraction fails gracefully or recovers partial text
**Result**: Fails gracefully with clear error

### 023_incomplete_unicode_mapping.pdf ✅ PASSED

**Purpose**: Missing Unicode mapping for some glyphs
**Content**: Text with (cid:x) output for unmapped glyphs
**Expected**: (cid:x) for unmapped, normal for rest
**Result**: Extracted successfully

### 024_embedded_fonts_obfuscated.pdf ✅ PASSED

**Purpose**: Custom/subset/obfuscated fonts
**Content**: Text rendered with embedded, subset fonts
**Expected**: Extraction recovers text or shows gibberish if font mapping missing
**Result**: Extracted successfully

### 025_rotated_text.pdf ✅ PASSED

**Purpose**: Rotated text (arbitrary angles)
**Content**: Text at 0°, 45°, 90°, 135°, etc.
**Expected**: All text extracted regardless of rotation
**Result**: Extracted successfully

### 026_overlapping_text_layers.pdf ✅ PASSED

**Purpose**: Multiple overlapping text layers (OCR + original)
**Content**: Original, OCR, watermark overlays
**Expected**: Avoid duplicate text, handle overlays
**Result**: Extracted successfully

### 027_digital_signatures_annotations.pdf ✅ PASSED

**Purpose**: Digital signatures, comments, highlights, sticky notes
**Content**: Text with annotations and signature
**Expected**: Ignore non-text annotations, skip signature fields
**Result**: Extracted successfully

### 028_vector_graphics_text_on_path.pdf ✅ PASSED

**Purpose**: Text in vector graphics or on path
**Content**: Text on curve, inside shapes, SVG-like
**Expected**: Extract text if possible, skip if unsupported
**Result**: Extracted successfully

### 029_encrypted_password_protected.pdf ✅ PASSED

**Purpose**: Password-protected/encrypted PDF
**Content**: Simple text, password required
**Expected**: Extraction fails with clear error
**Result**: Fails with "PDF is encrypted" error

### 030_mixed_writing_directions.pdf ✅ PASSED

**Purpose**: Mixed LTR and RTL text (e.g., English + Arabic/Hebrew)
**Content**: Paragraphs in both scripts
**Expected**: Preserve order and directionality
**Result**: Extracted successfully

### 031_embedded_files_attachments.pdf ✅ PASSED

**Purpose**: Embedded files/attachments (PDF/A, Excel, images)
**Content**: Text plus embedded files
**Expected**: Ignore attachments, may log presence
**Result**: Extracted successfully

---

## Test Commands

```bash
# Convert PDF to Markdown
cargo run --release --bin edgequake-pdf -- convert \\
  -i test-data/001_simple_text.pdf \\
  -o output/001.md

# Get PDF info
cargo run --release --bin edgequake-pdf -- info \\
  -i test-data/001_simple_text.pdf

# Run all tests
cd test-data && ./test_all.sh

# Test with page numbers
cargo run --release --bin edgequake-pdf -- convert \\
  -i test-data/008_multi_page_5_pages.pdf \\
  --page-numbers
```

---

## Priority Fixes for SOTA

### CRITICAL (Blocking SOTA)

1. **Three-column reading order** (017)

   - 2-column works after recent fix
   - 3-column still interleaves content
   - Root cause: Column detection or merging

2. **Complex table detection** (014)
   - Tables with spanning cells break
   - Extracted as individual paragraphs
   - Needs better table heuristics

### HIGH (Major Issues)

3. **Code block detection** (016)

   - Monospace text detected as table
   - Need font-based discrimination

4. **Number splitting** (018)
   - "110" becomes "1 10" in tables
   - Word boundary logic too aggressive

### MEDIUM (Quality Improvements)

5. **Nested list indentation** (013)
6. **Superscript/subscript** (015)
7. **Footnote layout** (019)
8. **Unicode glyph handling** (020)

---

## Known Issues & Workarounds

**Issue: Three columns interleaved**

- Priority: CRITICAL
- Test: 017
- Status: 2-column fixed, 3-column broken
- Workaround: Use 2-column layouts

**Issue: Code detected as table**

- Priority: HIGH
- Test: 016
- Workaround: None

**Issue: Number splitting "1 10"**

- Priority: MEDIUM
- Test: 018
- Workaround: None

**Issue: Canvas PDFs extract 0 chars**

- Priority: MEDIUM
- Test: 021
- Workaround: Use SimpleDocTemplate

---

## OODA Loop Process

For each failing test:

1. **OBSERVE**: Run conversion, examine output
2. **ORIENT**: Identify root cause
3. **DECIDE**: Plan fix
4. **ACT**: Implement
5. **ASSESS**: Verify + check regressions

Log iterations in `scratchpad_raw_log.md`.

---

## Contributing

New tests should:

1. Follow naming: `NNN_description.pdf`
2. Document purpose/expected output
3. Update this README
4. Run full suite before commit
5. Add to `test-data/ISSUES_FOUND.md` if exposing new issues

---

🔧 Quick commands

- Generate PDFs (fallback generator using ReportLab):

  ```bash
  python3 generate_simple_pdfs.py
  ```

- Run comprehensive Rust evaluation (ignored by default, run with ignored tests):

  ```bash
  cargo test --package edgequake-pdf --test comprehensive_evaluation -- --ignored --nocapture
  ```

6. Consider automation scripts for repetitive tasks

---

## References

- Issues: `test-data/ISSUES_FOUND.md`
- Dev log: `scratchpad_raw_log.md`
- Examples: `../examples/`

---

## Real-world Dataset

### ccn_2512.21804v1.pdf ✅ SOTA

**Purpose**: Real-world research paper validation
**Content**: 9-page paper with multi-column layout, figures, tables, and references.
**Expected**: High-quality markdown extraction suitable for RAG.
**Result**: 27,062 characters. Excellent preservation of structure, headers, and reading order.
