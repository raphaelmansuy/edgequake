# OODA-32 DECIDE: Create 5 Micro-Test Files

## Decision

Create 5 micro-test files per mission spec, each testing one specific feature.

## Implementation Plan

### 1. tests/micro_text.rs

**Purpose:** Test basic text extraction
**PDF:** test-data/001_simple_text.pdf (1.7KB)
**Assertions:**

- Markdown contains expected text
- No crashes on parse
- Output length reasonable

### 2. tests/micro_tables.rs

**Purpose:** Test table detection
**PDF:** test-data/004_simple_table_2x3.pdf (1.9KB)
**Assertions:**

- Markdown contains `|` table markers
- Table has expected row count

### 3. tests/micro_columns.rs

**Purpose:** Test two-column layout
**PDF:** test-data/003_two_columns.pdf (2.0KB)
**Assertions:**

- Reading order is correct (left then right)
- No interleaved columns

### 4. tests/micro_fonts.rs

**Purpose:** Test font encoding edge cases
**PDF:** test-data/023_incomplete_unicode_mapping.pdf (1.6KB)
**Assertions:**

- No replacement characters (U+FFFD)
- Text is readable

### 5. tests/micro_structure.rs

**Purpose:** Test header and list detection
**PDF:** test-data/legacy/002_headers_and_lists.pdf (1.9KB)
**Assertions:**

- Markdown contains `#` headers
- Markdown contains `-` or `*` lists

## Test Template

```rust
//! Micro-test for [feature]
//! Target: <0.05s execution time

use edgequake_pdf::PdfExtractor;
use std::sync::Arc;
use edgequake_llm::providers::mock::MockProvider;

const PDF_BYTES: &[u8] = include_bytes!("../test-data/xxx.pdf");

fn create_extractor() -> PdfExtractor {
    PdfExtractor::new(Arc::new(MockProvider::new()))
}

#[test]
fn test_feature() {
    let extractor = create_extractor();
    let markdown = extractor.extract_from_bytes(PDF_BYTES).unwrap();
    assert!(!markdown.is_empty());
}
```

## Expected Results

| Test            | Target Time | Assertions |
| --------------- | ----------- | ---------- |
| micro_text      | <0.02s      | 2          |
| micro_tables    | <0.03s      | 2          |
| micro_columns   | <0.02s      | 2          |
| micro_fonts     | <0.01s      | 2          |
| micro_structure | <0.02s      | 2          |
