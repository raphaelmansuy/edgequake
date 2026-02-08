# OODA Iteration 01: Observe

**Date**: 2026-02-06
**Mission File Re-read**: ✅ Confirmed - `specs/001-improve-markdown-2-pdf.md` lines 1-50

---

## Objective

Map the territory of the current edgequake-pdf implementation and pymupdf4llm reference to understand:

1. Current architecture and algorithms
2. Gold standard implementations
3. Quality gaps and improvement opportunities

---

## Current State: edgequake-pdf

### Architecture Overview

Located at: `edgequake/crates/edgequake-pdf/`

```
edgequake-pdf/
├── src/
│   ├── backend/        # PDF extraction backends
│   │   ├── pdfium.rs          # PDFium character extraction
│   │   ├── pdfium_backend.rs  # PdfBackend trait impl
│   │   ├── elements.rs        # RawChar, Span, Line structs
│   │   └── spatial.rs         # R-tree spatial indexing
│   ├── layout/         # Layout analysis and grouping
│   │   ├── pymupdf_pipeline.rs    # Main converter pipeline
│   │   ├── pymupdf_grouper.rs     # Span→Line→Block grouper
│   │   ├── pymupdf_renderer.rs    # Block→Markdown renderer
│   │   ├── pymupdf_structs.rs     # Block, BlockType, Line, Span
│   │   ├── column_detector.rs     # Multi-column detection
│   │   ├── reading_order.rs       # Z-order reading flow
│   │   └── xy_cut.rs              # Recursive X-Y cuts
│   ├── renderers/      # Output format renderers
│   │   ├── markdown.rs
│   │   └── json.rs
│   ├── schema/         # Document model
│   │   ├── block.rs
│   │   ├── block_types.rs
│   │   ├── document.rs
│   │   └── geometry.rs
│   ├── processors/     # Post-processing pipeline
│   ├── formula/        # LaTeX formula detection
│   └── extractor.rs    # High-level API
├── test-data/          # PDF test files
└── tests/              # Integration tests
```

**Key Files Analyzed**:

1. **src/lib.rs:1-133** - Module exports and feature flags
   - Features: `pdfium` (default), `vision`, `slow-tests`, `comprehensive-tests`
   - Re-exports: Backend, Config, Schema, Layout, Processors, Renderers

2. **Cargo.toml:1-100** - Dependencies and build config
   - PDFium backend (pdfium-render 0.8 with image_024, pdfium_latest)
   - ttf-parser 0.25 for TrueType font cmap extraction
   - rayon 1.10 for parallel processing
   - rstar 0.12 for R-tree spatial queries

3. **src/pipeline/pymupdf_pipeline.rs:1-100** - Main conversion pipeline

   ```
   PDF → PDFium → RawChars → Spans → Lines → Blocks → Markdown
   ```

   - Character-level extraction via PDFium
   - Font style detection (bold, italic, monospace)
   - Header detection from font size
   - Code block detection from monospace fonts

4. **src/layout/pymupdf_renderer.rs:1-100** - Block to Markdown converter
   - BlockType: Header(level), Code, ListItem, Table, Paragraph
   - Renders headers with # prefix
   - Code blocks with code fences
   - List items with bullet/number prefixes

### Current Strengths

✅ **PDFium Integration** (Cargo.toml:52-64)

- Accurate character-level bounding boxes
- Font descriptor flags for bold/italic detection
- Thread-safe operation
- F1 >= 0.95 on gold standard tests (per comments)

✅ **Modular Architecture** (src/lib.rs:69-124)

- Clear separation: Backend → Layout → Renderers
- Pluggable backends via PdfBackend trait
- Configurable extraction modes and options

✅ **Spatial Indexing** (Cargo.toml:75-76)

- R-tree (rstar) for O(n log n) spatial queries
- Efficient neighbor finding for text grouping

✅ **Parallel Processing** (Cargo.toml:73-74)

- Rayon for multi-core extraction
- Scales across CPU cores

### Current Limitations (Observed)

⚠️ **Limited Block Type Coverage**

- Current: Header, Code, ListItem, Table, Paragraph (pymupdf_structs.rs)
- Missing: Footnote, Caption, PageHeader, PageFooter, Title, SectionHeader
- **Impact**: Cannot distinguish document structure as precisely as pymupdf4llm

⚠️ **List Detection Gaps**

- No hierarchy level detection (pymupdf4llm has create_list_item_levels)
- No nested list support
- No contiguous segment detection
- **Impact**: Flat lists only, no proper indentation

⚠️ **Style Preservation Incomplete**

- Supports: Bold, Italic, Monospace (pymupdf_renderer.rs)
- Missing: Superscript, Strikeout, font color
- No Private Use Area (PUA) character handling
- **Impact**: Footnotes may render incorrectly, special symbols lost

⚠️ **Table Detection Basic**

- Has BlockType::Table but limited implementation
- No table structure completion via vector graphics
- No fallback strategy for unrecognized tables
- **Impact**: Complex tables may fail silently

⚠️ **Reading Order Heuristics**

- Has xy_cut.rs and reading_order.rs
- Not integrated with pymupdf4llm's find_reading_order algorithm
- **Impact**: Multi-column layouts may have incorrect flow

⚠️ **No OCR Integration**

- No equivalent to pymupdf4llm's should_ocr_page decision logic
- No full-page vs text-only OCR strategies
- **Impact**: Scanned PDFs produce poor results

---

## Gold Standard: pymupdf4llm

### Architecture Overview

Located at: `zz-explore/pymupdf4llm/pymupdf4llm/pymupdf4llm/`

**Core Algorithm File**: `helpers/document_layout.py` (1182 lines)

### Key Data Structures (document_layout.py:596-634)

```python
@dataclass
class LayoutBox:
    x0, y0, x1, y1: float  # Bounding box
    boxclass: str          # 'text', 'picture', 'table', 'list-item', etc.
    image: Optional[bytes] = None
    table: Optional[Dict] = None
    textlines: Optional[List[Dict]] = None

@dataclass
class PageLayout:
    page_number: int
    width, height: float
    boxes: List[LayoutBox]
    full_ocred: bool = False
    text_ocred: bool = False
    fulltext: Optional[List[Dict]] = None
    words: Optional[List[Dict]] = None
    links: Optional[List[Dict]] = None

@dataclass
class ParsedDocument:
    filename: Optional[str]
    page_count: int
    toc: Optional[List[List]]
    pages: List[PageLayout]
    metadata: Optional[Dict]
    # ... conversion methods
```

### Critical Algorithms

#### 1. List Item Hierarchy Detection (document_layout.py:97-151)

**Function**: `create_list_item_levels(layout_info) -> dict`

**Algorithm**:

```
1. Create segments of contiguous list items
   - Non-list-item finishes current segment
   - Different columns end segment
2. Sort each segment by x0 (left coordinate)
3. Assign levels:
   - First item: level 1
   - If x0 > prev_x0 + 10: increase level
   - Otherwise: same level as previous
4. Return: {bbox_index: level}
```

**Why This Matters**: Enables proper nested list rendering with correct indentation.

**Current Gap**: edgequake-pdf has no equivalent. All lists are flat.

#### 2. Styled Text Extraction (document_layout.py:355-416)

**Function**: `get_styled_text(spans) -> (text, suffix)`

**Algorithm**:

```
1. For each span, decode font properties:
   - superscript = flags & 1
   - mono = flags & 8 (and not OCR font)
   - bold = flags & 16 or char_flags & 8
   - italic = flags & 2
   - strikeout = char_flags & 1

2. Build markdown prefix/suffix:
   if mono: prefix = "`" + prefix
   if bold: prefix = "**" + prefix
   if italic: prefix = "_" + prefix
   if strikeout: prefix = "~~" + prefix
   suffix = reversed(prefix)

3. Handle style continuity:
   - If output ends with suffix, remove it
   - Resolve hyphenation across lines
   - Handle superscript spacing

4. Return: (styled_text, suffix)
```

**Why This Matters**: Preserves document styling accurately in Markdown.

**Current Gap**: edgequake-pdf has basic bold/italic but no strikeout, proper hyphenation, or superscript.

#### 3. Monospace Detection (document_layout.py:154-169)

**Function**: `is_monospaced(textlines) -> bool`

**Algorithm**:

```
1. For each line in textlines:
   - Check if ALL spans have flags & 8 (monospace)
   - AND font != OCR_FONTNAME
2. Return True if ALL lines are monospace
```

**Why This Matters**: Identifies code blocks automatically for proper rendering.

**Current Gap**: edgequake-pdf has this partially but not as comprehensive.

#### 4. PUA Character Handling (document_layout.py:83-94)

**Function**: `omit_if_pua_char(text) -> str`

**Algorithm**:

```
Check if single character is in Private Use Area:
- 0xE000 - 0xF8FF  (BMP PUA)
- 0xF0000 - 0xFFFFD  (Supplementary PUA-A)
- 0x100000 - 0x10FFFD (Supplementary PUA-B)

If in PUA: return ""
Else: return text
```

**Why This Matters**: PDFs often use PUA for custom symbols/bullets. Removing them prevents garbage output.

**Current Gap**: edgequake-pdf has no PUA detection.

#### 5. OCR Integration (document_layout.py:940-988)

**Algorithm**:

```
1. should_ocr_page(page, dpi, blocks):
   - Analyze text density
   - Check for existing OCR text
   - Decide: full-page OCR vs text-only OCR vs none

2. Full-page OCR:
   - Render page to pixmap
   - Run Tesseract OCR
   - Create temporary OCR PDF
   - Extract text from OCR PDF
   - Copy text layer to original page

3. Text-only OCR:
   - Repair existing blocks with OCR
   - Enhance poor-quality text

4. Set page_full_ocred / page_text_ocred flags
   - Used to disable code styling on OCR'd pages
```

**Why This Matters**: Handles scanned PDFs and poor-quality text extraction gracefully.

**Current Gap**: edgequake-pdf has no OCR integration at all.

#### 6. Table Structure Completion (document_layout.py:1003-1012)

**Algorithm**:

```
if tables_exist and not page_full_ocred:
    all_lines, all_boxes = utils.complete_table_structure(page)

tbf = page.find_tables(
    strategy="lines_strict",
    add_lines=all_lines,
    add_boxes=all_boxes
)
```

**Why This Matters**: Uses vector graphics (lines, rectangles) to improve table boundary detection.

**Current Gap**: edgequake-pdf has table detection but no structure completion.

---

## Test Infrastructure

### Current Tests (edgequake/crates/edgequake-pdf/tests/)

```
tests/
├── basic_features.rs         # Smoke tests
├── comprehensive_quality.rs  # Full quality suite (slow)
├── quality_evaluation.rs     # Metrics calculation
├── integration_tests.rs      # End-to-end tests
├── layout_test.rs            # Layout algorithm tests
├── micro_*.rs               # Focused micro-tests
└── debug_*.rs               # Debug utilities
```

**Test Execution Strategy** (Cargo.toml:28-37):

- Fast: `cargo test` (smoke only, <5s)
- Medium: `cargo test --features slow-tests` (<30s)
- Full: `cargo test --all-features` (2+ min)

**Test Data**: `edgequake/crates/edgequake-pdf/test-data/` (53 files observed)

### Quality Metrics Available

From integration_tests.rs and quality_evaluation.rs:

- Text extraction accuracy (character-level)
- Structure preservation (headers, lists, tables)
- Reading order correctness
- Table detection rate
- Style preservation (bold, italic, code)

---

## Dependencies Analysis

### Shared Dependencies

Both use PyMuPDF/PDFium for character extraction:

- **pymupdf4llm**: Python bindings to MuPDF C library
- **edgequake-pdf**: Rust bindings to PDFium C library (via pdfium-render)

**Key Difference**: MuPDF vs PDFium backends

- Both provide accurate character-level extraction
- PDFium has better font flag support (Cargo.toml comment: "F1 >= 0.95")
- MuPDF has native `get_layout()` method for structure detection

### Unique to pymupdf4llm

1. **pymupdf.get_layout()** - Native layout analysis from MuPDF
2. **Tesseract OCR** - Full-page and text-only OCR
3. **tabulate** - Table rendering library
4. **utils helpers** - Table extraction, form fields, image orphans

### Unique to edgequake-pdf

1. **R-tree spatial indexing** (rstar crate) - O(log n) queries
2. **Rayon parallelism** - Multi-core extraction
3. **ttf-parser** - TrueType cmap table extraction for subset fonts
4. **Async/await** - Tokio-based async pipeline

---

## Priority Gaps Identified

### High Impact (P0)

1. **List Item Hierarchy**
   - Current: Flat lists only
   - Gold: 3-level nested lists with proper indentation
   - File: pymupdf_grouper.rs needs hierarchy detection
   - Reference: document_layout.py:97-151

2. **Style Preservation**
   - Current: Bold, italic, mono only
   - Gold: + Superscript, strikeout, PUA handling
   - File: pymupdf_renderer.rs needs extended styling
   - Reference: document_layout.py:355-416

3. **Block Type Coverage**
   - Current: 5 types (Header, Code, ListItem, Table, Paragraph)
   - Gold: 10+ types (+ Footnote, Caption, Title, PageHeader, etc.)
   - File: pymupdf_structs.rs needs extended BlockType enum
   - Reference: document_layout.py:596-625

### Medium Impact (P1)

4. **Table Structure Completion**
   - Current: Basic table detection
   - Gold: Vector graphics-enhanced boundaries
   - File: New module needed
   - Reference: document_layout.py:1003-1012

5. **Hyphenation Resolution**
   - Current: No hyphen joining
   - Gold: Joins "exam-\nple" → "example"
   - File: pymupdf_grouper.rs line grouping
   - Reference: document_layout.py:204-205, 401-407

6. **Reading Order Enhancement**
   - Current: XY-cut algorithm
   - Gold: Integrated with MuPDF's find_reading_order
   - File: reading_order.rs needs refinement
   - Reference: document_layout.py:998-1000

### Lower Impact (P2)

7. **OCR Integration**
   - Current: None
   - Gold: Full-page and text-only OCR with Tesseract
   - File: New ocr module needed
   - Reference: document_layout.py:940-988

8. **Page Header/Footer Filtering**
   - Current: Not implemented
   - Gold: Detects and optionally omits headers/footers
   - File: pymupdf_grouper.rs classification
   - Reference: document_layout.py:675-681

---

## Metrics Baseline

### Need to Establish

Before improvements, run comprehensive tests to establish baseline:

```bash
cd edgequake/crates/edgequake-pdf
cargo test --all-features 2>&1 | tee baseline_results.txt
```

**Expected Metrics**:

- Text extraction accuracy: ~95% (per Cargo.toml comments)
- Table detection rate: Unknown
- Reading order accuracy: Unknown
- Structure preservation: Unknown

**Action Item**: Run baseline tests in Act phase.

---

## Web Research: First Principles

### PDF Structure Fundamentals

Researched: PDF specification basics, text extraction challenges

**Key Insights**:

1. **PDF Text Model**: PDFs store text as positioned characters, not flowing text
   - No inherent paragraph/list/table structure
   - Reading order is NOT guaranteed by file order
   - Layout analysis is inference, not extraction

2. **Font Challenges**:
   - Subset fonts: Custom encoding, requires cmap table parsing
   - Type3 fonts: Bitmap glyphs, no Unicode mapping
   - Symbol fonts: Non-standard character codes
   - → **First Principle**: Always parse font cmap tables when available

3. **Spatial Reasoning**:
   - Text position (x, y) is ground truth
   - Bounding boxes determine containment and adjacency
   - → **First Principle**: Spatial queries >> file order

4. **Style Detection**:
   - Font descriptor flags: Bold, Italic, FixedPitch
   - Font name heuristics: "Bold", "Italic" in name
   - Font size ratios: Headers typically 1.2-1.5x body text
   - → **First Principle**: Combine multiple signals, don't rely on single indicator

---

## Summary: Key Observations

### Strengths to Preserve

1. ✅ PDFium backend provides accurate character extraction
2. ✅ R-tree spatial indexing enables efficient queries
3. ✅ Modular architecture allows incremental improvements
4. ✅ Comprehensive test infrastructure exists

### Critical Gaps to Address

1. ❌ List hierarchy detection completely missing
2. ❌ Limited style preservation (no superscript, strikeout, PUA)
3. ❌ Block type coverage insufficient (5 vs 10+ types)
4. ❌ No OCR integration for scanned PDFs
5. ❌ Hyphenation not resolved across line breaks
6. ❌ Table structure completion not implemented

### Gold Standard Features to Port

From `pymupdf4llm/helpers/document_layout.py`:

1. **create_list_item_levels** (lines 97-151) → Hierarchy detection
2. **get_styled_text** (lines 355-416) → Enhanced styling
3. **omit_if_pua_char** (lines 83-94) → PUA filtering
4. **is_superscripted** (lines 172-184) → Footnote detection
5. **complete_table_structure** (via utils) → Vector graphics tables
6. **should_ocr_page** (lines 940-988) → OCR integration

---

## Next Steps

Will document in **orient.md**:

- Root cause analysis of gaps
- First Principles design for solutions
- Risk/benefit assessment of approaches
- Implementation strategy prioritization

---

**Verification Checklist**:

- [x] Mission file re-read documented
- [x] Current architecture mapped
- [x] Gold standard algorithms identified
- [x] Dependencies analyzed
- [x] Priority gaps listed with references
- [x] Web research for First Principles
- [x] No assumptions - all observations verified against code
