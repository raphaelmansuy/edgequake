# Mission: Pure Rust PDF Extraction Pipeline with pdfium-render

## Task

Your mission is to implement a completely new PDF-to-Markdown extraction pipeline using **pure Rust** with **pdfium-render** as the PDF backend. The goal is to achieve F1 >= 0.95 against pymupdf4llm gold standards by implementing pymupdf4llm's core algorithms in Rust.

## Context

- **Location**: `edgequake/crates/edgequake-pdf/src/`
- **Gold Standards**: `edgequake/crates/edgequake-pdf/test-data/real_dataset/*.pymupdf.gold.md`
- **Reference Implementation**: `zz-explore/pymupdf4llm/pymupdf4llm/pymupdf4llm/helpers/`
- **Current Baseline**: F1 = 0.685 (average across 7 documents)
- **Target**: F1 >= 0.95
- **PDF Backend**: `pdfium-render` v0.8.37 (MIT/Apache-2.0)

---

## CRITICAL CONSTRAINTS

1. **PURE RUST ONLY** - No Python, no subprocess, no FFI to Python
2. **PERMISSIVE LICENSE ONLY** - MIT or Apache-2.0 (no AGPL, no GPL)
3. **RE-READ THIS MISSION FILE AT THE START OF EVERY OODA ITERATION**

---

## Problem Analysis

### Current Issues (F1 = 0.685)

1. **Text Position Errors**: Our lopdf-based extraction produces different X/Y coordinates than pymupdf
2. **Line Grouping Too Loose**: Y-tolerance of `font_size * 0.5` merges consecutive lines
3. **Reading Order Broken**: Multi-column text gets interleaved incorrectly
4. **Word Merging Bugs**: Elements from different visual lines create garbage like "candatasets"

### Why pdfium-render is the Solution

pdfium-render wraps Google's PDFium (Chromium's PDF engine) which provides:

- Accurate character positioning via `PdfPageTextChar::bounds()` and `origin()`
- Font information via `font_size()`, `font_name()`
- Proper text matrix computation
- Robust handling of PDF quirks
- **MIT OR Apache-2.0 license** (permissive, commercial-friendly)

### License Comparison

| Criteria           | lopdf (current) | pdfium-render                | mupdf-rs            |
| ------------------ | --------------- | ---------------------------- | ------------------- |
| License            | MIT             | **MIT OR Apache-2.0**        | AGPL-3.0 (REJECTED) |
| Text positions     | Inaccurate      | Accurate (PDFium)            | Accurate            |
| Character bounds   | No              | `PdfPageTextChar::bounds()`  | Yes                 |
| Font info          | Limited         | `font_size()`, `font_name()` | Yes                 |
| Pure Rust API      | Yes             | Yes                          | Yes                 |
| Active maintenance | Yes             | 595 stars, 230 dependents    | Yes                 |

---

## Key pdfium-render APIs

```rust
use pdfium_render::prelude::*;

// Load PDF
let pdfium = Pdfium::default();
let document = pdfium.load_pdf_from_file("file.pdf", None)?;

// Iterate pages
for page in document.pages().iter() {
    let text = page.text()?;

    // Character-by-character with exact positions
    for char in text.chars() {
        let bounds = char.bounds();  // PdfRect with x0, y0, x1, y1
        let origin = char.origin();  // PdfPoint
        let font_size = char.font_size();
        let text = char.text();      // The actual character
    }
}
```

---

## Architecture

```
PDF Input
    |
    v
+------------------------+
| PdfiumBackend          |  <-- NEW: Uses pdfium-render for accurate extraction
| - load_document()      |
| - extract_page_chars() |      Returns: Vec<RawChar> with exact positions
| - get_page_size()      |
+------------------------+
    |
    v
+------------------------+
| Layout Analysis        |  <-- Port from pymupdf4llm algorithms
| - vertical_join()      |      (multi_column.py, get_text_lines.py)
| - boundary_normalize() |
| - smart_sort()         |
+------------------------+
    |
    v
+------------------------+
| Markdown Renderer      |  <-- Existing, minimal changes
| - headers              |
| - lists                |
| - tables               |
+------------------------+
    |
    v
Markdown Output
```

### RawChar Structure

```rust
pub struct RawChar {
    pub char: char,
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub font_size: f32,
    pub font_name: Option<String>,
    pub page_num: usize,
}
```

### PdfBackend Trait

```rust
trait PdfBackend {
    fn extract_page_chars(&self, page_num: usize) -> Result<Vec<RawChar>>;
    fn page_size(&self, page_num: usize) -> (f32, f32);
    fn page_count(&self) -> usize;
}
```

---

## Runtime Dependency: libpdfium

pdfium-render requires the PDFium dynamic library at runtime:

- **macOS**: `libpdfium.dylib`
- **Linux**: `libpdfium.so`
- **Windows**: `pdfium.dll`

Pre-built binaries available at: https://github.com/bblanchon/pdfium-binaries/releases

Download approach:

```bash
# macOS (arm64)
curl -L -o pdfium-mac-arm64.tgz \
  "https://github.com/bblanchon/pdfium-binaries/releases/latest/download/pdfium-mac-arm64.tgz"
tar -xzf pdfium-mac-arm64.tgz
export PDFIUM_DYNAMIC_LIB_PATH="$(pwd)/lib/libpdfium.dylib"
```

---

## Layout Analysis (pymupdf4llm Algorithms)

### Phase 1: Vertical Join (tolerance = 10pt)

```
delta = (0, 0, 0, 10)  # allow 10pt gap below
for each rect pair:
    if (rect0 + delta) intersects rect1:
        join rects
```

### Phase 2: Boundary Normalization (tolerance = 3pt)

```
for each rect:
    x0 = min([r.x0 for r in rects if |r.x0 - rect.x0| <= 3])
    x1 = max([r.x1 for r in rects if |r.x1 - rect.x1| <= 3])

sort by (x0, y0)
join rects with similar borders and close Y
```

### Phase 3: Smart Sort Key

```
       Q +---------+
         | next is |
   P +-------+   |  this   |
     | left  |   |  block  |
     | block |   +---------+
     +-------+

For block Q:
  - Find P (left-most block with vertical overlap)
  - Sort key = (P.y0, Q.x0)
  - This ensures Q comes after P in reading order
```

---

## Key Constants (from pymupdf4llm)

| Constant                       | Value         | WHY (First Principles)                                              |
| ------------------------------ | ------------- | ------------------------------------------------------------------- |
| `VERTICAL_JOIN_TOLERANCE`      | 10pt          | Typical line spacing is 10-12pt; gaps < 10pt are within same region |
| `BOUNDARY_ALIGNMENT_TOLERANCE` | 3pt           | PDF coordinates vary by 1-3pt due to rounding                       |
| `LINE_TOLERANCE`               | 3pt           | Spans on same line have Y within 3pt                                |
| `WORD_JOIN_THRESHOLD`          | 10% font_size | Spans closer than 10% of font size should join                      |

---

## Implementation Plan

### Phase A: Backend Integration (OODA 1-10)

1. Add pdfium-render to Cargo.toml
2. Download libpdfium for macOS/Linux
3. Create `PdfiumBackend` struct implementing extraction
4. Create test harness comparing old vs new extraction

### Phase B: Layout Algorithms (OODA 11-30)

5. Implement Phase 1 vertical join
6. Implement Phase 2 boundary normalization
7. Implement Phase 3 smart sort key
8. Implement `get_raw_lines()` from `get_text_lines.py`
9. Implement `column_boxes()` from `multi_column.py`

### Phase C: Integration (OODA 31-40)

10. Wire new layout into existing pipeline
11. Create fallback to lopdf for environments without libpdfium
12. Add configuration for backend selection

### Phase D: Validation (OODA 41-50)

13. Run F1 comparison on all 7 gold standards
14. Fix edge cases and regressions
15. Document the new pipeline
16. Create benchmarks

---

## Success Metrics

| Metric                | Baseline | Target  | Status        |
| --------------------- | -------- | ------- | ------------- |
| Average F1            | 0.685    | >= 0.95 | [In Progress] |
| 01_2512.25075v1       | 0.552    | >= 0.90 | [Pending]     |
| one_tool_2512.20957v2 | 0.596    | >= 0.90 | [Pending]     |
| AlphaEvolve           | 0.620    | >= 0.90 | [Pending]     |
| v2_2512.25072v1       | 0.689    | >= 0.90 | [Pending]     |
| 2900_Goyal_et_al      | 0.722    | >= 0.90 | [Pending]     |
| ccn_2512.21804v1      | 0.807    | >= 0.90 | [Pending]     |
| agent_2510.09244v1    | 0.810    | >= 0.90 | [Pending]     |

---

## Process: OODA Loop (50 iterations minimum)

Execute iterative OODA cycles. Each iteration produces 4 files:

Mission file: `specs/005-perfect-pdf-pymupdf4llm-inspired-conversion.md`

```
005-perfect-pdf-pymupdf4llm-inspired-conversion/ooda_loop/
├── iteration_01/
│   ├── observe.md   # Data gathered: code, business rules, workflows
│   ├── orient.md    # Analysis of findings vs. current docs
│   ├── decide.md    # Prioritized action plan
│   └── act.md       # Changes made, with file:line references + commit hashes
├── iteration_02/
│   └── ...
└── summary.md       # Cross-iteration insights
```

### Per-Iteration Requirements

| Step        | Output                                                     |
| ----------- | ---------------------------------------------------------- |
| **Observe** | Code analysis, feature inventory, dependency mapping       |
| **Orient**  | Gap analysis, documentation quality assessment             |
| **Decide**  | Specific changes prioritized by signal value               |
| **Act**     | Implementation with commit (`OODA-XX: <decision summary>`) |

### Constraints

1. **Re-read mission** every iteration: `specs/005-perfect-pdf-pymupdf4llm-inspired-conversion.md`
2. **Continue** from existing iterations—never restart
3. **Reference** real code: file paths, line numbers, commit SHAs
4. **Use ASCII diagrams** for architecture/flow visualization
5. **Split large files** for maintainability, Use Single Responsibility Principle (SRP)
6. **Optimize** Rust build speed (latest toolchain)
7. **Document amendments** in WHY, high signal value, and precise terms in comments
8. **You must perform tests** and deliver evidence that all tests are passing

---

## Reference: pymupdf4llm Source Files

| File                 | Purpose             | Key Functions                               |
| -------------------- | ------------------- | ------------------------------------------- |
| `multi_column.py`    | Column detection    | `column_boxes()`, `join_rects_phase1/2/3()` |
| `get_text_lines.py`  | Line grouping       | `get_raw_lines()`, `sanitize_spans()`       |
| `document_layout.py` | Layout analysis     | `IdentifyHeaders`, `make_blocks()`          |
| `pymupdf_rag.py`     | Fallback extraction | `to_markdown()`                             |
| `utils.py`           | Helpers             | `is_white()`, `outside_bbox()`              |

---


VERY IMPORTANT ENSURE the metrics used really reflect quaility of extraction compared to pymupdf4llm gold standards. Use First Principles to design the best metrics possible. What about Rouge / bleu / words difference / structural similarity, etc.

When implementing, prioritize correctness and maintainability over micro-optimizations. Focus on clear, well-documented code that accurately replicates pymupdf4llm's algorithms. zz-explore/pymupdf4llm is your reference for algorithmic behavior. If you have better ideas that improve quality while adhering to constraints, document them thoroughly in the OODA loop.

Important:

Ensure to clean old implementations and document all changes thoroughly when pdfium-render is fully integrated.

## Changelog

| OODA | Date       | Change                               | F1 Impact                 |
| ---- | ---------- | ------------------------------------ | ------------------------- |
| 01   | 2026-02-04 | Initial mission creation             | Baseline: 0.685           |
| 01   | 2026-02-04 | Changed to pure Rust + pdfium-render | N/A (architecture change) |
