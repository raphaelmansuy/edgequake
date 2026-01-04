# Quality Gaps Analysis

> **OODA Loop - Decide**: Systematic identification of missing features and quality improvements needed for production-grade PDF extraction.

**Current Quality Score**: 88/100  
**Target Quality Score**: 95/100  
**Gap**: 7 points across 5 categories

---

## Executive Summary

| Category                 | Current | Target | Gap  | Priority |
| ------------------------ | ------- | ------ | ---- | -------- |
| **Text Extraction**      | 95/100  | 99/100 | -4%  | P0       |
| **Math Formula Support** | 40/100  | 90/100 | -50% | P0       |
| **Table Quality**        | 85/100  | 95/100 | -10% | P1       |
| **OCR Integration**      | 0/100   | 85/100 | -85% | P0       |
| **Error Recovery**       | 60/100  | 90/100 | -30% | P1       |

**Key Missing Features**:

1. Scanned PDF support (OCR)
2. Math formula/equation handling
3. Merged table cell detection
4. CJK/Arabic font support
5. Progress callbacks
6. Incremental rendering

---

## 1. OCR Integration (Critical Gap)

### 1.1 Current Limitation

**Problem**: Scanned PDFs extract as empty or garbled

```rust
// Current behavior
let pdf = load_scanned_pdf("scanned_paper.pdf");
let doc = extractor.extract_document(&pdf)?;

// Result: Empty document or meaningless symbols
assert_eq!(doc.pages[0].blocks.len(), 0);  // No text extracted
```

**Impact**: ~30% of academic papers are scanned or hybrid (text + scanned images)

---

### 1.2 Proposed OCR Pipeline

```rust
pub struct OcrConfig {
    pub engine: OcrEngine,  // Tesseract, Azure Vision, AWS Textract
    pub language: String,   // "eng", "eng+fra", etc.
    pub dpi: u32,          // Resolution for rendering
    pub confidence_threshold: f32,  // Minimum confidence to accept
}

pub enum OcrEngine {
    Tesseract,
    AzureVision { api_key: String },
    AwsTextract { credentials: AwsCredentials },
}

impl PdfExtractor {
    pub async fn extract_with_ocr(&self, pdf_bytes: &[u8], config: OcrConfig)
        -> Result<Document> {

        let doc = self.extract_document(pdf_bytes)?;

        // Detect pages with low text confidence
        for (idx, page) in doc.pages.iter().enumerate() {
            if page.stats.text_confidence < 0.5 {
                // Render page to image
                let image = self.render_page_to_image(pdf_bytes, idx, config.dpi)?;

                // Run OCR
                let ocr_text = self.run_ocr(&image, &config).await?;

                // Merge OCR results with native text
                page = self.merge_ocr_results(page, ocr_text)?;
            }
        }

        Ok(doc)
    }

    async fn run_ocr(&self, image: &Image, config: &OcrConfig) -> Result<OcrResult> {
        match config.engine {
            OcrEngine::Tesseract => {
                tesseract::ocr_image(image, &config.language)
            }
            OcrEngine::AzureVision { api_key } => {
                azure_vision::analyze_image(image, api_key).await
            }
            OcrEngine::AwsTextract { credentials } => {
                aws_textract::detect_text(image, credentials).await
            }
        }
    }
}
```

---

### 1.3 Implementation Plan

**Week 1-2**: Tesseract Integration

- [ ] Add `tesseract-sys` Rust bindings
- [ ] Implement page rendering (PDF → PNG via `image` crate)
- [ ] Add OCR text extraction
- [ ] Add confidence scoring
- [ ] Test with scanned documents

**Week 3**: Cloud OCR Providers

- [ ] Add Azure Vision API client
- [ ] Add AWS Textract client
- [ ] Add rate limiting and retry logic
- [ ] Add cost estimation

**Week 4**: OCR Result Merging

- [ ] Detect hybrid pages (native + scanned)
- [ ] Merge OCR bounding boxes with native text
- [ ] Resolve conflicts (overlapping regions)
- [ ] Validate with test dataset

**Expected Quality Improvement**: +40 points (scanned PDFs go from 0% to 85% accuracy)

---

## 2. Math Formula Support (Critical Gap)

### 2.1 Current Behavior

**Problem**: Formulas extract with lost structure

```
Input PDF:     E = mc²
                   ∫₀¹ f(x)dx
                   xᵢ + yⱼ = ∑ₖ zₖ

Current Output: E = mc2
                 ∫01 f(x)dx  (subscripts lost)
                 xi + yj = Σk zk  (broken)

Expected:      $E = mc^2$
                $\int_0^1 f(x)dx$
                $x_i + y_j = \sum_k z_k$
```

**Root Cause**: No special handling for:

1. Subscript/superscript positioning
2. Symbol recognition (∫, ∑, ∂)
3. Fraction bars (horizontal lines)
4. Equation delimiters

---

### 2.2 Formula Detection Algorithm

```rust
pub struct FormulaDetector {
    symbol_patterns: HashMap<char, String>,  // Unicode → LaTeX
}

impl FormulaDetector {
    pub fn detect_formulas(&self, page: &Page) -> Vec<Formula> {
        let mut formulas = Vec::new();

        for block in &page.blocks {
            // Check for math symbols
            let math_density = self.count_math_symbols(&block.text)
                / block.text.len() as f32;

            if math_density > 0.15 {  // 15% math symbols
                let formula = self.reconstruct_formula(block)?;
                formulas.push(formula);
            }
        }

        formulas
    }

    fn count_math_symbols(&self, text: &str) -> usize {
        text.chars().filter(|c| {
            matches!(c,
                '∫' | '∑' | '∏' | '√' | '∂' | '∞' | '≠' | '≤' | '≥' |
                'α' | 'β' | 'γ' | 'δ' | 'ε' | 'π' | 'σ' | 'θ' | 'λ' | 'μ'
            )
        }).count()
    }

    fn reconstruct_formula(&self, block: &Block) -> Result<Formula> {
        let mut latex = String::new();

        // Analyze element positioning to detect super/subscripts
        for span in &block.spans {
            let y_offset = span.bbox.y1 - block.bbox.y1;

            if y_offset < -2.0 {
                // Superscript (above baseline)
                latex.push_str(&format!("^{{{}}}", span.text));
            } else if y_offset > 2.0 {
                // Subscript (below baseline)
                latex.push_str(&format!("_{{{}}}", span.text));
            } else {
                // Regular text
                latex.push_str(&span.text);
            }
        }

        // Convert Unicode symbols to LaTeX
        for (unicode, tex) in &self.symbol_patterns {
            latex = latex.replace(*unicode, tex);
        }

        Ok(Formula {
            latex,
            bbox: block.bbox.clone(),
            confidence: self.calculate_confidence(&latex),
        })
    }
}
```

---

### 2.3 Symbol Mapping Table

```rust
fn build_symbol_map() -> HashMap<char, &'static str> {
    hashmap! {
        // Greek letters
        'α' => r"\alpha",
        'β' => r"\beta",
        'γ' => r"\gamma",
        'Δ' => r"\Delta",
        'π' => r"\pi",
        'σ' => r"\sigma",

        // Math operators
        '∫' => r"\int",
        '∑' => r"\sum",
        '∏' => r"\prod",
        '√' => r"\sqrt",
        '∂' => r"\partial",

        // Relations
        '≠' => r"\neq",
        '≤' => r"\leq",
        '≥' => r"\geq",
        '≈' => r"\approx",
        '∞' => r"\infty",

        // Arrows
        '→' => r"\rightarrow",
        '←' => r"\leftarrow",
        '↔' => r"\leftrightarrow",
    }
}
```

---

### 2.4 Implementation Checklist

- [ ] Implement `FormulaDetector` with symbol density check
- [ ] Add Y-offset analysis for super/subscripts
- [ ] Create comprehensive Unicode → LaTeX mapping
- [ ] Add fraction bar detection (horizontal line analysis)
- [ ] Implement equation delimiter detection
- [ ] Add confidence scoring
- [ ] Test with arXiv papers dataset (10k+ formulas)
- [ ] Validate output with LaTeX compiler

**Estimated Effort**: 2 weeks  
**Expected Improvement**: +30 points (40% → 90% formula accuracy)

---

## 3. Table Quality Improvements

### 3.1 Merged Cell Detection

**Problem**: Merged cells not detected

```
Current Output:
| A | B | C |
|---|---|---|
| 1 | 2 | 3 |

Expected (with merged header):
| A   | B | C |  (A spans 2 columns)
|-----|---|---|
| 1 | 2 | 3 |
```

**Solution**: Detect cell spans in lattice engine

```rust
impl LatticeEngine {
    fn detect_merged_cells(&self, grid: &Grid) -> Vec<MergedCell> {
        let mut merged = Vec::new();

        for row in 0..grid.rows {
            for col in 0..grid.cols {
                let cell = &grid.cells[row][col];

                // Check horizontal merge (no vertical line between cells)
                let mut span = 1;
                for next_col in (col + 1)..grid.cols {
                    if !self.has_vertical_line_between(col, next_col, row) {
                        span += 1;
                    } else {
                        break;
                    }
                }

                if span > 1 {
                    merged.push(MergedCell {
                        row,
                        col,
                        row_span: 1,
                        col_span: span,
                    });
                }
            }
        }

        merged
    }
}
```

**Effort**: 5 days  
**Impact**: +10 points table accuracy

---

### 3.2 Headerless Table Detection

**Current**: Requires box borders  
**Improvement**: Detect aligned whitespace-separated tables

```rust
fn detect_text_tables(&self, blocks: &[Block]) -> Vec<Table> {
    // Group blocks by Y-position (rows)
    let rows = self.group_into_rows(blocks);

    // For each row, detect column alignment
    let col_positions = self.detect_column_positions(&rows);

    // If 3+ aligned columns across 3+ rows → table
    if col_positions.len() >= 3 && rows.len() >= 3 {
        return self.construct_table(rows, col_positions);
    }

    vec![]
}
```

**Effort**: 7 days  
**Impact**: +5 points (handles 20% more tables)

---

## 4. CJK and Arabic Font Support

### 4.1 Current Limitation

**Problem**: CJK characters extract as "□" or garbled

```rust
// Chinese PDF
Input:  你好世界
Output: □□□□  (missing glyphs)

// Arabic PDF
Input:  مرحبا
Output: ????? (encoding error)
```

**Root Cause**: Missing font encoding tables for non-Latin scripts

---

### 4.2 Solution: Extended Encoding Tables

```rust
// Add to encodings.rs

pub const GB2312_ENCODING: &[(u16, char)] = &[
    (0xB0A1, '啊'),
    (0xB0A2, '阿'),
    // ... 6,763 more mappings
];

pub const SHIFT_JIS_ENCODING: &[(u16, char)] = &[
    (0x8140, '　'),  // Ideographic space
    (0x8141, '、'),
    // ... 7,000+ more mappings
];

pub const ARABIC_ENCODING: &[(u16, char)] = &[
    (0x0621, 'ء'),
    (0x0622, 'آ'),
    // ... 200+ more mappings
];

impl FontInfo {
    fn get_encoding(&self, doc: &LopdfDocument, font_dict: &Dictionary) -> Encoding {
        // ... existing logic

        // Add CJK detection
        if let Some(encoding_name) = self.detect_cjk_encoding(font_dict) {
            return match encoding_name {
                "GB2312" => Encoding::GB2312,
                "ShiftJIS" => Encoding::ShiftJIS,
                "Big5" => Encoding::Big5,
                _ => Encoding::Identity,
            };
        }

        // Add Arabic detection
        if self.is_arabic_font(font_dict) {
            return Encoding::Arabic;
        }

        // ... fallback
    }
}
```

**Effort**: 2 weeks (encoding table creation + testing)  
**Impact**: Enables international document support

---

## 5. Error Recovery Improvements

### 5.1 Current Behavior

**Problem**: Single error aborts entire extraction

```rust
// Current: One bad page → entire document fails
let doc = extractor.extract_document(&pdf_bytes)?;  // ❌ Error on page 5

// Result: Pages 1-4 and 6-N lost
```

---

### 5.2 Graceful Degradation

```rust
pub struct ExtractionResult {
    pub document: Document,
    pub errors: Vec<PageError>,
    pub warnings: Vec<String>,
}

pub struct PageError {
    pub page: usize,
    pub error: PdfError,
    pub recoverable: bool,
}

impl PdfExtractor {
    pub fn extract_with_recovery(&self, pdf_bytes: &[u8])
        -> Result<ExtractionResult> {

        let mut pages = Vec::new();
        let mut errors = Vec::new();

        for (page_num, page_id) in page_ids.iter().enumerate() {
            match self.backend.extract_page(&lopdf_doc, *page_id, page_num) {
                Ok(page) => pages.push(page),
                Err(e) if e.is_recoverable() => {
                    // Add placeholder page
                    pages.push(Page::placeholder(page_num));
                    errors.push(PageError {
                        page: page_num,
                        error: e,
                        recoverable: true,
                    });
                }
                Err(e) => {
                    errors.push(PageError {
                        page: page_num,
                        error: e,
                        recoverable: false,
                    });
                    // Continue with remaining pages
                }
            }
        }

        Ok(ExtractionResult {
            document: Document { pages, .. },
            errors,
            warnings: vec![],
        })
    }
}
```

**Effort**: 3 days  
**Impact**: +20 points reliability (90% of errors are recoverable)

---

## 6. Missing Features Summary

| Feature                          | Status         | Priority | Effort  | Impact  |
| -------------------------------- | -------------- | -------- | ------- | ------- |
| **OCR Integration**              | ❌ Not started | P0       | 4 weeks | +40 pts |
| **Math Formula Support**         | ❌ Not started | P0       | 2 weeks | +30 pts |
| **Merged Cell Detection**        | ❌ Not started | P1       | 5 days  | +10 pts |
| **CJK/Arabic Support**           | ❌ Not started | P1       | 2 weeks | Enabler |
| **Error Recovery**               | ⚠️ Partial     | P1       | 3 days  | +20 pts |
| **Progress Callbacks**           | ❌ Not started | P2       | 2 days  | UX      |
| **Incremental Rendering**        | ❌ Not started | P2       | 1 week  | UX      |
| **Equation Delimiter Detection** | ❌ Not started | P2       | 3 days  | +5 pts  |
| **Headerless Table Detection**   | ⚠️ Partial     | P2       | 7 days  | +5 pts  |

**Total Effort**: ~12 weeks (3 months)  
**Quality Improvement**: 88/100 → 95+/100 (7+ point gain)

---

## Next Document

[TESTING_EXPANSION.md](TESTING_EXPANSION.md) - Test coverage improvements and quality validation strategies.
