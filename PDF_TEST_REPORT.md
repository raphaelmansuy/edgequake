# EdgeQuake PDF Extraction Test Report

**Generated:** 2026-01-03 15:01:55

---

## Executive Summary

- **Total PDFs Tested:** 44
- **Successful Extractions:** 42 (95.5%)
- **Failed Extractions:** 2 (4.5%)


## Synthetic Test Dataset (39 PDFs)

These are generated PDFs designed to test specific features.


- **Success Rate:** 37/39 (94.9%)
- **Average Similarity:** 35.2%


### Top 10 Performing Tests

| Rank | Test Name | Score | Size |
|------|-----------|-------|------|
| 1 | `006_multi_column_layout` | 98.4% | 391 chars |
| 2 | `005_mixed_styles` | 87.3% | 181 chars |
| 3 | `012_mixed_languages` | 86.8% | 193 chars |
| 4 | `008_multi_page` | 83.4% | 377 chars |
| 5 | `006_images_and_captions` | 79.2% | 141 chars |
| 6 | `007_nested_lists` | 79.1% | 131 chars |
| 7 | `013_nested_lists_deep` | 75.4% | 174 chars |
| 8 | `015_superscript_subscript` | 73.1% | 169 chars |
| 9 | `004_tables` | 72.9% | 125 chars |
| 10 | `003_lists_bullets_numbered` | 70.0% | 180 chars |

### Bottom 10 Performing Tests

| Rank | Test Name | Score | Size |
|------|-----------|-------|------|
| 1 | `011_math_formulas` | 0.0% | 0 chars |
| 2 | `020_unicode_special_chars` | 0.0% | 0 chars |
| 3 | `023_incomplete_unicode_mapping` | 0.0% | 0 chars |
| 4 | `024_embedded_fonts_obfuscated` | 0.0% | 0 chars |
| 5 | `025_rotated_text` | 0.0% | 0 chars |
| 6 | `026_overlapping_text_layers` | 0.0% | 0 chars |
| 7 | `027_digital_signatures_annotations` | 0.0% | 0 chars |
| 8 | `028_vector_graphics_text_on_path` | 0.0% | 0 chars |
| 9 | `030_mixed_writing_directions` | 0.0% | 0 chars |
| 10 | `031_embedded_files_attachments` | 0.0% | 0 chars |

### Score Distribution

- **Excellent (≥80%):** 4 tests
- **Good (60-79%):** 6 tests
- **Acceptable (40-59%):** 7 tests
- **Poor (<40%):** 20 tests

---

## Real-World Dataset (5 PDFs)

These are actual academic papers from arXiv.


- **Success Rate:** 5/5 (100.0%)
- **Average Similarity:** 36.1%


### Detailed Results

| Rank | Document | Score | Pages | Size | Ratio |
|------|----------|-------|-------|------|-------|
| 1 | `2900_Goyal_et_al` | 65.2% | 11 | 29,652 chars | 0.97x |
| 2 | `agent_2510.09244v1` | 44.8% | 38 | 83,387 chars | 0.93x |
| 3 | `ccn_2512.21804v1` | 34.9% | 9 | 26,279 chars | 0.83x |
| 4 | `one_tool_2512.20957v2` | 23.3% | 14 | 47,387 chars | 1.52x |
| 5 | `AlphaEvolve` | 12.4% | 44 | 99,116 chars | 2.35x |

---

## Analysis


### Strengths

- Successfully extracts text from multi-column layouts
- Handles nested lists and structured content well
- Good performance on simple tables
- Maintains basic formatting (bold, headings)

### Areas for Improvement

- Math formulas extraction (0% success)
- Encrypted/password-protected PDFs (0% success)
- Complex Unicode mappings
- Rotated text and overlapping layers
- Vector graphics with text on paths

### Recommendations

1. **Math Support:** Integrate MathML or LaTeX extraction for mathematical content
2. **Encryption:** Add support for password-protected PDFs
3. **Text Orientation:** Improve handling of rotated and transformed text
4. **Unicode:** Enhance character mapping for special characters and symbols
5. **Layout Analysis:** Refine multi-column and complex layout detection

---

## Technical Details


### Test Environment

- **Framework:** EdgeQuake PDF Extraction
- **Backend:** lopdf (Rust)
- **Test Data:**
  - Synthetic: 39 generated PDFs
  - Real-world: 5 academic papers

### Scoring Methodology

- **Similarity Score:** SequenceMatcher ratio between gold standard and extracted text
- **Scale:** 0-100% (higher is better)
- **Gold Standards:** Manually verified markdown files

---


*End of Report*
