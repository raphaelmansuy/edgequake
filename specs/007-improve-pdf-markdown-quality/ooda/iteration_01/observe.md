# OODA Iteration 01 - Observe

**Mission**: Improve PDF-to-Markdown Conversion Quality
**Mission File**: `specs/007-improve-pdf-markdown-quality.md`
**Date**: 2026-02-05

---

## 1. Territory Mapping

### 1.1 Codebase Structure

```
edgequake-pdf/src/
├── backend/                 # PDF parsing (lopdf-based)
│   ├── block_builder.rs     # Block construction from text elements
│   ├── column_detection.rs  # Histogram-based column detection (deprecated)
│   ├── content_parser.rs    # PDF content stream parsing
│   ├── element_processing.rs # Deduplication, merging
│   ├── elements.rs          # RawChar, TextElement types
│   ├── encodings.rs         # Font encoding (WinAnsi, MacRoman, etc.)
│   ├── extraction_engine.rs # Main extraction orchestration
│   ├── font_handling.rs     # Font info, bold/italic detection
│   ├── glyph_list.rs        # Adobe glyph name mapping
│   ├── lattice.rs           # Table detection via line intersections
│   ├── pdfium.rs            # PDFium backend (optional)
│   ├── pdfium_backend.rs    # PDFium extraction logic
│   ├── spatial.rs           # R-tree spatial indexing
│   ├── text_grouping.rs     # Char→Line→Block grouping
│   └── truetype_cmap.rs     # TrueType CMap parsing
├── layout/                  # Layout analysis
│   ├── block_classifier.rs  # Header/list/code detection
│   ├── column_detector.rs   # DBSCAN column detection (OODA-46)
│   ├── geometric.rs         # DBSCAN clustering utilities
│   ├── pymupdf_grouper.rs   # Two-column text grouping
│   ├── pymupdf_renderer.rs  # Markdown rendering
│   ├── pymupdf_structs.rs   # Span, Line, Block structures
│   ├── reading_order.rs     # Reading order algorithms
│   └── xy_cut.rs            # XY-cut segmentation
├── processors/              # Post-processing
│   ├── code_block.rs        # Code block detection
│   ├── formatting.rs        # Bold/italic Markdown
│   ├── list_detection.rs    # List item detection
│   ├── mod.rs               # Processor pipeline
│   └── table_detection.rs   # Table detection/formatting
├── renderers/               # Output generation
│   ├── json.rs              # JSON output
│   └── markdown.rs          # Markdown rendering
├── bin.rs                   # CLI entry point
├── config.rs                # Configuration
├── error.rs                 # Error types
├── extractor.rs             # High-level API
├── lib.rs                   # Library entry
└── vision.rs                # LLM vision integration
```

### 1.2 Test Results Baseline

```
cargo test --package edgequake-pdf --lib
Result: 494 passed, 0 failed, 0 ignored
Time: 0.08s
```

### 1.3 Current Quality Issues (Observed from AI_Services__Elitizon.pdf)

1. **Fragmented text blocks** - Text split incorrectly:
   - "Elitizon designs and delivers production-grade AI systems with a focus on"
   - "workflows" (should be same paragraph)
   
2. **Reading order issues** - Content appears out of order:
   - "vs-buy, and investment sequencing." appears before "AI Strategy & Roadmap"
   - List items mixed with section headers

3. **Missing list markers** - Bullet points not detected:
   - Items like "Use-case portfolio" should be bulleted
   - Nested lists not indented

4. **Column merging artifacts** - Two-column content merged incorrectly:
   - ": prioritized use cases, target operating model, build- : reference architecture"
   - This shows left/right columns being concatenated

5. **Bold/Italic inconsistency** - Some bold preserved, some lost:
   - "**software delivery automation**" preserved
   - "AI Strategy & Roadmap" lost bold formatting

### 1.4 PyMuPDF4LLM Reference Analysis

Key modules from `zz-explore/pymupdf4llm/`:

**multi_column.py** (531 lines):
- Uses `textpage.extractDICT()` for structured text
- `column_boxes()` - identifies column regions
- Three-phase rectangle joining algorithm
- Sorts by computed key: `(left_rect.y0, current_rect.x0)`
- Handles background colors separately

**Key Algorithm**:
```
1. Extract text blocks from page
2. Filter by header/footer margins
3. Join touching rectangles (3 phases)
4. Sort by "left-most overlapping rect" rule
5. Return bboxes for extraction
```

### 1.5 Test Documents Inventory

`zz_test_docs/` contains 25 PDFs:
- **Business docs**: AI_Services__Elitizon.pdf (5 pages)
- **Academic papers**: agentfail_2601.22984v1.pdf, lighrag_2410.05779v3.pdf
- **Technical manuals**: Renault PDFs (complex tables)
- **Multilingual**: French technical docs

### 1.6 Lines of Code Analysis

```
edgequake-pdf/src/: ~15,000 lines Rust
Key files by size:
- backend/lattice.rs: 800+ lines
- backend/extraction_engine.rs: 700+ lines
- layout/column_detector.rs: 500 lines
- layout/pymupdf_grouper.rs: 600+ lines
```

---

## 2. Key Observations

### 2.1 Architecture Strengths
- DBSCAN clustering for column detection (no magic bin sizes)
- Spatial R-tree indexing for efficient queries
- Lattice-based table detection via line intersections
- Font-based bold/italic detection

### 2.2 Architecture Weaknesses
- **Text grouping happens before column detection** - causes fragments
- **Reading order computed after layout** - loses column context
- **List detection relies on text patterns** - misses visual bullets
- **No header/footer margin filtering** - noise in output

### 2.3 Critical Gaps vs PyMuPDF4LLM

| Feature | PyMuPDF4LLM | EdgeQuake | Gap |
|---------|-------------|-----------|-----|
| Header/footer margins | Yes | No | **Critical** |
| Rectangle joining (3 phases) | Yes | No | High |
| Background color handling | Yes | No | Medium |
| Column-aware sorting | Yes | Partial | High |
| Page-level bboxes | Returns | Renders inline | Medium |

---

## 3. Test Document Conversion Results

### AI_Services__Elitizon.pdf (5 pages)

**Issues Found**:
1. Page 1: Text fragmentation (3 instances)
2. Page 1-2: List items not bulleted (12 items)
3. Page 2-3: Reading order jumbled
4. Page 3: Table-like content rendered as paragraphs
5. All pages: No page breaks indicated

**Quality Score Estimate**: 60/100

---

## 4. Dependency Analysis

```toml
# Cargo.toml key dependencies
lopdf = "0.34"          # PDF parsing
rstar = "0.12"          # R-tree spatial index
rayon = "1.10"          # Parallel processing
tracing = "0.1"         # Logging
tokio = "1.43"          # Async runtime
```

No blocking dependency issues found.

---

## 5. Next Steps for Orient Phase

1. Analyze text grouping algorithm in detail
2. Trace reading order computation path
3. Compare list detection logic with visual inspection
4. Identify specific code locations for improvements
