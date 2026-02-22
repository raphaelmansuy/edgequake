# Testing Expansion Plan

> **OODA Loop - Act**: Comprehensive test coverage expansion to validate improvements and prevent regressions.

**Current Test Coverage**: 72%  
**Target Test Coverage**: 90%+  
**Current Test Count**: 239 tests  
**Target Test Count**: 400+ tests

---

## Executive Summary

| Test Category              | Current   | Target     | Gap  | Priority |
| -------------------------- | --------- | ---------- | ---- | -------- |
| **Unit Tests**             | 189 tests | 280 tests  | +91  | P0       |
| **Integration Tests**      | 40 tests  | 80 tests   | +40  | P0       |
| **Performance Benchmarks** | 10 tests  | 40 tests   | +30  | P1       |
| **Edge Case Coverage**     | 60%       | 95%        | +35% | P0       |
| **Fuzzing Tests**          | 0 tests   | Continuous | New  | P1       |

**Critical Gaps**:

1. Font encoding edge cases (15+ missing tests)
2. Large document stress tests (100+ page PDFs)
3. Corrupted PDF handling (malformed content streams)
4. Concurrent access safety
5. Memory leak detection

---

## 1. Unit Test Expansion

### 1.1 Font Encoding Tests (Current: 15 tests → Target: 35 tests)

**Missing Coverage**:

```rust
// tests/font_encodings_comprehensive.rs

#[cfg(test)]
mod font_encoding_tests {
    use super::*;

    #[test]
    fn test_gb2312_chinese_simple() {
        let encoding = Encoding::GB2312;
        let bytes = vec![0xB0, 0xA1];  // '啊'
        let result = encoding.decode(&bytes);
        assert_eq!(result, "啊");
    }

    #[test]
    fn test_shift_jis_japanese_kanji() {
        let encoding = Encoding::ShiftJIS;
        let bytes = vec![0x93, 0xFA];  // '日'
        let result = encoding.decode(&bytes);
        assert_eq!(result, "日");
    }

    #[test]
    fn test_big5_traditional_chinese() {
        let encoding = Encoding::Big5;
        let bytes = vec![0xA4, 0xA4];  // '中'
        let result = encoding.decode(&bytes);
        assert_eq!(result, "中");
    }

    #[test]
    fn test_arabic_isolated_form() {
        let encoding = Encoding::Arabic;
        let bytes = vec![0x06, 0x21];  // 'ء'
        let result = encoding.decode(&bytes);
        assert_eq!(result, "ء");
    }

    #[test]
    fn test_arabic_contextual_forms() {
        // Test initial, medial, final forms
        let forms = vec![
            (vec![0x06, 0x28], "ب"),  // Isolated
            (vec![0xFE, 0x90], "ب"),  // Initial
            (vec![0xFE, 0x92], "ب"),  // Medial
            (vec![0xFE, 0x91], "ب"),  // Final
        ];

        for (bytes, expected) in forms {
            let result = Encoding::Arabic.decode(&bytes);
            assert_eq!(result, expected);
        }
    }

    #[test]
    fn test_identity_encoding_surrogate_pairs() {
        // Unicode surrogate pairs (U+10000 - U+10FFFF)
        let bytes = vec![0xD8, 0x00, 0xDC, 0x00];  // U+10000
        let result = Encoding::Identity.decode(&bytes);
        assert_eq!(result, "𐀀");
    }

    #[test]
    fn test_encoding_invalid_byte_sequence() {
        // Incomplete multi-byte sequence
        let bytes = vec![0xB0];  // GB2312 requires 2 bytes
        let result = Encoding::GB2312.decode(&bytes);
        assert_eq!(result, "�");  // Replacement character
    }

    #[test]
    fn test_mixed_encoding_document() {
        // Document with Latin + CJK
        let doc = create_test_pdf_with_mixed_fonts();
        let extractor = PdfExtractor::new();
        let result = extractor.extract_document(&doc).unwrap();

        assert!(result.text.contains("Hello"));
        assert!(result.text.contains("你好"));
    }

    #[test]
    fn test_symbol_font_mapping() {
        // Symbol font (Zapf Dingbats, Symbol)
        let encoding = Encoding::Symbol;
        let bytes = vec![0x61];  // Maps to α (alpha)
        let result = encoding.decode(&bytes);
        assert_eq!(result, "α");
    }

    #[test]
    fn test_custom_encoding_from_differences() {
        // PDF with /Differences array
        let font_dict = create_font_with_differences();
        let encoding = FontInfo::get_encoding(&font_dict);

        // Test remapped character
        let result = encoding.decode(&[0x41]);  // 'A' remapped
        assert_eq!(result, "★");
    }

    #[test]
    fn test_encoding_normalization() {
        // Test Unicode normalization (NFC vs NFD)
        let bytes_nfc = vec![0xC3, 0xA9];  // é (NFC)
        let bytes_nfd = vec![0x65, 0xCC, 0x81];  // e + combining acute (NFD)

        let result_nfc = decode_and_normalize(&bytes_nfc);
        let result_nfd = decode_and_normalize(&bytes_nfd);

        assert_eq!(result_nfc, result_nfd);  // Should normalize to same form
    }
}
```

**New Tests to Add** (20 tests):

- [ ] GB2312 encoding (Chinese simplified)
- [ ] Shift-JIS encoding (Japanese)
- [ ] Big5 encoding (Chinese traditional)
- [ ] Arabic encoding with contextual forms
- [ ] Hebrew encoding (right-to-left)
- [ ] Symbol font mappings
- [ ] Zapf Dingbats font
- [ ] Custom /Differences array
- [ ] Surrogate pair handling
- [ ] Invalid byte sequence handling
- [ ] Mixed encoding documents
- [ ] Unicode normalization (NFC/NFD)
- [ ] CID-keyed font mapping
- [ ] Type0 composite fonts
- [ ] TrueType font with custom encoding

---

### 1.2 Math Formula Tests (New: 25 tests)

```rust
// tests/math_formula_extraction.rs

#[cfg(test)]
mod formula_tests {
    #[test]
    fn test_simple_equation() {
        let pdf = create_pdf_with_formula("E = mc^2");
        let doc = extract_with_formula_detection(&pdf);

        assert_eq!(doc.formulas[0].latex, r"E = mc^2");
    }

    #[test]
    fn test_integral_with_limits() {
        let pdf = create_pdf_with_formula("∫₀¹ f(x)dx");
        let doc = extract_with_formula_detection(&pdf);

        assert_eq!(doc.formulas[0].latex, r"\int_0^1 f(x)dx");
    }

    #[test]
    fn test_fraction() {
        // Test horizontal bar detection
        let pdf = create_pdf_with_fraction("x+y", "z");
        let doc = extract_with_formula_detection(&pdf);

        assert_eq!(doc.formulas[0].latex, r"\frac{x+y}{z}");
    }

    #[test]
    fn test_summation_notation() {
        let pdf = create_pdf_with_formula("∑ₖ₌₁ⁿ k²");
        let doc = extract_with_formula_detection(&pdf);

        assert_eq!(doc.formulas[0].latex, r"\sum_{k=1}^n k^2");
    }

    #[test]
    fn test_greek_letters() {
        let pdf = create_pdf_with_formula("α + β = γ");
        let doc = extract_with_formula_detection(&pdf);

        assert_eq!(doc.formulas[0].latex, r"\alpha + \beta = \gamma");
    }

    #[test]
    fn test_matrix_notation() {
        let pdf = create_pdf_with_matrix(&[
            [1, 0],
            [0, 1],
        ]);
        let doc = extract_with_formula_detection(&pdf);

        assert_eq!(doc.formulas[0].latex,
            r"\begin{pmatrix} 1 & 0 \\ 0 & 1 \end{pmatrix}");
    }

    #[test]
    fn test_nested_superscripts() {
        let pdf = create_pdf_with_formula("e^(x^2)");
        let doc = extract_with_formula_detection(&pdf);

        assert_eq!(doc.formulas[0].latex, r"e^{x^2}");
    }

    #[test]
    fn test_formula_confidence_scoring() {
        let pdf = create_pdf_with_formula("x + 123");
        let doc = extract_with_formula_detection(&pdf);

        // Low confidence (mostly numbers)
        assert!(doc.formulas[0].confidence < 0.5);
    }
}
```

**Formula Test Categories** (25 tests total):

- [ ] Basic equations (5 tests)
- [ ] Integrals and limits (4 tests)
- [ ] Fractions and divisions (3 tests)
- [ ] Greek letters (4 tests)
- [ ] Matrix notation (2 tests)
- [ ] Nested structures (3 tests)
- [ ] Multi-line equations (2 tests)
- [ ] Confidence scoring (2 tests)

---

### 1.3 Table Detection Tests (Current: 12 → Target: 30 tests)

```rust
// tests/table_detection_comprehensive.rs

#[test]
fn test_merged_cell_horizontal() {
    let table_html = r#"
        <table>
            <tr><th colspan="2">Header</th><th>C</th></tr>
            <tr><td>1</td><td>2</td><td>3</td></tr>
        </table>
    "#;

    let pdf = create_pdf_from_html(table_html);
    let doc = extract_with_tables(&pdf);

    let table = &doc.tables[0];
    assert_eq!(table.rows[0].cells[0].col_span, 2);
}

#[test]
fn test_merged_cell_vertical() {
    // Test rowspan detection
    let pdf = create_table_with_rowspan(3);
    let doc = extract_with_tables(&pdf);

    assert_eq!(doc.tables[0].rows[0].cells[0].row_span, 3);
}

#[test]
fn test_nested_table() {
    // Table within table cell
    let pdf = create_nested_table_pdf();
    let doc = extract_with_tables(&pdf);

    assert_eq!(doc.tables.len(), 2);
    assert!(doc.tables[1].bbox.is_inside(&doc.tables[0].bbox));
}

#[test]
fn test_headerless_table() {
    // Whitespace-aligned columns
    let text = r#"
        Name       Age    City
        Alice      25     NYC
        Bob        30     LA
    "#;

    let pdf = create_pdf_from_text(text);
    let doc = extract_with_tables(&pdf);

    assert_eq!(doc.tables.len(), 1);
    assert_eq!(doc.tables[0].cols, 3);
}

#[test]
fn test_rotated_table() {
    // 90-degree rotated table
    let pdf = create_rotated_table_pdf(90.0);
    let doc = extract_with_tables(&pdf);

    assert_eq!(doc.tables.len(), 1);
    // Verify column order after rotation
}

#[test]
fn test_table_with_images() {
    // Table cells containing images
    let pdf = create_table_with_embedded_images();
    let doc = extract_with_tables(&pdf);

    assert!(doc.tables[0].rows[0].cells[0].contains_image);
}
```

**New Table Tests** (18 tests):

- [ ] Merged cells (horizontal/vertical)
- [ ] Nested tables
- [ ] Headerless tables
- [ ] Rotated tables
- [ ] Tables with images
- [ ] Multi-page spanning tables
- [ ] Irregular cell sizes
- [ ] Tables with borders only on some sides
- [ ] Gradient/colored backgrounds
- [ ] Tables with footnotes

---

## 2. Integration Tests

### 2.1 Real-World Document Tests (New: 40 tests)

```rust
// tests/integration/real_world_documents.rs

#[test]
fn test_arxiv_paper_extraction() {
    // Test with real arXiv paper (e.g., 2301.00001.pdf)
    let pdf_bytes = download_arxiv_paper("2301.00001");
    let extractor = PdfExtractor::new();
    let doc = extractor.extract_document(&pdf_bytes).unwrap();

    // Validate structure
    assert!(doc.pages.len() >= 8);
    assert!(doc.tables.len() >= 2);
    assert!(doc.formulas.len() >= 10);

    // Validate content quality
    let char_accuracy = calculate_char_accuracy(&doc.text, &gold_standard);
    assert!(char_accuracy > 0.98);
}

#[test]
fn test_financial_report_pdf() {
    // SEC 10-K filing
    let pdf = load_test_file("sec_10k_sample.pdf");
    let doc = extract_with_tables(&pdf);

    // Validate financial tables
    assert!(doc.tables.len() >= 5);
    assert!(doc.tables[0].rows.len() > 10);
}

#[test]
fn test_scanned_document_ocr() {
    let pdf = load_test_file("scanned_receipt.pdf");
    let doc = extract_with_ocr(&pdf, OcrConfig::default()).await;

    assert!(doc.pages[0].stats.text_confidence > 0.8);
    assert!(doc.text.contains("Total"));
}

#[test]
fn test_multilingual_document() {
    // Document with English, Chinese, Arabic
    let pdf = load_test_file("multilingual_manual.pdf");
    let doc = extract_with_encoding_detection(&pdf);

    assert!(doc.text.contains("Hello"));
    assert!(doc.text.contains("你好"));
    assert!(doc.text.contains("مرحبا"));
}

#[test]
fn test_large_document_100_pages() {
    let pdf = load_test_file("large_thesis.pdf");
    let start = Instant::now();

    let doc = extract_with_progress(&pdf, |progress| {
        println!("Progress: {:.1}%", progress * 100.0);
    });

    let duration = start.elapsed();
    assert!(doc.pages.len() == 100);
    assert!(duration.as_secs() < 60);  // Must complete in 1 minute
}
```

**Real-World Test Coverage** (40 tests):

- [ ] Academic papers (arXiv, IEEE) - 10 tests
- [ ] Financial reports (10-K, earnings) - 5 tests
- [ ] Legal documents (contracts, briefs) - 5 tests
- [ ] Technical manuals - 5 tests
- [ ] Scanned documents - 5 tests
- [ ] Multilingual documents - 5 tests
- [ ] Large documents (50-200 pages) - 5 tests

---

### 2.2 Error Handling Tests (New: 15 tests)

```rust
// tests/integration/error_handling.rs

#[test]
fn test_corrupted_pdf_header() {
    let mut pdf_bytes = load_test_file("valid.pdf");
    pdf_bytes[0..4].copy_from_slice(b"JUNK");  // Corrupt %PDF header

    let result = extractor.extract_document(&pdf_bytes);
    assert!(result.is_err());

    match result.unwrap_err() {
        PdfError::InvalidHeader { .. } => {},
        _ => panic!("Expected InvalidHeader error"),
    }
}

#[test]
fn test_missing_page_tree() {
    let pdf = create_pdf_without_pages();
    let result = extractor.extract_document(&pdf);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("page tree"));
}

#[test]
fn test_infinite_loop_detection() {
    // Circular reference in page tree
    let pdf = create_pdf_with_circular_refs();

    let result = extractor.extract_document(&pdf);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("circular"));
}

#[test]
fn test_encrypted_pdf_without_password() {
    let pdf = load_test_file("encrypted_no_password.pdf");
    let result = extractor.extract_document(&pdf);

    assert!(result.is_err());
    match result.unwrap_err() {
        PdfError::Encrypted { .. } => {},
        _ => panic!("Expected Encrypted error"),
    }
}

#[test]
fn test_malformed_content_stream() {
    let pdf = create_pdf_with_malformed_stream();
    let result = extractor.extract_document(&pdf);

    // Should recover gracefully
    assert!(result.is_ok());
    assert!(result.unwrap().errors.len() > 0);
}

#[test]
fn test_memory_limit_exceeded() {
    let config = ExtractionConfig {
        max_memory_mb: 100,
        ..Default::default()
    };

    let pdf = create_huge_pdf(1000);  // 1GB+ decompressed
    let result = extractor.extract_with_config(&pdf, config);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("memory limit"));
}
```

**Error Handling Tests** (15 tests):

- [ ] Corrupted file headers
- [ ] Missing critical objects
- [ ] Circular references
- [ ] Encrypted PDFs
- [ ] Malformed streams
- [ ] Invalid encodings
- [ ] Memory limit exceeded
- [ ] Timeout exceeded
- [ ] Missing fonts
- [ ] Unsupported PDF versions

---

## 3. Performance Benchmarks

### 3.1 Benchmark Suite Design

```rust
// benches/extraction_benchmarks.rs

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

fn bench_extraction_by_page_count(c: &mut Criterion) {
    let mut group = c.benchmark_group("extraction_scaling");

    for page_count in [1, 5, 10, 20, 50, 100] {
        let pdf = create_test_pdf_with_pages(page_count);

        group.bench_with_input(
            BenchmarkId::new("sequential", page_count),
            &pdf,
            |b, pdf| {
                b.iter(|| {
                    let extractor = PdfExtractor::new();
                    extractor.extract_document(black_box(pdf))
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("parallel", page_count),
            &pdf,
            |b, pdf| {
                b.iter(|| {
                    let extractor = PdfExtractor::with_parallelism(4);
                    extractor.extract_document(black_box(pdf))
                })
            },
        );
    }

    group.finish();
}

fn bench_table_detection_complexity(c: &mut Criterion) {
    let mut group = c.benchmark_group("table_detection");

    for cell_count in [10, 50, 100, 500, 1000] {
        let pdf = create_pdf_with_table(cell_count);

        group.bench_with_input(
            BenchmarkId::new("lattice_o_n2", cell_count),
            &pdf,
            |b, pdf| {
                b.iter(|| {
                    let engine = LatticeEngine::new();
                    engine.detect_tables_naive(black_box(pdf))
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("lattice_rtree", cell_count),
            &pdf,
            |b, pdf| {
                b.iter(|| {
                    let engine = LatticeEngine::with_rtree();
                    engine.detect_tables_optimized(black_box(pdf))
                })
            },
        );
    }

    group.finish();
}

fn bench_memory_usage(c: &mut Criterion) {
    c.bench_function("memory_per_page", |b| {
        let pdf = load_test_file("sample_100_pages.pdf");

        b.iter(|| {
            let initial = get_memory_usage();
            let doc = extract_document(black_box(&pdf));
            let final_mem = get_memory_usage();

            let mem_per_page = (final_mem - initial) / 100;
            assert!(mem_per_page < 2_000_000);  // <2MB/page
        });
    });
}

criterion_group!(
    benches,
    bench_extraction_by_page_count,
    bench_table_detection_complexity,
    bench_memory_usage,
);
criterion_main!(benches);
```

**Benchmark Categories** (30 benchmarks):

- [ ] Page extraction scaling (1-100 pages)
- [ ] Table detection complexity (10-1000 cells)
- [ ] Font encoding performance
- [ ] Memory usage per page
- [ ] Parallel vs sequential comparison
- [ ] Cold start vs warm cache
- [ ] Different document types (text-heavy, table-heavy, image-heavy)

---

## 4. Fuzzing Tests

### 4.1 cargo-fuzz Setup

```toml
# Cargo.toml
[dependencies]
# ... existing dependencies

[dev-dependencies]
arbitrary = "1.3"

[[bin]]
name = "fuzz_pdf_parsing"
path = "fuzz/fuzz_targets/pdf_parsing.rs"
```

```rust
// fuzz/fuzz_targets/pdf_parsing.rs

#![no_main]
use libfuzzer_sys::fuzz_target;
use edgequake_pdf::PdfExtractor;

fuzz_target!(|data: &[u8]| {
    let extractor = PdfExtractor::new();

    // Should never panic
    let _ = extractor.extract_document(data);
});
```

**Fuzzing Strategy**:

1. **Structure-Aware Fuzzing**: Use grammar-based fuzzing for PDF structure
2. **Differential Fuzzing**: Compare output with other PDF libraries
3. **Sanitizer Integration**: Run with AddressSanitizer, MemorySanitizer
4. **Continuous Fuzzing**: 24/7 on CI infrastructure

**Expected Outcomes**:

- Discover edge cases in font encodings
- Find memory safety issues
- Identify infinite loop conditions
- Validate error handling

---

## 5. Property-Based Testing

### 5.1 QuickCheck/Proptest Integration

```rust
// tests/property_based.rs

use proptest::prelude::*;

proptest! {
    #[test]
    fn prop_bounding_box_contains_text(
        text in "\\PC{1,100}",
        x in 0.0f32..1000.0,
        y in 0.0f32..1000.0,
    ) {
        let bbox = BoundingBox::new(x, y, x + 100.0, y + 20.0);
        let block = Block {
            text: text.clone(),
            bbox,
            ..Default::default()
        };

        // Property: All characters should be within bbox
        for span in &block.spans {
            prop_assert!(bbox.contains(&span.bbox));
        }
    }

    #[test]
    fn prop_merge_blocks_preserves_text(
        blocks in prop::collection::vec(any::<Block>(), 1..10)
    ) {
        let original_text: String = blocks.iter()
            .map(|b| b.text.as_str())
            .collect();

        let merged = merge_blocks(&blocks);

        // Property: Merging shouldn't lose text
        prop_assert!(merged.text.len() >= original_text.len() - blocks.len());
    }

    #[test]
    fn prop_table_grid_consistency(
        rows in 2usize..20,
        cols in 2usize..10,
    ) {
        let table = create_random_table(rows, cols);

        // Property: Grid dimensions match declared dimensions
        prop_assert_eq!(table.grid.len(), rows);
        prop_assert!(table.grid.iter().all(|row| row.len() == cols));
    }
}
```

**Properties to Test** (20 properties):

- [ ] Bounding box containment
- [ ] Text preservation after merge
- [ ] Table grid consistency
- [ ] Font encoding round-trip
- [ ] Block ordering preservation
- [ ] Page boundary validation

---

## 6. Test Infrastructure

### 6.1 Golden File Testing Framework

```rust
// tests/golden_files.rs

use std::fs;

#[test]
fn golden_test_arxiv_sample() {
    let pdf = load_test_file("golden/arxiv_sample.pdf");
    let gold_md = fs::read_to_string("golden/arxiv_sample.md").unwrap();

    let extractor = PdfExtractor::new();
    let doc = extractor.extract_document(&pdf).unwrap();
    let output_md = doc.to_markdown();

    if output_md != gold_md {
        // Write actual output for debugging
        fs::write("golden/arxiv_sample.actual.md", &output_md).unwrap();

        // Show diff
        let diff = diff_strings(&gold_md, &output_md);
        panic!("Output differs from golden file:\n{}", diff);
    }
}
```

**Golden File Coverage**:

- [ ] 20 representative documents with gold standard outputs
- [ ] Automatic regeneration on intentional changes
- [ ] Diff visualization on failure

---

## 7. Continuous Integration Tests

### 7.1 CI Pipeline Configuration

```yaml
# .github/workflows/pdf_tests.yml

name: PDF Tests

on: [push, pull_request]

jobs:
  unit-tests:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        rust: [stable, nightly]

    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: ${{ matrix.rust }}

      - name: Run unit tests
        run: cargo test --package edgequake-pdf --lib

      - name: Run integration tests
        run: cargo test --package edgequake-pdf --test '*'

  benchmarks:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Run benchmarks
        run: cargo bench --package edgequake-pdf

      - name: Check performance regression
        run: |
          # Compare with baseline
          python scripts/check_regression.py

  fuzzing:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Install cargo-fuzz
        run: cargo install cargo-fuzz

      - name: Run fuzzing (5 minutes)
        run: cargo fuzz run fuzz_pdf_parsing -- -max_total_time=300
```

---

## 8. Test Metrics Dashboard

### 8.1 Tracking Over Time

```rust
// scripts/test_metrics.rs

pub struct TestMetrics {
    pub total_tests: usize,
    pub unit_tests: usize,
    pub integration_tests: usize,
    pub coverage_percent: f32,
    pub avg_test_duration_ms: u64,
    pub flaky_tests: Vec<String>,
}

impl TestMetrics {
    pub fn collect() -> Self {
        // Run tests with coverage
        let output = Command::new("cargo")
            .args(&["tarpaulin", "--out", "Json"])
            .output()
            .unwrap();

        let coverage: Coverage = serde_json::from_slice(&output.stdout).unwrap();

        Self {
            total_tests: coverage.test_count,
            coverage_percent: coverage.coverage,
            // ... parse other metrics
        }
    }

    pub fn save_to_history(&self) {
        let path = format!("metrics/tests_{}.json", Utc::now().format("%Y%m%d"));
        fs::write(path, serde_json::to_string_pretty(self).unwrap()).unwrap();
    }
}
```

**Tracked Metrics**:

- Test count over time
- Coverage percentage trends
- Performance regression detection
- Flaky test identification
- CI build duration

---

## Summary

| Phase                       | Duration | Tests Added  | Coverage Gain |
| --------------------------- | -------- | ------------ | ------------- |
| **Phase 1: Unit Tests**     | 2 weeks  | +91 tests    | +10%          |
| **Phase 2: Integration**    | 2 weeks  | +40 tests    | +5%           |
| **Phase 3: Benchmarks**     | 1 week   | +30 tests    | N/A           |
| **Phase 4: Infrastructure** | 1 week   | Fuzzing + CI | +3%           |
| **Total**                   | 6 weeks  | +161 tests   | +18% coverage |

**Final State**: 400+ tests, 90%+ coverage, continuous fuzzing, golden file validation

---

## Next Document

[IMPLEMENTATION_PRIORITIES.md](IMPLEMENTATION_PRIORITIES.md) - Ranked action items with ROI analysis and phased timeline.
