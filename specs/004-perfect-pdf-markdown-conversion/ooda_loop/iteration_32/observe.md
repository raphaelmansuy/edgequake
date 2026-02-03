# OODA-32 OBSERVE: Micro-Test Requirements

## Summary

Assessing requirements for creating micro-tests per mission spec.

## Current State

### Existing Test Infrastructure

| Test File                | Purpose                | Time  |
| ------------------------ | ---------------------- | ----- |
| quick_smoke.rs           | Basic extraction tests | 0.08s |
| basic_features.rs        | Feature validation     | 0.32s |
| comprehensive_quality.rs | Quality metrics        | 175s  |
| fast_quality.rs          | Fast quality checks    | ~3s   |

### Available Small PDFs

From `test-data/`:

```
001_simple_text.pdf         1.7KB  # Good for text micro-test
003_two_columns.pdf         2.0KB  # Good for column micro-test
004_simple_table_2x3.pdf    1.9KB  # Good for table micro-test
025_rotated_text.pdf        1.5KB  # Good for edge case
023_incomplete_unicode.pdf  1.6KB  # Good for font micro-test
```

### Missing Micro-Tests (per spec)

| Test File          | Status     | Target Time |
| ------------------ | ---------- | ----------- |
| micro_text.rs      | ❌ Missing | 0.02s       |
| micro_tables.rs    | ❌ Missing | 0.03s       |
| micro_columns.rs   | ❌ Missing | 0.02s       |
| micro_fonts.rs     | ❌ Missing | 0.01s       |
| micro_structure.rs | ❌ Missing | 0.02s       |

### Design Requirements (from spec)

1. Each test uses **exactly 1 minimal PDF** (< 10KB)
2. PDFs are **generated programmatically** or embedded as bytes
3. No file I/O in hot path (use `include_bytes!`)
4. Test **one assertion** per test function

## Observations

### PDFs Available for Use

The existing test-data already has suitable minimal PDFs:

- Text: `001_simple_text.pdf` (1.7KB)
- Tables: `004_simple_table_2x3.pdf` (1.9KB)
- Columns: `003_two_columns.pdf` (2.0KB)
- Fonts: `023_incomplete_unicode_mapping.pdf` (1.6KB)

Need to identify or create a PDF for structure (headers/lists).

### Structure PDF Candidates

Looking at gold/03-headers/ directory for headers test PDF.
