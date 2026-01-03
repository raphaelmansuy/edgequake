# PDF-to-Markdown Conversion Test Protocol

## Executive Summary

This document defines a rigorous, first-principles evaluation protocol for testing the PDF-to-Markdown converter. The protocol is designed to:

- Identify where the converter excels and struggles
- Quantify loss of information and fidelity
- Drive continuous improvement through empirical evidence
- Enable regression testing across updates

## Methodology: First Principles

We evaluate the converter against a **gold standard**—the original markdown source document—to answer:

**"When we convert Markdown → PDF → Markdown, how much information/fidelity is lost?"**

This captures the practical reality: users create content in markdown, we convert to PDF for distribution, then need to extract back to markdown for editing.

## Test Data Organization

### Folder Structure

```
test-data/
├── gold/                    # Original markdown documents (single source of truth)
│   ├── 01-basics/
│   ├── 02-formatting/
│   ├── 03-lists-tables/
│   ├── 04-multi-column/
│   ├── 05-complex-layouts/
│   ├── 06-edge-cases/
│   └── 07-adversarial/
├── pdfs/                    # PDF versions (generated from gold markdown)
│   ├── 01-basics/
│   ├── 02-formatting/
│   └── ... (mirrors gold structure)
├── extracted/               # PDF→Markdown extraction results
│   ├── 01-basics/
│   ├── 02-formatting/
│   └── ... (mirrors gold structure)
├── diffs/                   # Diff analysis between gold and extracted
│   └── report.json
└── metrics.json             # Quantitative evaluation results
```

### Document Categories (120 total gold files)

1. **Basics (9 docs)**: Plain text and simple structures

   - Line breaks and spacing (3)
   - Blockquotes and horizontal rules (2)
   - Numbers, symbols, punctuation (3)
   - URLs and dates (1)

2. **Formatting (15 docs)**: Typography and styling

   - Bold text (3)
   - Italic text (3)
   - Bold-italic combinations (3)
   - Strikethrough (3)
   - Mixed bold/italic/normal (3)

3. **Headers (15 docs)**: Title and heading hierarchy

   - Single H1 (3)
   - H1→H6 hierarchy (3)
   - Mixed header placement (3)
   - Headers with formatting (3)
   - Deep nesting (3)

4. **Lists (15 docs)**: Bullet and numbered lists

   - Simple bullet lists (3)
   - Simple numbered lists (3)
   - Nested lists (3)
   - Mixed bullet/numbered (3)
   - Lists with formatting (3)

5. **Tables (15 docs)**: Tabular data

   - Simple 2×3 tables (3)
   - Wide tables (4+ cols) (3)
   - Tall tables (6+ rows) (3)
   - Complex headers (3)
   - Tables with formatting (3)

6. **Code (10 docs)**: Code blocks and inline code

   - Inline code (2)
   - Simple code blocks (2)
   - Multi-language blocks (2)
   - Long code blocks (2)
   - Code with special chars (2)

7. **Multi-Column (10 docs)**: Advanced layouts

   - 2-column text (3)
   - 3-column text (3)
   - Mixed column layouts (2)
   - Column breaks (2)

8. **Complex (10 docs)**: Real-world documents

   - Academic papers (2)
   - Technical specs (2)
   - Reports with sections (2)
   - Mixed complex layouts (4)

9. **Edge Cases (5 docs)**: Boundary conditions

   - Unicode handling (1)
   - Math formulas (1)
   - Footnotes (1)
   - Rotated text (1)
   - Overlapping elements (1)

10. **Adversarial (10 docs)**: Challenging PDFs
    - Corrupted structures (2)
    - Embedded fonts (2)
    - Digital signatures (2)
    - Password-protected (1)
    - Complex vector graphics (3)

**Total: 120 gold markdown files across 10 categories**

- Mixed single and multi (2)
- 4+ columns (2)

8. **Complex Layouts (10 docs)**: Real-world documents

   - Academic papers (2)
   - Technical reports (2)
   - Newsletter-style (2)
   - Magazine-style (2)
   - Mixed everything (2)

9. **Edge Cases (5 docs)**: Challenging scenarios

   - Unicode and RTL text (1)
   - Special math symbols (1)
   - Footnotes/references (1)
   - Overlapping content (1)
   - Rotated text (1)

10. **Adversarial (10 docs)**: Tests designed to fail
    - Intentionally ambiguous layouts (2)
    - Scanned PDFs (lossy) (2)
    - Encrypted PDFs (2)
    - Corrupted PDFs (2)
    - Very large documents (2)

## Test Execution Protocol

### Phase 1: Setup (One-time)

1. Create all 105 markdown documents (manually, intentionally)
2. Generate PDF files from markdown using a stable PDF generator
3. Extract metadata about each document:
   - Word count, line count, char count
   - Formatting density (% text formatted)
   - Table complexity score
   - Column layout type
   - Known difficult elements

### Phase 2: Extraction

1. Run PDF→Markdown extraction for each document
2. Store extracted markdown in `extracted/` with same directory structure
3. Capture extraction time and memory usage
4. Log any errors or warnings

### Phase 3: Evaluation

For each document, compute:

#### 3.1 Textual Diff Analysis

- Line-by-line diff between gold and extracted
- Count additions, deletions, modifications
- Calculate edit distance (Levenshtein)
- Identify diff patterns (formatting loss, spacing, etc.)

#### 3.2 Structural Analysis

- Count headers, lists, tables, code blocks in both
- Detect missing or spurious structures
- Measure structural fidelity

#### 3.3 Metrics Calculation

For each document:

```
text_preservation = (preserved_chars / total_chars) × 100
formatting_preservation = (preserved_formatting_elements / total_elements) × 100
structural_fidelity = (matched_structures / gold_structures) × 100
compression_ratio = (extracted_size / gold_size) × 100
```

For categories:

```
category_score = (sum of document scores) / (count of documents)
```

Overall:

```
overall_score = weighted_average(categories)
```

## Diff Interpretation Rules

### Loss Categories

1. **Whitespace Loss**

   - Extra blank lines removed
   - Indentation normalized
   - Multiple spaces collapsed
   - _Impact_: Minor (typically ignorable)

2. **Formatting Loss**

   - Bold markers (**text**) → plain text
   - Italic markers (_text_) → plain text
   - Strikethrough (~~text~~) → plain text
   - _Impact_: Medium (preserves meaning, loses style)

3. **Structure Loss**

   - Table columns misaligned
   - Lists flattened or broken
   - Headers demoted/promoted
   - _Impact_: High (changes meaning/readability)

4. **Content Loss**

   - Words missing or truncated
   - Lines completely absent
   - Columns lost in multi-column layouts
   - _Impact_: Critical (information loss)

5. **Content Hallucination**
   - Extra words not in original
   - Invented structure
   - Spurious formatting
   - _Impact_: Critical (false information)

## Success Criteria

### Per Document

- **Excellent**: ≥95% text preservation, ≥90% formatting, ≥95% structure
- **Good**: ≥90% text preservation, ≥80% formatting, ≥90% structure
- **Acceptable**: ≥85% text preservation, ≥70% formatting, ≥85% structure
- **Poor**: <85% text preservation or <70% formatting or <85% structure

### Per Category

- **Excellent**: ≥95% average across documents
- **Good**: ≥90% average
- **Acceptable**: ≥85% average
- **Needs Work**: <85% average

### Overall (All 105 documents)

- **Excellent**: ≥93% overall score
- **Good**: ≥88% overall score
- **Acceptable**: ≥83% overall score
- **Needs Improvement**: <83% overall score

## Regression Testing

After code changes to the converter:

1. Re-run full extraction on all 105 documents
2. Compute diffs against previous evaluation
3. Identify categories with score changes
4. Flag any regressions (score decrease >2%)
5. Celebrate improvements (score increase >3%)

## Reporting

For each evaluation run, generate:

1. **metrics.json**: Quantitative data

   ```json
   {
     "timestamp": "2025-01-03T14:30:00Z",
     "overall_score": 87.3,
     "categories": {
       "01-basics": 94.2,
       "02-formatting": 88.1,
       ...
     },
     "document_scores": {
       "01-basics/001_plain_text.md": {
         "text_preservation": 99.8,
         "formatting_preservation": 0,
         "structural_fidelity": 100,
         "status": "excellent"
       },
       ...
     }
   }
   ```

2. **diffs/report.txt**: Human-readable summary

   - For each category: count of excellent/good/acceptable/poor
   - Top 5 best-performing documents
   - Top 5 worst-performing documents
   - Common failure patterns

3. **diffs/detailed/{document}.diff**: Per-document diff
   - Full diff output
   - Loss category analysis
   - Specific failure points

## Validation Rules

### Absolute Truths

- **No content hallucination**: Extracted text must be subset of gold (with minor exceptions for OCR artifacts)
- **No structure inversion**: If gold has structure X, extracted must not present contradiction
- **Format → plaintext OK**: Losing **bold** to bold is acceptable, but losing **meaning** is not

### Subjective Judgment

- Minor spacing differences ignored (normalized)
- Formatting loss accepted if content preserved
- Structure loss in unsupported formats (e.g., RTL text) expected

## Tools & Implementation

### Required

- Rust test harness for automation
- Diff tool (unified diff format)
- JSON serialization for metrics
- Markdown parser for structure validation

### Deliverables

- Test data in `test-data/` with organized structure
- PDF generation script (external tool)
- Rust test crate `tests/comprehensive_evaluation.rs`
- Evaluation runner script `eval.sh`
- Report generator `report.py`

## Test Execution

### Running Tests

```bash
# Run all PDF extraction tests
cd edgequake/crates/edgequake-pdf
cargo test

# Run specific test suites
cargo test --test quality_evaluation
cargo test --test integration_tests
cargo test --test edge_cases_and_complex

# Run lib tests only (fast)
cargo test --lib

# Run with coverage
cargo test --lib -- --nocapture
```

### Current Test Status (January 2026)

- **Total Tests**: 239 passing
- **Lib Tests**: 164 passing
- **Integration Tests**: 75 passing
- **Clippy Warnings**: 6 (acceptable intentional patterns)

### Test Categories

1. **Unit Tests** (164 in lib):

   - Encoding tests (17)
   - Lattice engine tests (7)
   - SotaBackend tests (6)
   - Layout tests (20+)
   - Processor tests (50+)
   - Renderer tests (40+)

2. **Integration Tests** (75):
   - Quality evaluation (5)
   - Edge cases (19)
   - Comprehensive data (53)
   - Smoke tests (1)
   - Layout tests (1)

---

## Version History

| Version | Date       | Changes                                      |
| ------- | ---------- | -------------------------------------------- |
| 1.0     | 2025-01-03 | Initial protocol definition                  |
| 1.1     | 2026-01-03 | Updated counts (120 gold files), test status |
