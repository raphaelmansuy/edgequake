# Mission: Perfect PDF to Markdown Conversion

## Task

Your mission is to achieve **production-grade, high-fidelity PDF to Markdown conversion** for the edgequake-pdf crate with **best-in-class speed and quality**. This involves:

1. **Speed Optimization**: Achieve <1 second per page extraction while maintaining quality
2. **Quality Enhancement**: Match or exceed quality of leading tools (Marker, Docling, PyMuPDF4LLM)
3. **Test Acceleration**: Split tests into micro-focused suites for rapid feedback loops
4. **Algorithm Perfection**: Iteratively improve using insights from Python ecosystem tools

## 🚀 Primary Goals (Updated Feb 2026)

| Goal | Target | Current | Priority |
|------|--------|---------|----------|
| **Speed** | <1s per page | 0.028-0.104s ✅ | ACHIEVED |
| **Quality (TPS)** | ≥98% | 81.3% | P0 - Critical |
| **Quality (SFS)** | ≥95% | 68.0% | P0 - Critical |
| **Smoke Test Time** | <1s total | 0.07s ✅ | Achieved |
| **Feature Test Time** | <5s total | 0.32s ✅ | Achieved |
| **Micro Tests** | <0.1s each | 0.02-0.22s ✅ | Achieved |

---

## 🔬 Python PDF Tools: Lessons Learned

Study these leading Python tools for inspiration on architecture and algorithms:

### 1. Marker (31K⭐) - [github.com/datalab-to/marker](https://github.com/datalab-to/marker)

**Key Insights:**
- **Pipeline Architecture**: Providers → Builders → Processors → Renderers
- **LLM Hybrid Mode**: Optional LLM enhancement for tables, math, forms
- **Batch Processing**: 25 pages/second on H100 in batch mode
- **Performance**: 122 pages/second projected throughput

**Applicable Patterns:**
```
┌─────────────┐    ┌──────────────┐    ┌─────────────┐    ┌──────────────┐
│  Providers  │ →  │   Builders   │ →  │ Processors  │ →  │  Renderers   │
│ (PDF/text)  │    │ (layout/OCR) │    │ (tables/eq) │    │ (Markdown)   │
└─────────────┘    └──────────────┘    └─────────────┘    └──────────────┘
```

### 2. Docling (52K⭐) - [github.com/docling-project/docling](https://github.com/docling-project/docling)

**Key Insights:**
- **Unified DoclingDocument Format**: Expressive intermediate representation
- **Heron Layout Model**: New fast layout detection
- **VLM Support**: GraniteDocling for vision-language processing
- **Multiple Export Formats**: Markdown, HTML, DocTags, JSON

**Applicable Patterns:**
- Strong typing with Pydantic models
- Layout detection as separate pipeline stage
- Modular format-agnostic export

### 3. PyMuPDF4LLM (1.3K⭐) - [github.com/pymupdf/pymupdf4llm](https://github.com/pymupdf/pymupdf4llm)

**Key Insights:**
- **Lightweight**: No ML models, pure extraction
- **Multi-column**: Automatic column detection and reading order
- **Page Chunks**: `page_chunks=True` for RAG-optimized output
- **Speed**: Very fast, relies on MuPDF's C library

**Applicable Patterns:**
- Character-level bbox extraction
- Geometric clustering for columns
- Minimal dependencies for speed

### 4. MarkItDown (86K⭐) - [github.com/microsoft/markitdown](https://github.com/microsoft/markitdown)

**Key Insights:**
- **MCP Server**: Model Context Protocol for LLM agents
- **Multi-format**: PDF, DOCX, PPTX, XLSX, HTML, EPUB
- **Streaming API**: `convert_stream()` for memory efficiency
- **Plugin System**: Extensible with 3rd-party plugins

**Applicable Patterns:**
- Stream-based processing (no temp files)
- Plugin architecture for extensibility
- Azure Document Intelligence integration option

---

## 🏎️ Speed Optimization Strategy

### Algorithm Complexity Targets

| Operation | Current | Target | Approach |
|-----------|---------|--------|----------|
| Text Extraction | O(n²) suspected | O(n) | Direct character stream |
| Column Detection | O(n²) | O(n log n) | Interval tree clustering |
| Table Detection | O(cells²) | O(cells) | Lattice line detection |
| Block Merging | O(blocks²) | O(blocks) | Spatial indexing (R-tree) |

### Speed Improvements Roadmap

1. **Lazy Loading**: Only parse pages when needed
2. **Parallel Page Processing**: Independent pages in parallel
3. **Spatial Indexing**: R-tree for bbox queries instead of O(n) scans
4. **Character Buffering**: Stream characters instead of collecting all first
5. **Skip Irrelevant Content**: Header/footer detection early in pipeline
6. **Incremental Extraction**: Cache intermediate results per page

### Performance Profiling Commands

```bash
# Profile extraction time breakdown
RUST_LOG=edgequake_pdf=trace cargo run --example convert_test_docs 2>&1 | grep -E "took|elapsed"

# Flamegraph for hotspot analysis
cargo flamegraph --example convert_test_docs

# Memory profiling
heaptrack cargo run --example convert_test_docs
```

---

Extremely important:

Ensure to use First Principles thinking. Use your knowledge of PDF structures, text encoding, layout analysis, and Markdown syntax to guide your decisions. Use Donald Knuth knowledge about text composition and typesetting where applicable: space between letters, words, line breaks, paragraph structure, ligatures, font styles, etc. If you don't know something, research it thoroughly on the web: PDF, typesetting, Markdown, text extraction techniques. When you take decision don't rely on easy an d short coming heuristics only: explain WHY you chose specific thresholds or algorithms using First Principles reasoning. Use comments in code to explain your thinking.

YOU MUST ENSURE THE TEST ARE EXECUTED QUICKLY: optimize for speed and efficiency. Use O notation to analyze time complexity of your algorithms. You must ensure that the extraction runs quickly even on large documents without sacrificing quality.


You must also optimize the conversion process for speed and efficiency, ensuring that the extraction runs quickly even on large documents without sacrificing quality. Don't use image extraction by default in tests.


You must always assess the golden standard Markdown output using the Markitdown MCP tool to compare against your extracted Markdown.

ALWAY USE ASCII DIAGRAMS TO DEEPLY REFLECT YOUR THINKING about Data, geometric processing pipelines, architecture, workflows, etc. Search about PDF handling, text extraction, layout analysis, and Markdown formatting as needed. See other project such as Markitdown (https://github.com/microsoft/markitdown)

Use markitdown mcp to compare extracted Markdown against gold standard references: but be very smart about quality of gold standards: if the gold standard is poor, your quality metrics will be misleading. You must ensure that the gold standard markdown files are of high quality and accurately represent the intended structure and content of the original PDFs. You want to exceed markitdown quality if possible: it why you are building your own PDF to Markdown converter. Always analyze the gold standard files for quality before using them as benchmarks: use your knowledge of Markdown syntax and best practices to assess their quality. If you find gold standard files that are subpar, document the issues and consider improving them or creating new high-quality references.


You can always challenge gold with markitdown if you don't understand the mistmatches: sometimes markitdown makes mistakes too. Use your deep knowledge of PDF internals, text extraction techniques, geometric processing, and Markdown formatting to guide your analysis. The gold data files where not created by markitdown: they were created by human experts. So you can always challenge markitdown if you find mistakes in the gold data files.


Be careful about parallelizing too much: sometimes sequential processing is better for quality because you can use context from previous pages to inform extraction on later pages. Use your judgment to balance speed and quality based on document characteristics.


Addeditional Important Guidelines:
Be generic in your approach: avoid hardcoding for specific documents. Your algorithms should generalize well across diverse PDF layouts and content types. Create a rule for arvivx for example is a BIG BIG code smells --> You remove such kind of hardcoding by building generic algorithms that can handle a wide range of cases based on document structure and content analysis, using deep knowledge of PDF internals and text extraction techniques and geometric processing.

VERY IMPORTANT: Optimize for speed and efficiency. Study the algoryithms used in text extraction and layout analysis to ensure they run quickly even on large documents. Use big notation (O notation) to analyze time complexity of your algorithms.





OODA Loop directory: specs/004-perfect-pdf-markdown-conversion/ooda_loop/

## Test Acceleration Strategy ✅ ENHANCED

Tests have been restructured into **micro-focused suites** for maximum speed and targeted feedback:

### Test Pyramid Architecture

```
                    ┌─────────────────────┐
                    │   Comprehensive     │  ← Run before release (2min)
                    │   (7 real PDFs)     │
                    └──────────┬──────────┘
                               │
                  ┌────────────┴────────────┐
                  │      Feature Tests      │  ← Run before commit (5s)
                  │  (tables, columns, etc) │
                  └────────────┬────────────┘
                               │
    ┌──────────────────────────┴──────────────────────────┐
    │                    Micro-Smoke Tests                 │  ← Run on save (<1s each)
    │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐ │
    │  │  Text   │  │ Tables  │  │ Columns │  │  Font   │ │
    │  │ 0.02s   │  │ 0.03s   │  │ 0.02s   │  │ 0.01s   │ │
    │  └─────────┘  └─────────┘  └─────────┘  └─────────┘ │
    └─────────────────────────────────────────────────────┘
```

### Tier 0: Micro-Smoke Tests (<0.1s each) - NEW!

**Purpose:** Instant feedback for specific functionality during development

```bash
# Run individual micro-tests
cargo test --package edgequake-pdf --test micro_text      # 0.02s - basic text
cargo test --package edgequake-pdf --test micro_tables    # 0.03s - table detection
cargo test --package edgequake-pdf --test micro_columns   # 0.02s - column detection
cargo test --package edgequake-pdf --test micro_fonts     # 0.01s - font encoding
cargo test --package edgequake-pdf --test micro_structure # 0.02s - headers/lists
```

**Files to create:**
- `tests/micro_text.rs` - Single paragraph extraction (1 tiny PDF)
- `tests/micro_tables.rs` - 2x2 table detection (1 tiny PDF)
- `tests/micro_columns.rs` - 2-column reading order (1 tiny PDF)
- `tests/micro_fonts.rs` - ToUnicode/CID handling (1 tiny PDF)
- `tests/micro_structure.rs` - H1-H3 detection (1 tiny PDF)

**Design principles:**
- Each test uses **exactly 1 minimal PDF** (< 10KB)
- PDFs are **generated programmatically** or embedded as bytes
- No file I/O in hot path (use `include_bytes!`)
- Test **one assertion** per test function

### Tier 1: Smoke Tests (< 1 second) - DEFAULT

```bash
cargo test --package edgequake-pdf --test quick_smoke
```

- **File:** `tests/quick_smoke.rs`
- **PDFs:** 3 small files (sample.pdf, 001_simple_text.pdf, 002_headers_and_lists.pdf)
- **Purpose:** Instant feedback during development
- **Checks:** Non-zero output, no crashes, basic parsing
- **Actual time:** 0.07s (✅ target: <5s)

### Tier 2: Feature Tests (< 1 minute)

```bash
cargo test --package edgequake-pdf --test basic_features --features slow-tests
```

- **File:** `tests/basic_features.rs`
- **PDFs:** 4 medium files testing columns, tables, structure
- **Purpose:** Verify feature functionality before committing
- **Checks:** Table detection, multi-column, batch processing
- **Actual time:** 0.32s (✅ target: <30s)

### Tier 3: Comprehensive Quality (2+ minutes)

```bash
cargo test --package edgequake-pdf --test comprehensive_quality --features comprehensive-tests
```

- **File:** `tests/comprehensive_quality.rs`
- **PDFs:** All 7 files in real_dataset/ (27MB total)
- **Purpose:** Full quality validation before releases
- **Checks:** Text Preservation Score (TPS), Structural Fidelity Score (SFS)
- **Actual time:** 118s (✅ target: <3min)
- **Current quality:** 74.6% overall (Text: 81.3%, Structure: 68.0%)

### Optimization Results

| Test Tier           | Before           | After  | Speedup | Use Case        |
| ------------------- | ---------------- | ------ | ------- | --------------- |
| **Micro**           | N/A              | 0.02s  | ∞       | Per-keystroke   |
| **Smoke**           | 116s (all tests) | 0.07s  | 1657x   | Every save      |
| **Feature**         | 116s             | 0.32s  | 362x    | Before commit   |
| **Comprehensive**   | 116s             | 118s   | 1x      | Before release  |

### Speed-Quality Tradeoff Matrix

```
Quality ▲
    │      ┌─────────────────────────┐
100%│      │   TARGET ZONE          │
    │      │   (95%+ quality,       │
 95%│  ....│.....<1s.per.page).....│............
    │      │                         │
 80%│  ●   │ Current: 81% TPS       │
    │  Current                       │
 68%│  ● Current: 68% SFS           │
    │                                │
    └────────────────────────────────────────▶ Speed
         17s    5s    1s   0.5s   0.1s  (per page)
```

### CI/CD Integration (Updated)

**Recommended pipeline:**

```yaml
# On every push (instant feedback)
- name: Micro tests
  run: |
    cargo test --package edgequake-pdf --test micro_text
    cargo test --package edgequake-pdf --test micro_tables

# PR checks (fast feedback)
- name: Smoke tests
  run: cargo test --package edgequake-pdf --test quick_smoke

# Pre-merge (feature validation)
- name: Feature tests
  run: cargo test --package edgequake-pdf --test basic_features --features slow-tests

# Nightly/Release (full quality)
- name: Comprehensive tests
  run: cargo test --package edgequake-pdf --test comprehensive_quality --features comprehensive-tests
```

### Developer Workflow (Updated)

```
┌──────────────────────────────────────────────────────────────────┐
│  Code Change                                                      │
└─────────────────────────────┬────────────────────────────────────┘
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│  1. MICRO TEST (0.02s) - Test specific feature being changed     │
│     cargo test --package edgequake-pdf --test micro_tables       │
└─────────────────────────────┬────────────────────────────────────┘
                              ▼ Pass?
┌──────────────────────────────────────────────────────────────────┐
│  2. SMOKE TEST (0.07s) - Verify no regressions                   │
│     cargo test --package edgequake-pdf --test quick_smoke        │
└─────────────────────────────┬────────────────────────────────────┘
                              ▼ Pass?
┌──────────────────────────────────────────────────────────────────┐
│  3. FEATURE TEST (0.32s) - Validate feature interactions         │
│     cargo test --package edgequake-pdf --features slow-tests     │
└─────────────────────────────┬────────────────────────────────────┘
                              ▼ Pass?
┌──────────────────────────────────────────────────────────────────┐
│  4. GIT COMMIT - All fast tests pass                             │
└─────────────────────────────┬────────────────────────────────────┘
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│  5. COMPREHENSIVE (118s) - Run before PR/release only            │
│     cargo test --package edgequake-pdf --features comprehensive  │
└──────────────────────────────────────────────────────────────────┘
```

**Quick reference:**

```bash
# Per-feature development (instant)
cargo test --package edgequake-pdf --test micro_text        # 0.02s
cargo test --package edgequake-pdf --test micro_tables      # 0.03s

# Development (default - no flags needed)
cargo test --package edgequake-pdf --test quick_smoke       # 0.07s

# Integration testing
cargo test --package edgequake-pdf --test basic_features --features slow-tests  # 0.32s

# Full quality validation
cargo test --package edgequake-pdf --test comprehensive_quality --features comprehensive-tests  # 118s

# Run all tests (4 tiers)
cargo test --package edgequake-pdf --all-features
```

### First Principles: Why Split Tests?

**Problem:** Original quality_evaluation.rs processed all 7 PDFs (27MB) sequentially, taking 116 seconds.

**Root cause:** No incremental feedback loop. Developers wait 2 minutes to see if basic changes work.

**Solution:** Stratified testing based on Donald Knuth's principle:

> "Premature optimization is the root of all evil, but we should not pass up our opportunities in that critical 3%."

The critical 3% is the development loop. Most changes need only smoke tests (<1s). Feature tests (0.32s) catch integration issues. Comprehensive tests (118s) validate production quality.

**Trade-off:** Maintaining 4 test tiers vs. 1657x faster feedback loop. Worth it.

### First Principles: Speed vs Quality

**Key Insight from Python Tools:**

1. **Marker**: Uses heuristics first, LLM only when needed → Fast by default, accurate optionally
2. **PyMuPDF4LLM**: Pure extraction, no ML → Fastest, lower quality ceiling
3. **Docling**: Deep learning models → Highest quality, slower

**Our Strategy: Tiered Quality Modes**

```rust
pub enum QualityMode {
    Fast,       // O(n) algorithms only, no expensive detection
    Balanced,   // Default: heuristics + simple ML
    Quality,    // All algorithms, including expensive table/layout detection
    LLMEnhanced // Use LLM for correction (optional)
}
```

**Algorithm Selection by Mode:**

| Feature | Fast | Balanced | Quality | LLM |
|---------|------|----------|---------|-----|
| Text extraction | ✅ | ✅ | ✅ | ✅ |
| Column detection | Skip | Heuristic | R-tree | R-tree |
| Table detection | Skip | Lattice only | Lattice+Stream | +LLM fix |
| Reading order | Simple | Geometric | Graph-based | +LLM |
| Font fallback | None | Guess | Full CMap | +LLM |

---

RUST_LOG=info cargo run --example convert_test_docs 2>&1 | cat

Avoid Magic Number !!! : Think by First Principles to choose how to set thresholds and constants.
Use comments to explain WHY you chose specific values.

## Context

- **Location**: `/Users/raphaelmansuy/Github/03-working/edgequake/edgequake/crates/edgequake-pdf/`
- **Test Documents**: `/Users/raphaelmansuy/Github/03-working/edgequake/zz_test_docs/`
- **Existing Test Data**: `/Users/raphaelmansuy/Github/03-working/edgequake/edgequake/crates/edgequake-pdf/test-data/`

### Comparison Tool: MCP Markitdown

Use `mcp_markitdown_convert_to_markdown` tool to generate reference markdown from PDFs. This provides a baseline for comparison against our extraction output.

### Test Documents in `zz_test_docs/`


Addition new documents have been added for evaluation, former documents:

| Document                                    | Pages | Type                     | Status                    |
| ------------------------------------------- | ----- | ------------------------ | ------------------------- |
| `Qwen.pdf`                                  | 1     | Type3 fonts, web capture | ✅ Fixed (OODA-01/02)     |
| `001-BEYONG-TRANFORMER-OUTLINE-V1_1.pdf`    | 10+   | Academic outline         | ✅ Extracts               |
| `AgenticPlatformReference Architecture.pdf` | 50+   | Architecture doc         | ✅ Extracts               |
| `Apple-Sandbox-Guide-v1.0.pdf`              | ?     | Technical guide          | 🔄 New - needs evaluation |
| `agentfail_2601.22984v1.pdf`                | ?     | arXiv paper              | 🔄 New - needs evaluation |
| `hotmess_2601.23045v1.pdf`                  | ?     | arXiv paper              | 🔄 New - needs evaluation |


List documents  to analyze for root causes of poor extraction quality.

### Current State Assessment

**Known Issues Discovered**:

1. `Qwen.pdf` - ✅ FIXED: Was returning 0 bytes due to OCR layer detection issue
2. `001-BEYONG-TRANFORMER-OUTLINE-V1_1.pdf` - Table extraction produces malformed output
3. `AgenticPlatformReference Architecture.pdf` - ASCII art not preserved as code blocks
4. New PDFs need evaluation against markitdown baseline

**Root Cause Candidates**:

- Font encoding issues (CID fonts, embedded fonts, ToUnicode mapping failures)
- Table detection heuristics too strict or too loose
- Multi-column layout detection misfiring
- Block merging destroying structure
- Special content (ASCII art, code blocks) not recognized
- Others ... need deeper analysis

---

## Process: OODA Loop (50 iterations minimum)

Execute iterative OODA cycles. Each iteration produces 4 files:

**⚠️ CRITICAL: Re-read this mission file at the START of EVERY iteration!**

Mission file: `specs/004-perfect-pdf-markdown-conversion.md`

You must always produce 4 files per iteration:

1. `observe.md` → Map the territory. Never assume code structure - verify against codebase. Search web for documentation when needed.
2. `orient.md` → Analyze findings using First Principles. Assess risks/benefits of each approach.
3. `decide.md` → Prioritize specific changes by signal value and impact.
4. `act.md` → Implement changes with precision. Reference file:line numbers and commit SHAs.

```
specs/004-perfect-pdf-markdown-conversion/ooda_loop/
├── iteration_01/
│   ├── observe.md   # Data gathered: code, business rules, workflows
│   ├── orient.md    # Analysis of findings vs. current docs
│   ├── decide.md    # Prioritized action plan
│   └── act.md       # Changes made, with file:line references + commit hashes
├── iteration_02/
│   └── observe.md
│   └── orient.md
│   └── decide.md
│   └── act.md
├── iteration_03/
│   └── ...
└── summary.md       # Cross-iteration insights
```

### Per-Iteration Requirements

| Step        | Output                                                              |
| ----------- | ------------------------------------------------------------------- |
| **Observe** | Code analysis, failure root cause investigation, dependency mapping |
| **Orient**  | Gap analysis, quality assessment, research findings                 |
| **Decide**  | Specific changes prioritized by signal value                        |
| **Act**     | Implementation with commit (`OODA-XX: <decision summary>`)          |

### Constraints

1. **Re-read mission** every iteration: `specs/004-perfect-pdf-markdown-conversion.md`
2. **Continue** from existing iterations—never restart
3. **Reference** real code: file paths, line numbers, commit SHAs
4. **Use ASCII diagrams** for architecture/flow visualization
5. **Split large files** for maintainability (SRP)
6. **Optimize** Rust build speed (latest toolchain)
7. **Document amendments** in WHY comments with high signal value
8. **You must perform tests** and deliver evidence that all tests pass

---

## Testing Roadmap & Current Status

### Current Test Infrastructure ✅

**Status:** Test acceleration complete (OODA-09)

**Files:**

- `tests/quick_smoke.rs` - Smoke tests (0.07s, 3 PDFs)
- `tests/basic_features.rs` - Feature tests (0.32s, 4 PDFs) [--features slow-tests]
- `tests/comprehensive_quality.rs` - Quality metrics (118s, 7 PDFs) [--features comprehensive-tests]
- `tests/quality_evaluation.rs` - DEPRECATED (backward compatibility only)

**Current Coverage:**

- ✅ Smoke tests: 3 PDFs (simple text, headers/lists, sample)
- ✅ Feature tests: 4 PDFs (multi-column, tables, batch processing)
- ✅ Comprehensive: 7 real academic papers from arXiv
- ⚠️ Total: 14 unique PDFs tested (target: 100 per spec)

**Quality Metrics (Comprehensive Suite - Feb 2, 2026):**

- Text Preservation: 81.3%
- Structural Fidelity: 68.0%
- Overall Quality: 74.6%
- Target: 95%+ on all metrics

**Gap Analysis:**

- 86 PDFs needed to reach 100 test coverage goal
- Structure fidelity at 68% vs. 95% target → 27 percentage points gap
- Need: table detection improvements, multi-column reading order fixes

### Test Expansion Plan (OODA-10+)

**Priority 0: Create Micro-Tests (OODA-10) - NEW**

Create minimal test PDFs and test files for instant feedback:

```
tests/
├── micro_text.rs          # 0.02s - Single paragraph
├── micro_tables.rs        # 0.03s - 2x2 table
├── micro_columns.rs       # 0.02s - 2-column layout
├── micro_fonts.rs         # 0.01s - CID/ToUnicode
├── micro_structure.rs     # 0.02s - Headers/lists
└── test-pdfs/
    ├── micro_text.pdf       # < 5KB
    ├── micro_table_2x2.pdf  # < 5KB
    ├── micro_columns.pdf    # < 5KB
    ├── micro_cid_font.pdf   # < 10KB
    └── micro_headers.pdf    # < 5KB
```

**Micro-Test Template:**

```rust
// tests/micro_text.rs
//! Micro-test for basic text extraction
//! Target: <0.05s execution time

use edgequake_pdf::PdfExtractor;

// Embed PDF bytes directly - no file I/O
const MICRO_PDF: &[u8] = include_bytes!("test-pdfs/micro_text.pdf");

#[test]
fn test_basic_text_extraction() {
    let extractor = PdfExtractor::from_bytes(MICRO_PDF).unwrap();
    let md = extractor.to_markdown().unwrap();
    
    assert!(md.contains("Hello World"));
    assert!(md.len() > 10);
}

#[test]
fn test_no_crash_on_empty() {
    // Edge case: minimal valid PDF
    let result = PdfExtractor::from_bytes(MICRO_PDF);
    assert!(result.is_ok());
}
```

**Priority 1: Complete 10 test categories (OODA-11-15)**
See "Test Categories (Comprehensive Coverage)" section below for full breakdown.

Focus on:

1. Tables (15 PDFs) - current bottleneck at 68% structural fidelity
2. Multi-column (10 PDFs) - reading order accuracy needed
3. Edge cases (15 PDFs) - robustness improvements

**Priority 2: Improve quality metrics (OODA-16-25)**

Inspired by Python tools' approaches:

- Target: 85%+ overall quality → 95%+ final target
- Strategy 1: Port Marker's lattice table detection algorithm
- Strategy 2: Implement PyMuPDF4LLM's column clustering
- Strategy 3: Add optional LLM correction layer (like Marker's `--use_llm`)
- Measure: TPS, SFS, ROA, TCA, FPS metrics

**Priority 3: Performance optimization (OODA-26-30)**

- Target: <1 second per page average (currently ~17s per PDF)
- Current bottleneck: Character extraction + block merging
- Strategy: Profile → Identify O(n²) → Replace with O(n log n)
- Benchmark against: PyMuPDF4LLM (10+ pages/s target)

---

## Quality Metrics (Target: 95%+ on all)

### 1. Text Preservation Score (TPS)

```
TPS = (words_in_output ∩ words_in_source) / words_in_source × 100
```

- Measures: No text loss during extraction
- Target: ≥ 98%

### 2. Structural Fidelity Score (SFS)

```
SFS = (matched_structures / source_structures) × 100
```

Structures: headers, lists, tables, code blocks, blockquotes

- Target: ≥ 95%

### 3. Reading Order Accuracy (ROA)

```
ROA = correct_block_sequence_pairs / total_block_pairs × 100
```

- Measures: Multi-column and complex layouts read correctly
- Target: ≥ 95%

### 4. Table Cell Accuracy (TCA)

```
TCA = (correct_cells / total_cells) × 100
```

- Measures: Table structure and cell content preservation
- Target: ≥ 90%

### 5. Format Preservation Score (FPS)

```
FPS = (preserved_formatting_elements / total_formatting_elements) × 100
```

Formatting: bold, italic, headers, links

- Target: ≥ 90%

### 6. Edge Case Robustness (ECR)

```
ECR = (successful_extractions / edge_case_pdfs) × 100
```

Edge cases: corrupt fonts, CID encodings, rotated text, scanned pages

- Target: ≥ 85%

---

## 📊 Benchmark Against Python Tools

To validate our speed and quality claims, compare against leading tools:

### Speed Benchmark Protocol

```bash
# Create benchmark script
#!/bin/bash
PDF="test-data/real_dataset/arxiv_2408.09869.pdf"

echo "=== Speed Benchmark ==="

# Our tool (Rust)
echo "EdgeQuake PDF:"
time cargo run --release --example convert_pdf -- "$PDF" > /dev/null

# PyMuPDF4LLM (Python - fastest)
echo "PyMuPDF4LLM:"
time python -c "import pymupdf4llm; pymupdf4llm.to_markdown('$PDF')" > /dev/null

# MarkItDown (Python - Microsoft)
echo "MarkItDown:"
time python -c "from markitdown import MarkItDown; MarkItDown().convert('$PDF')" > /dev/null

# Marker (Python - ML-based)
echo "Marker (no LLM):"
time marker_single "$PDF" --output_format markdown > /dev/null
```

### Quality Benchmark Protocol

Compare output quality on the same PDFs:

| Tool | TPS | SFS | ROA | Speed (pages/s) |
|------|-----|-----|-----|-----------------|
| **EdgeQuake (target)** | ≥98% | ≥95% | ≥95% | ≥1.0 |
| EdgeQuake (current) | 81% | 68% | TBD | 0.06 |
| Marker (reported) | 95.7% | - | - | 2.8 |
| PyMuPDF4LLM | ~85% | ~60% | ~80% | 10+ |
| Docling | 86.7% | - | - | 0.3 |

### Competitive Analysis: What They Do Better

1. **Marker**: LLM hybrid mode for table merging across pages
2. **Docling**: Vision-language models for complex layouts
3. **PyMuPDF4LLM**: Raw speed through C library + minimal processing
4. **MarkItDown**: Excellent multi-format support, MCP integration

### Our Differentiators

1. **Rust performance**: Native speed without Python GIL
2. **RAG optimization**: Designed for LLM ingestion from day 1
3. **EdgeQuake integration**: Part of graph-based knowledge system
4. **No ML dependencies**: Fast startup, small binary

---

## Test Categories (Comprehensive Coverage)

### Category 1: Basic Text (10 PDFs)

- [ ] Single paragraph
- [ ] Multiple paragraphs
- [ ] Unicode characters (émojis, CJK, RTL)
- [ ] Special symbols (math, currency)
- [ ] Line breaks and spacing

### Category 2: Formatting (10 PDFs)

- [ ] Bold text
- [ ] Italic text
- [ ] Bold+italic combinations
- [ ] Underline (converted to emphasis)
- [ ] Strikethrough

### Category 3: Headers (10 PDFs)

- [ ] H1 through H6 hierarchy
- [ ] Headers with formatting
- [ ] Nested section structure
- [ ] Table of contents

### Category 4: Lists (10 PDFs)

- [ ] Bullet lists
- [ ] Numbered lists
- [ ] Nested lists
- [ ] Mixed list types

### Category 5: Tables (15 PDFs)

- [ ] Simple 2x2 tables
- [ ] Complex multi-column tables
- [ ] Tables with merged cells
- [ ] Tables with formatting
- [ ] Tables spanning pages

### Category 6: Multi-Column (10 PDFs)

- [ ] 2-column academic papers
- [ ] 3-column newsletters
- [ ] Mixed layouts (1-col + 2-col)
- [ ] Columns with images

### Category 7: Code & Technical (10 PDFs)

- [ ] Inline code
- [ ] Code blocks (monospace)
- [ ] Syntax-highlighted code
- [ ] Mathematical formulas

### Category 8: Images & Diagrams (10 PDFs)

- [ ] Embedded images
- [ ] ASCII art diagrams
- [ ] Flow charts
- [ ] Captions

### Category 9: Edge Cases (15 PDFs)

- [ ] CID fonts (e.g., Qwen.pdf failure)
- [ ] Embedded fonts (obfuscated)
- [ ] Rotated text
- [ ] Overlapping layers
- [ ] Digital signatures
- [ ] Password-protected (decrypt test)
- [ ] Very large documents (100+ pages)

### Category 10: Real-World Documents (10 PDFs)

- [ ] Academic papers (arXiv style)
- [ ] Technical specifications
- [ ] Books/manuals
- [ ] Invoices/forms
- [ ] Scanned documents

**Total: 100 test PDFs with gold standard Markdown**

---

## Success Criteria

1. ✅ All 6 quality metrics meet or exceed targets
2. ✅ All 100 test PDFs extract without crashes
3. ✅ `zz_test_docs/` PDFs produce readable, accurate Markdown
4. ✅ Automated regression tests prevent quality degradation
5. ✅ Performance: <1 second per page on average

---

## Technical Focus Areas (Updated Priority Order)

### Priority 0: Speed Optimization (NEW - Critical)

Target: <1 second per page extraction

**Hotspots to Profile:**
- Character extraction loop (suspected O(n²))
- Block merging algorithm (nested loops)
- Font lookup/decoding per character
- Table cell detection (exhaustive search)

**Quick Wins:**
1. **Lazy font loading**: Don't parse all fonts upfront
2. **Character buffering**: Batch character insertions
3. **Skip invisible text**: Early exit for white-on-white
4. **Page independence**: Parallel page extraction

**Benchmark Commands:**
```bash
# Time per operation breakdown
RUST_LOG=edgequake_pdf::timing=trace cargo run --release --example convert_test_docs

# Compare with Python tools baseline
python -c "import pymupdf4llm; print(pymupdf4llm.to_markdown('test.pdf'))" | wc -c
```

### Priority 1: Font Encoding (Qwen.pdf failure)

- ToUnicode CMap handling
- CID font decoding
- Adobe-Identity-H encoding
- Fallback strategies

### Priority 2: Table Detection

**Inspired by Marker's approach:**
- Lattice tables: Line-based detection (fast)
- Stream tables: Whitespace alignment (slower)
- LLM fallback: For complex nested tables

**Algorithm Complexity:**
- Current: O(cells²) - checking all cell pairs
- Target: O(cells log cells) - spatial indexing

### Priority 3: Layout Analysis

**Inspired by Docling's Heron model:**
- Column detection using vertical gap analysis
- Reading order via topological sort
- Block merging with confidence scoring
- Header/footer detection by position

### Priority 4: Special Content

- ASCII art preservation (detect fixed-width blocks)
- Code block detection (monospace fonts)
- Formula recognition (LaTeX output)
- Image placeholder generation

---

## ⚠️ CRITICAL SAFETY MANDATE ⚠️

**YOU MUST RE-READ THIS ENTIRE MISSION FILE AT THE START OF EVERY OODA ITERATION.**

Failure to re-read causes alignment drift → catastrophic safety issues → user frustration → system unreliability.

---

## References

- Test Protocol: `/Users/raphaelmansuy/Github/03-working/edgequake/edgequake/crates/edgequake-pdf/TEST_PROTOCOL.md`
- Gold Dataset: `/Users/raphaelmansuy/Github/03-working/edgequake/edgequake/crates/edgequake-pdf/test-data/gold/`
- Main Extractor: `/Users/raphaelmansuy/Github/03-working/edgequake/edgequake/crates/edgequake-pdf/src/extractor.rs`
- Backend Engine: `/Users/raphaelmansuy/Github/03-working/edgequake/edgequake/crates/edgequake-pdf/src/backend/`
