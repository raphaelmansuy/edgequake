# Mission: Pure Rust PDF Extraction Pipeline with pdfium-render

## Task

Your mission is to implement a completely new PDF-to-Markdown extraction pipeline using **pure Rust** with **pdfium-render** as the PDF backend. The goal is to achieve Quality >= 0.95 against pymupdf4llm gold standards by implementing pymupdf4llm's core algorithms in Rust.

Always compare with pymupdf4llm outputs to ensure fidelity. Ensure to check algorithm details in `zz-explore/pymupdf4llm/pymupdf4llm/helpers/`. Eliminate other backends such as lopdf.

Accelerate the tests in order to validate progress quickly and speedup OODA iterations. The full test is very slow.

Use first principles to guide design decisions, always check pymupdf4llm behavior, and isolate issues with micro-tests. speed up with micro tests to learn about algorithmic issues quickly.

Create dedicated micro-tests for each algorithmic component to isolate issues. (VERY IMPORTANT)


ALWAYS CHALLENGE OUR ALGORITHMS using first principles against pymupdf4llm behavior: for example italic, bold, font detections, etc, layout, etc ....  If you find better approaches that improve quality while adhering to constraints, document them thoroughly in the OODA loop.

FULLY Read THIS MISSION FILE at the start of every OODA iteration.

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

## Quality Metrics (Multi-Dimensional Evaluation)

### CRITICAL: Why Word F1 Alone is Misleading

The original word-set F1 metric has fatal flaws:

1. **SET ignores ORDER**: "the cat sat" vs "sat cat the" score identically
2. **SET ignores DUPLICATES**: Common words in academic papers are lost
3. **Strips markdown**: Doesn't verify headers, bold, italic preservation
4. **False confidence**: Old F1=0.877 vs Real Quality=0.573 (43% overestimated!)

### Comprehensive Quality Score (NEW - Feb 2025)

We now use a 4-dimensional evaluation:

```
QUALITY = 0.40×ROUGE-L + 0.30×Word_F1 + 0.15×Structure + 0.10×Format + 0.05×BLEU-4
```

#### Dimension 1: Content Accuracy (Word F1)

- **What**: Bag-of-words F1 with multiset (counts duplicates)
- **Captures**: Vocabulary coverage - are the right words present?
- **Weight**: 30% of quality score

#### Dimension 2: Order Preservation (ROUGE-L)

- **What**: Longest Common Subsequence F1
- **Captures**: Reading order - are words in correct sequence?
- **Weight**: 40% of quality score (MOST IMPORTANT)
- **Formula**: `ROUGE-L = F1(LCS/extracted, LCS/gold)`

#### Dimension 3: Structural Fidelity

- **What**: Heading count, paragraph count, line count ratios
- **Captures**: Document layout preservation
- **Weight**: 15% of quality score
- **Components**: `0.4×headings + 0.3×paragraphs + 0.3×lines`

#### Dimension 4: Formatting Fidelity

- **What**: Bold, italic, list marker count ratios
- **Captures**: Markdown formatting preservation
- **Weight**: 10% of quality score
- **Components**: `0.4×bold + 0.4×italic + 0.2×lists`

### Current Status (OODA-09)

| Metric            | Current | Target  | Gap    |
| ----------------- | ------- | ------- | ------ |
| **Quality Score** | 0.732   | >= 0.95 | -0.218 |
| ROUGE-L (order)   | 0.698   | >= 0.90 | -0.202 |
| Word F1 (content) | 0.893   | >= 0.95 | -0.057 |
| Structure Score   | 0.602   | >= 0.80 | -0.198 |
| Format Score      | 0.573   | >= 0.70 | -0.127 |

### Per-File Breakdown (OODA-09)

| File                  | Quality | ROUGE-L | Word F1 | Struct | Format |
| --------------------- | ------- | ------- | ------- | ------ | ------ |
| agent_2510.09244v1    | 0.874   | 0.932   | 0.934   | 0.951  | 0.354  |
| 2900_Goyal_et_al      | 0.861   | 0.939   | 0.939   | 0.535  | 0.795  |
| AlphaEvolve           | 0.801   | 0.851   | 0.873   | 0.602  | 0.700  |
| ccn_2512.21804v1      | 0.702   | 0.615   | 0.929   | 0.489  | 0.653  |
| one_tool_2512.20957v2 | 0.677   | 0.570   | 0.872   | 0.668  | 0.525  |
| 01_2512.25075v1       | 0.618   | 0.493   | 0.838   | 0.545  | 0.558  |
| v2_2512.25072v1       | 0.593   | 0.486   | 0.865   | 0.424  | 0.427  |

### Key Insight: Current Focus

**Format dimension improved significantly** (0.470→0.573, +22%) thanks to italic detection.

Remaining gaps:

1. ROUGE-L: -0.202 (reading order still the biggest issue)
2. Structure: -0.198 (headings/paragraphs/lines)
3. Format: -0.127 (closer to target now)

### Evaluation Script

Run comprehensive evaluation:

```bash
python3 scripts/eval_comprehensive.py           # All files
python3 scripts/eval_comprehensive.py --verbose  # With details
python3 scripts/eval_comprehensive.py -f Alpha   # Single file
```

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

Fully review how F1 score is computed and ensure it captures all aspects of quality. If needed, propose new metrics or adjustments to existing ones to better reflect extraction fidelity.

When implementing, prioritize correctness and maintainability over micro-optimizations. Focus on clear, well-documented code that accurately replicates pymupdf4llm's algorithms. zz-explore/pymupdf4llm is your reference for algorithmic behavior. If you have better ideas that improve quality while adhering to constraints, document them thoroughly in the OODA loop.

Important:

Ensure to clean old implementations and document all changes thoroughly when pdfium-render is fully integrated.

## Changelog

| OODA | Date       | Change                                              | Quality Impact                     |
| ---- | ---------- | --------------------------------------------------- | ---------------------------------- |
| 01   | 2026-02-04 | Initial mission creation                            | Baseline: Word F1=0.685            |
| 01   | 2026-02-04 | Changed to pure Rust + pdfium-render                | N/A (architecture change)          |
| 02   | 2026-02-04 | Space character synthesis                           | Word F1: 0.874→0.892 (+0.018)      |
| 03   | 2026-02-04 | **NEW METRICS**: Comprehensive Quality Score        | Revealed Quality=0.573 (was 0.877) |
| 03   | 2026-02-04 | Added ROUGE-L, BLEU-4, Structure, Format metrics    | Gap identified: 0.377 vs 0.073     |
| 04   | 2025-01-27 | **FIX**: Removed column gutter check from line join | Quality: 0.573→0.675 (+18%)        |
| 04   | 2025-01-27 | Line fragmentation fixed (9892→1814 lines)          | ROUGE-L: 0.491→0.702 (+43%)        |
| 05   | 2025-01-27 | Font style: Added "medi/semi/demi" bold detection   | Structure: 0.350→0.453 (+29%)      |
| 05   | 2025-01-27 | Header levels: Adjusted ratio thresholds            | Format: 0.343→0.470 (+37%)         |
| 05   | 2025-01-27 | Combined effect                                     | Quality: 0.675→0.702 (+4%)         |
| 06   | 2025-01-27 | Line breaks: Changed join(" ") to join("\\n")       | Structure: 0.453→0.602 (+33%)      |
| 06   | 2025-01-27 | Lines now match gold paragraph format               | Quality: 0.702→0.724 (+3%)         |
| 07   | 2025-01-28 | Author line rescue: Fragments at X>boundary rescued | Specific fix (no aggregate change) |
| 07   | 2025-01-28 | v2_2512: Jitendra Malik now block 2 (was block 10)  | Reading order fix for title pages  |
| 08   | 2026-02-04 | **FIX**: Update width during element merge          | Specific fix (test_qwen_reading)   |
| 08   | 2026-02-04 | Word splitting: "Push ing" → "Pushing" fixed        | test_qwen_reading_order PASS       |
| 08   | 2026-02-04 | Relaxed OODA-42 threshold (0.5× → 1× font_size)     | Handles width estimation error     |
| 09   | 2026-02-04 | Add "ital" pattern for Nimbus italic fonts          | Quality: 0.724→0.732 (+0.8%)       |
| 09   | 2026-02-04 | Re-enable "medi" bold detection (was disabled)      | Format: 0.470→0.573 (+22%)         |
| 09   | 2026-02-04 | v2_2512 Italic: 0% → 28.5%, Bold improved           | 2900_Goyal Format: +75%            |

---

## OODA Iterations 60-120: Comprehensive Quality Roadmap

### Guiding Principles

1. **Two Pipelines, One Goal**: PDFium (new, preferred) and lopdf (legacy, deprecated)
2. **Font Style Accuracy**: PDFium flags > font name patterns
3. **SRP**: Single Responsibility Principle - small, focused modules
4. **DRY**: Don't Repeat Yourself - shared utilities between pipelines
5. **Clean Code**: No clippy warnings, high-signal WHY comments, ASCII diagrams

---

### Phase A: Code Architecture Cleanup (OODA 60-70)

#### OODA-60: Unify Font Style Detection

\`\`\`text
┌─────────────────────────────────────────────────────────────────────────────┐
│                     FONT STYLE DETECTION CONSOLIDATION                       │
├
---

## OODA Iterations 60-120: Comprehensive Quality Roadmap

### Guiding Principles

1. **Two Pipelines, One Goal**: PDFium ???#─
### Guiding Principles

1. **Two Pipelines, One Goal**???1. **Two Pipelines, ???. **Font Style Accuracy**: PDFium flags > font name patterns
3. **SRP**: Single Resp  3. **SRP**: Single Responsibility Principle - small, focused  4. **DRY**: Don't Repeat Yourself - shared utilities between pipeli?. **Clean Code**: No clippy warnings, high-signal WHY comments, ASCI??
---

### Phase A: Code Architecture Cleanup (OODA 60-70)

#### OODA-60: Unify?  
#│
#### OODA-60: Unify Font Style Detection

\`\`\`t   
\`\`\`text
┌──────── is┌──  │                     FONT STYLE DETECTION CONSOLIDATION                       │
├
---

## OODA Iterations 60-120: Comprehensive Quality Roadmap

### Guiding Principles

1. **Two Pipelines, One Goal**: PDFium ???#─
### Guiding ??├
---

## OODA Iterations 60-120: Comprehensive Quality Roadmap

### Guiding Prin??--??#─
### Guiding Principles

1. **Two Pipelines, One Goal**?? 
1. **Two Pipelines,    ### Guiding Principles

1. **Two Pipelines, On  
1. **Two Pipelines, nif3. **SRP**: Single Resp  3. **SRP**: Single Responsibility Principle - small, focused  4. **DRY**: Don't Repeat Yours??---

### Phase A: Code Architecture Cleanup (OODA 60-70)

#### OODA-60: Unify?  
#│
#### OODA-60: Unify Font Style Detection

\`\`\`t   
\`\`\`text
┌──────── is┌──  │                     FONT STYLE ld
#lag
#### OODA-60: Unify?  
#│
#### OODA-60: Unify F   #│
#### OODA-60: Un- ###it
\`\`\`t   
\`\`\`text
┌────?  \`\`\`tex  ┌──  ├
---

## OODA Iterations 60-120: Comprehensive Quality Roadmap

### Guiding Principles

1. **Two Pipelines, One Goal**: Pum--nd
#opd
### Guiding Principles

1. **Two Pipelines, One Goal**???1. **Two Pipelines, ???## Guiding ??├
---

## OODA Iterations 60-1??--

## OODA Ite??#─
### Guiding Prin??--??#─
### Guiding Principles

1.   ### Guiding Principles

1.  
1. **Two Pipelines,    1. **Two Pipelines,    ### Guidi  
1. **Two Pipelines, On  
1. **Two Pipelines─1. **Two Pipelines, nif??### Phase A: Code Architecture Cleanup (OODA 60-70)

#### OODA-60: Unify?  
#│
#### OODA-60: Unify Font Style Detection

\`\`\`t   
\`\`\`tex???#### OODA-60: Unify?  
#│
#### OODA-60: Unify Fate#│
#### OODA-60: Unth###if
\`\`\`t   
\`\`\`text
┌────?DR\`\`\`texnc┌──te#lag
#### OODA-60: Unify?  
#│
#### OODA-60: Unify F   #│
#### OODA-60: U A###\`#│
#### OODA-60: Unpd###od#### OODA-60: Un- ###it
\`\**\`\`\`t   
\`\`\`text
ca\`\`\`texin┌──ra---

## OODA Iterations 60-120: Comprehensio 
#fiu
### Guiding Principles

1. **Two Pipelines, One Goal**
\`
1. **Two Pipelines, per#opd
### Guiding Principles

1. **Two ??##ch
1. **Two Pipelines, 0 l---

## OODA Iterations 60-1??--

## OODA Ite??#─
### Guiding Prin??-li
#s) 
## OODA Ite??#─
### Gui├### Guiding Prin?oc### Guiding Principles

1. g
1.   ### Guiding Pri???1.  
1. **Two Pipelines, ~151. in1. **Two Pipelines, On  
1. **Two Pipelines─1. **Two P.r1. **Two Pipelines─1.- 
#### OODA-60: Unify?  
#│
#### OODA-60: Unify Font Style Detection

\`\`\`t   
\`\`\`tex???##
- #│
#### OODA-60: Unin###na
\`\`\`t   
\`\`\`tex???#### OODA-60:  Sp\`\`\`texct#│
#### OODA-60: Unify Fate#│
tr###io#### OODA-60: Unth###if
\`\? \`\`\`t   
\`\`\`text
co\`\`\`texno┌──on#### OODA-60: Unify?  
#│
#### OODA-60: Un
?│
#### OODA-60: Unn.###  #### OODA-60: U A###\`#│
R #### OODA-60: Unpd###od## p\`\**\`\`\`t   
\`\`\`text
ca\`\`\`texin┌?g\`\`\`text
ca\tica\`\`\`t?## OODA Iterations 60-120   #fiu
### Guiding Principles

1. **Two Pin###??
1. **Two Pipelines,    \`
1. **Two Pipelines, per#opbl1c ### Guiding Principles

1.`

1. **Two ??##ch
1. ule1. **Two Pipeli c
## OODA Iterations 60-1?er 
## OODA Ite??#─
### Guiend### Guiding Prin?dd#s) 
## OODA Ite??# A## ri### Gui├### Gui: 
1. g
1.   ### Guiding Pri???1.  
1. **Two Pipeli**F1. s*1. **Two Pipelines, ~151. is 1. **Two Pipelines─1. **Two P.r1. **Two Pipelinesfu#### OODA-60: Unify?  
#│
#### OODA-60: Unify Font Stylou#│
#### OODA-60: Un: ###ou
\`\`\`t   
\`\`\`tex???##
- #│
####
- \`\`\`tex**- #│
#### OOit#### pl\`\`\`t   
\`\`\`tex?? F\`\`\`texoa#### OODA-60: Unify Fate#│
tr###io#### OODixtr###io#### OODA-60: Unth##co\`\? \`\`\`t   
\`\`\`text
co*F\`\`\`text
co\usco\`\`\`tle#│
#### OODA-60: Un
?│
#### OODA-60: Unnle###co?│
#### OODAno####
#R #### OODA-60: Unpd###od## p\`\**\`\`\`t   
\`\
-\`\`\`text
ca\`\`\`texin┌?g\`\`\`text
cs,ca\`\`\`t_fca\tica\`\`\`t?## OODA Iterse### Guiding Principles

1. **Two Pin###??
1. **T: 
1. **Two Pin###??
1.## 1. **Two Pipelint 1. **Two Pipelines, per#\`
1.`

1. **Two ??##ch
1. ule1. **Two Pipeli c
## OO???1─1. ule1. **Two ?# OODA Iterations 60-?# OODA Ite??#─
### Gui??### Guiend### Gui??# OODA Ite??# A## ri### Gui├?. g
1.   ### Guiding Pri???1.  
1. **T?. ??. **Two Pipeli**F1. s*1. *  #│
#### OODA-60: Unify Font Stylou#│
#### OODA-60: Un: ###ou
\`\`\`t   
\`\`\`tex???##
- #│
####
- \`\`\`tex**- #│
##??###?### OODA-60: Un: ###ou
\`\`\`t   ?`\`\`t   
\`\`\`tex???`\`\`tex?? #│
####
- ?###
??- \?### OOit#### pl\`??`\`\`tex?? F\`\`\`texoa#??r###io#### OODixtr###io#### OODA-60: Unth##co\`\?   \`\`\`text
co*F\`\`\`text
co\usco\`\`\`tle#│
#### OODA-60:?o*F\`\`\fEco\usco\`\`\`h_#### OODA-60: Un
?  ?│
#### OODA  ####  #### OODAno####
#R #### OODA??#R #### OODA-6  \`\
-\`\`\`text
ca\`\`\`texin┌?g\`\`\`te  -\  ca\`\`\`te  cs,ca\`\`\`t_fca\tica\`\`\`t? 
1. **Two Pin###??
1. **T: 
1. **Two Pin###??
1.## 1. **Two Pipeli─1. **T: 
1. **Tw??1. **Tw??1.## 1. **Two Pi??1.`

1. **Two ??##ch
1. ule1. **Two Pipeli c
##?1???. ule1. **Two   ## OO???1─1. ule1.  ?## Gui??### Guiend### Gui??# OODA Ite??# A## ri### Gui├?. g
1.    1.   ### Guiding Pri???1.  
1. **T?. ??. **Two Pipeli**F1. s*?. **T?. ??. **Two Pipel??### OODA-60: Unify Font Stylou#│
#### OODA??#### OODA-60: Un: ###ou
\`\`\`t    ?`\`\`t   
\`\`\`tex??  \`\`\`tex  - #│
####
-   ####
  - \  ##??###?### OODA  \`\`\`t   ?`\`\`t   
\`\`\`te  \`\`\`tex???`\`\`te  ####
- ?###
??- \?### OOi  - ? ??- \? co*F\`\`\`text
co\usco\`\`\`tle#│
#### OODA-60:?o*F\`\`\fEco\usco\`\`\`h_#### OODA-60: Un
?  ?│
#### OODA  #??o\usco\`\`\`  #### OODA-60:?o*F  ?  ?│
#### OODA  ####  #### OODAno####
#R #### OODA? ?### OOD  #R #### OODA??#R #### OODA-6  \ ?\`\`\`text
ca\`\`\`texin┌?g?a\`\`\`te?. **Two Pin###??
1. **T: 
1. **Two Pin###??
1.## 1. **Two Pipeli─1. **T: ??1. **T: 
1. **Tw??. **Tw??.## 1. **Two Pi??. **Tw??1. **Tw??1.## 1. **TwES
1. **Two ??##ch
1. ule1. **Two Pipel   1. ule1. **Two   ##?1???. ule1. **Tw  1.    1.   ### Guiding Pri???1.  
1. **T?. ??. **Two Pipeli**F1. s*?. **T?. ??. **Two Pipel??### OODA-60:?. **T?. ??. **Two Pipeli**F1.??#### OODA??#### OODA-60: Un: ###ou
\`\`\`t    ?`\`\`t   
\`\`\`tex??  \`\`\`tex  - #│
####
-   ###f \`\`\`t    ?`\`\`t   
\`\`\`tex?  \`\`\`tex??  \`\`\`te  ####
-   ####
  - \  ##??###??-  ac  - \  ??\`\`\`te  \`\`\`tex???`\`\`te  ####
- ?###
???- ?###
??- \?### OOi  - ? ??- \LE??- \└?co\usco\`\`\`tle#│
#### OODA-60:?o*F\`\`??#### OODA-60:?o*F???  ?│
#### OODA  #??o\usco\`\`\`  #### OODA-60:?o?### OOD??#### OODA  ####  #### OODAno####
#R #### OODA? ?### OOD    #R #### OODA? ?### OOD  #R ###  ca\`\`\`texin┌?g?a\`\`\`te?. **Two Pin###??
1. **T: 
1. **Tw  1. **T: 
1. **Two Pin###??
1.## 1. **Two Pipeli─??. **Tw??.## 1. **Two Pi??. **Tw??. **Tw??.## 1. **Two Pi??. *??. **Two ??##ch
1. ule1. **Two Pipel   1. ule1. **Two   ##?1?????. ule1. **Two ??1. **T?. ??. **Two Pipeli**F1. s*?. **T?. ??. **Two Pipel??### OODA-60:?. **T?. ??.me\`\`\`t    ?`\`\`t   
\`\`\`tex??  \`\`\`tex  - #│
####
-   ###f \`\`\`t    ?`\`\`t   
\`\`\`tex?  \`\`\`tex??  \`\`\`te  ####
-   ####
  - \  ##?- \`\`\`tex??  \`\`\`tefo####
-   ###f \`\`\`t    ?`\d -  se\`\`\`tex?  \`\`\`tex??  \`\`\ss-   ####
  - \  ##??###??-  ac  - \  Co  - \  n
- ?###
???- ?###
??- \?### OOi  - ? ??- \LE??- \└?co\usco\?????- └???- \?##??#### OODA-60:?o*F\`\`??#### OODA-60:?o*F???  ?│
#?### OODA  #??o\usco\`\`\`  #### OODA-60:?o?### O??R #### OODA? ?### OOD    #R #### OODA? ?### OOD  #R ###  ca\`\`\`texin┌?g?a\`\`\ P1. **T: 
1. **Tw  1. **T: 
1. **Two Pin###??
1.## 1. **Two Pipeli─??. **Tw??.## 1. **Two Pi??. **Tw??. *??. **Tw??. **Two Pin###???.## 1. **Two Pi??. ule1. **Two Pipel   1. ule1. **Two   ##?1?????. ule1. **Two ??1. **T?. ??. **Two Pipeli**F1. s*?.??`\`\`tex??  \`\`\`tex  - #│
####
-   ###f \`\`\`t    ?`\`\`t   
\`\`\`tex?  \`\`\`tex??  \`\`\`te  ####
-   ####
  - \  ##?- \`\`\`tex??  \`\`\`tefo####
-   ###f \`\`\`t    ?`\d -  se\  ####
-   ###f \`\`\`t    ?`\??  ??\`\`\`tex?  \`\`\`tex??  \`\`\?   ####
  - \  ##?- \`\`\`tex??  \`\`??  - \  ??-   ###f \`\`\`t    ?`\d -  se\`\`\`t?? - \  ##??###??-  ac  - \  Co  - \  n
- ?###
???- ?###
??- \?# F- ?###
???- ?###
??- \?### OOi  0 ???- ?o??- \?##or#?### OODA  #??o\usco\`\`\`  #### OODA-60:?o?### O??R #### OODA? ?### OOD    #R #### OODA? ?### OOD  #R ###  ca\`  1. **Tw  1. **T: 
1. **Two Pin###??
1.## 1. **Two Pipeli─??. **Tw??.## 1. **Two Pi??. **Tw??. *??. **Tw??. **Two Pin###???.## 1. **Two Pi??. ule1. ??1. **Two Pin###?  1.## 1. **Two Pihe####
-   ###f \`\`\`t    ?`\`\`t   
\`\`\`tex?  \`\`\`tex??  \`\`\`te  ####
-   ####
  - \  ##?- \`\`\`tex??  \`\`\`tefo####
-   ###f \`\`\`t    ?`\d -  se\  ####
-   ###f \`\`\`t    ?`\??  ??\`\`\`tex?  \`\`\`tex??  \`\`\?   ####
  - \  ##?- \`\?  ??`\`\`tex?  \`\`\`tex??  \`\`\??-   ####
  - \  ##?- \`\`\`tex??  \`\`?? - \  ??   ###f \`\`\`t    ?`\d -  se\  ####?   ###f \`\`\`t    ?`\??  ??\`\`\`? - \  ##?- \`\`\`tex??  \`\`??  - \  ??-   ###f \`\`\`t    ?`\d -  s b- ?###
???- ?###
??- \?# F- ?###
???- ?###
??- \?### OOi  0 ???- ?o??- \?##or#?### OODA  #??o\usco\ts???- ?1??- \?#  O???- ?###
??- \?d??- \?##et1. **Two Pin###??
1.## 1. **Two Pipeli─??. **Tw??.## 1. **Two Pi??. **Tw??. *??. **Tw??. **Two Pin###???.## 1. **Two Pi??. ule1. ??1. **Two Pin###?  1.## 1. **Two Pihe#### 11.## 1. **Two Pi):-   ###f \`\`\`t    ?`\`\`t   
\`\`\`tex?  \`\`\`tex??  \`\`\`te  ####
-   ####
  - \  ##?- \`\`\`tex??  \`\`\`tefo####
-   ###f \`\`\`t    ?`\d -  se\  ####
- at\`\`\`tex?  \`\`\`tex??  \`\`\o -   ####
  - \  ##?- \`\`\`tex??  \`\`de  - \  rc-   ###f \`\`\`t    ?`\d -  se\  ####St-   ###f \`\`\`t    ?`\??  ??\`\`\`72  - \  ##?- \`\?  ??`\`\`tex?  \`\`\`tex??  \`\`\??-   ####
  - \  In  - \  ##?- \`\`\`tex??  \`\`?? - \  ??   ###f \`\`\`t    ? ???- ?###
??- \?# F- ?###
???- ?###
??- \?### OOi  0 ???- ?o??- \?##or#?### OODA  #??o\usco\ts???- ?1??- \?#  O???- ?###
??- \?d??- \?##et1. **Two Pin###??
1.## 1. **Two Pior??- \?# en???- ?###
??- \?b??- \?##re??- \?d??- \?##et1. **Two Pin###??
1.## 1. **Two Pipeli─??. **Tw??.## 1. **Two Pi??. **Tw?m 1.## 1. **Two Pipeli─??. **Tw??.tu\`\`\`tex?  \`\`\`tex??  \`\`\`te  ####
-   ####
  - \  ##?- \`\`\`tex??  \`\`\`tefo####
-   ###f \`\`\`t    ?`\d -  se\  ####
- at\`\`\`tex?  \`\`\`tex??  \`\`\o -   ####
  - \  ##?- \`\`\`tex??  \`\`de  - \  rcir-   ####
  - \  ##?- \`\`\`tex??  \`\`ns  - \    -   ###f \`\`\`t    ?`\d -  se\  ####ub- at\`\`\`tex?  \`\`\`tex??  \`\`\o -o)  - \  ##?- \`\`\`tex??  \`\`de  - \  rc-  ct  - \  In  - \  ##?- \`\`\`tex??  \`\`?? - \  ??   ###f \`\`\`t    ? ???- ?###
??- \?# F- ?###
???- ?###
??- \?### OOi  0 ???- ?o??- \?##or#?### OODA  #??o\uscodj??- \?# F- ?###
???- ?###
??- \?### OOi  0 ???- ?o??- \?##or#?### OO-7???- ?###
??- \?e??- \?##sk??- \?d??- \?##et1. **Two Pin###??
1.## 1. **Two Pior??- \?# en???- ?###
??- \?b??- \?##CT1.## 1. **Two Pior??- \?# en???- ?s??- \?b??- \?##re??- \?d??- \?##etnt1.## 1. **Two Pipeli─??. **Tw??.## 1. **Two Pi??. *##-   ####
  - \  ##?- \`\`\`tex??  \`\`\`tefo####
-   ###f \`\`\`t    ?`\d -  se\  ####
- at\`\`\`tex?  \`\`\`tex??  \`\`\o -   ####
  - \CB  - \  
 -   ###f \`\`\`t    ?`\d -  se\  #### -- at\`\`\`tex?  \`\`\`tex??  \`\`\o -? - \  ##?- \`\`\`tex??  \`\`de  - \  rcir-20  - \  ##?- \`\`\`tex??  \`\`ns  - \    -   ###f  *??- \?# F- ?###
???- ?###
??- \?### OOi  0 ???- ?o??- \?##or#?### OODA  #??o\uscodj??- \?# F- ?###
???- ?###
??- \?### OOi  0 ???- ?o??- \?##or#?### OO-7???- ?###
??- \?e??- \?##sk??- \?d??- \?##et1. **Two Pinyp???- ?###
??- \?i??- \?##`\???- ?###
??- \?### OOi  0 ???- ?o??- \?##or#?### OO-7???- ?###
??- \?e O??- \?##ot??- \?e??- \?##sk??- \?d??- \?##et1. **Two Pin###??
1.##- 1.## 1. **Two Pior??- \?# en???- ?###
??- \?b??- \?I??- \?b??- \?##CT1.## 1. **Two Pior??##  - \  ##?- \`\`\`tex??  \`\`\`tefo####
-   ###f \`\`\`t    ?`\d -  se\  ####
- at\`\`\`tex?  \`\`\`tex??  \`\`\o -   ####
  - \CB  - \  
 -   ###f \`\`\`t    ?`\dru-   ###f \`\`\`t    ?`\d -  se\  #### A- at\`\`\`tex?  \`\`\`tex??  \`\`\o -m*  - \CB  - \  
 -   ###f \`\`\`t    ?`\d - * -   ###f \`\De???- ?###
??- \?### OOi  0 ???- ?o??- \?##or#?### OODA  #??o\uscodj??- \?# F- ?###
???- ?###
??- \?### OOi  0 ???- ?o??- \?##or#?### OO-7???- ?###
??- \?e??- \?##sket??- \?##ac???- ?###
??- \?### OOi  0 ???- ?o??- \?##or#?### OO-7???- ?###
??- \?eOD??- \?##i-??- \?e??- \?##sk??- \?d??- \?##et1. **Two Pinyp???- ??─- \?i??- \?##`\???- ?###
??- \?### OOi  0 ???- ?o??-  1??- \?### OOi  0 ???- ?o??ad??- \?e O??- \?##ot??- \?e??- \?##sk??- \?d??- \?##et? 1.##- 1.## 1. **Two Pior??- \?# en???- ?###
??- \?b??- \?I??- \?b??- \─??- \?b??- \?I??- \?b??- \?##CT1.## 1. **??-   ###f \`\`\`t    ?`\d -  se\  ####
- at\`\`\`tex?  \`\`\`tex??  \`\`\o -   ####
  - \CB  - \OU- at\`\`\`tex?  \`\`\`tex??  \`\`\o -e   - \CB  - \  
 -   ###f \`\`\`t    ?`\dru:  -   ###f \`\in -   ###f \`\`\`t    ?`\d - * -   ###f \`\De???- ?###
??- \?### OOi  0 ???- ?o??- \?##or#?### OODA  #??o\usc# ??- \?### OOi  0 ???- ?o??- \?##or#?### OODA  #: ???- ?###
??- \?### OOi  0 ???- ?o??- \?##or#?### OO-7???- ?###
??- \?e_7??- \?## }??- \?e??- \?##sket??- \?##ac???- ?###
??- \?### OOi es??- \?### OOi  0 ???- ?o??- \?##or#? }??- \?eOD??- \?##i-??- \?e??- \?##sk??- \?d??- \?##etal??- \?### OOi  0 ???- ?o??-  1??- \?### OOi  0 ???- ?o??ad??- \?e O??- \?##ot??- \?e??- \?##sk??- \?des??- \?b??- \?I??- \?b??- \─??- \?b??- \?I??- \?b??- \?##CT1.## 1. **??-   ###f \`\`\`t    ?`\d -  se\  ####
- at\`\`\`tex?  \`\`\`tex??  \`\`\o -   ####
  - \CB  - \ad- at\`\`\`tex?  \`\`\`tex??  \`\`\o -   ####
  - \CB  - \OU- at\`\`\`tex?  \`\`\`tex??  \`\`\o -e   - \CB  - \  
 -  f   - \CB  - \OU- at\`\`\`tex?  \`\`\`tex??  ui -   ###f \`\`\`t    ?`\dru:  -   ###f \`\in -   ###f \`\`\`t    ha??- \?### OOi  0 ???- ?o??- \?##or#?### OODA  #??o\usc# ??- \?### OOi  0 ???- ?o??- \?a??- \?### OOi  0 ???- ?o??- \?##or#?### OO-7???- ?###
??- \?e_7??- \?## }??- \?e??- \?##sket??- \?##ac???- ?###
-9??- \?e_7??- \?## }??- \?e??- \?##sket??- \?##ac???- ?r??- \?### OOi es??- \?### OOi  0 ???- ?o??- \?##or#? }??%
- at\`\`\`tex?  \`\`\`tex??  \`\`\o -   ####
  - \CB  - \ad- at\`\`\`tex?  \`\`\`tex??  \`\`\o -   ####
  - \CB  - \OU- at\`\`\`tex?  \`\`\`tex??  \`\`\o -e   - \CB  - \  
 -  f   - \CB  - \OU- at\`\`\`tex?  \`\`\`tex??  ui -   ###f \`\`\`t    ?`\dru:  -   ###f \`\in -   ###f \`\`\`t    ha??- \?### OOi  0 ???- ?o??- \?##or#?### OODA  #??o\usc# ??- \?9  - \CB  - \ad- at\`\`\`tex?  \`\`\`tex??  :   - \CB  - \OU- at\`\`\`tex?  \`\`\`tex??  \`\`\o -e   - dl -  f   - \CB  - \OU- at\`\`\`tex?  \`\`\`tex??  ui -   ###f \`\`\DA??- \?e_7??- \?## }??- \?e??- \?##sket??- \?##ac???- ?###
-9??- \?e_7??- \?## }??- \?e??- \?##sket??- \?##ac???- ?r??- \?### OOi es??- \?### OOi  0 ???- ?o??- \?##or#? }??%
- at\`\`\`tex?  \`\`\`tex??  \`\`\o -   ####
  - \CB  - \ad- at\`\`\`tex?  \`\`\`tr-9??- \?e_7??- \?## }??- \?e??- \?##sket??- \?##ac???- ?r**- at\`\`\`tex?  \`\`\`tex??  \`\`\o -   ####
  - \CB  - \ad- at\`\`\`tex?  \`\`\`tex??  \`\`\o -   ####
  - \CB  - \OU- at\`\`\`tex?rr  - \CB  - \ad- at\`\`\`tex?  \`\`\`tex??  ge  - \CB  - \OU- at\`\`\`tex?  \`\`\`tex??  \`\`\o -e   -  R -  f   - \CB  - \OU- at\`\`\`tex?  \`\`\`tex??  ui -   ###f \`\`\cu-9??- \?e_7??- \?## }??- \?e??- \?##sket??- \?##ac???- ?r??- \?### OOi es??- \?### OOi  0 ???- ?o??- \?##or#? }??%
- at\`\`\`tex?  \`\`\`tex??  \`\`\o -   ####
  - \CB  - \ad- at\`\`\`tex?  \`\`\`tr-9??- \?e_7??- \?## }??- \?e??- \?##sket??- \?##ac???- ?r**- at\`\`\`tex?  \`\`\`tex??  \`\`\o -   ####
  - \CB  - \ad- at\`\`\`tex?  \`\`\`tex??  \`\`\o -   ####
  - \CB  - \OU- at\`\`\`tex?rr  - \CB  - \ti- at\`\`\`tex?  \`\`\`tex??  \`\`\o -   ####
  - \CB  - \ad- at\`\`\`tex?  \`\`\`tr-9??- \?e_7??- \?## }??- \?e??- \?##sket??-pa  - \CB  - \ad- at\`\`\`tex?  \`\`\`tr-9??-:   - \CB  - \ad- at\`\`\`tex?  \`\`\`tex??  \`\`\o -   ####
  - \CB  - \OU- at\`\`\`tex?rr  - \CB  - \ad- at\`\`\`tex?  \`\`\`tex??  ge  - \CB  - \OU- O  - \CB  - \OU- at\`\`\`tex?rr  - \CB  - \ad- at\`\`\`texpl- at\`\`\`tex?  \`\`\`tex??  \`\`\o -   ####
  - \CB  - \ad- at\`\`\`tex?  \`\`\`tr-9??- \?e_7??- \?## }??- \?e??- \?##sket??- \?##ac???- ?r**- at\`\`\`tex?  \`\`\`tex??  \`\`\o -   ####
  - \CB  - \ad- at\`\`\`tex?  \`\`\`tex??  \`\`\o -   ####
  - \CB  - \OU- at\`\`\`tex?rr  - \CB  - \ti- at\`\`\`tex?  \`\`\`tex??  \`\`\o S  - \CB  - \ad- at\`\`\`tex?  \`\`\`tr-9??-n*  - \CB  - \ad- at\`\`\`tex?  \`\`\`tex??  \`\`\o -   ####
  - \CB  - \OU- at\`\`\`tex?rr  - \CB  - \ti- at\`\`\`tex?  \`\`\`tex??  \`\`\o -   ####
 de  - \CB  - \OU- at\`\`\`tex?rr  - \CB  - \ti- at\`\`\`texim  - \CB  - \ad- at\`\`\`tex?  \`\`\`tr-9??- \?e_7??- \?## }??- \?e??- \?##sket??-a   - \CB  - \OU- at\`\`\`tex?rr  - \CB  - \ad- at\`\`\`tex?  \`\`\`tex??  ge  - \CB  - \OU- O  - \CB  - \OU- at\`\`\`tex?rr  - \CB  - \ad- at\`\`\`texpl- at\`\`\`tex?  \`\`\`tex??  \`\`\o -    D  - \CB  - \ad- at\`\`\`tex?  \`\`\`tr-9??- \?e_7??- \?## }??- \?e??- \?##sket??- \?##ac???- ?r**- at\`\`\`tex?  \`\`\`tex??  \`\`\o -   ####
  - \CB  - \ad- at\`\`\`tex?  \`\`\`tex??  *P  - \CB  - \ad- at\`\`\`tex?  \`\`\`tex??  \`\`\o -   ####
  - \CB  - \OU- at\`\`\`tex?rr  - \CB  - \ti- at\`\`\`tex?  \`\`\`tex??  \`\`\o S  - \CB  bl  - \CB  - \OU- at\`\`\`tex?rr  - \CB  - \ti- at\`\`\`texng  - \CB  - \OU- at\`\`\`tex?rr  - \CB  - \ti- at\`\`\`tex?  \`\`\`tex??  \`\`\o -   ####
 de  - \CB  - \OU- at\`\`\`tex?rr  - \CB  - \ti- at\`\`\`texim  - \CB  - \ad- at\`\`\`tex?  \`er de  - \CB  - \OU- at\`\`\`tex?rr  - \CB  - \ti- at\`\`\`texim  - \CB  - \ad- at\`\`\`tCh  - \CB  - \ad- at\`\`\`tex?  \`\`\`tex??  *P  - \CB  - \ad- at\`\`\`tex?  \`\`\`tex??  \`\`\o -   ####
  - \CB  - \OU- at\`\`\`tex?rr  - \CB  - \ti- at\`\`\`tex?  \`\`\`tex??  \`\`\o S  - \CB  bl  - \CB  - \OU- at\`\`\`tex?rr  - \CB  - \ti- at\`\`\`texng  - \CB  - \OU- at\`\`\`tex?rr  - \CB  - \ti- at\`\`\`tex?  \`\`\`tex??  \`\`\o -   ####
 de  - \CB  - \OU- at\`\`\`tex?rr  - \CB  - \ti- at\`\`\`texim  - \CB  - \ad- at\`\`\`tex?  \`er de  - \CB  - \OU- at\`\`\`tex?rr  - \CB  - \ti- at\`ge  - \CB  - \OU- at\`\`\`tex?rr  - \CB  - \ti- at\`\`\`tex?  \`\`\`tex??  \`\`\o S  - \CB  bl  - \CB  -in de  - \CB  - \OU- at\`\`\`tex?rr  - \CB  - \ti- at\`\`\`texim  - \CB  - \ad- at\`\`\`tex?  \`er de  - \CB  - \OU- at\`\`\`tex?rr  - \CB  - \ti- at\`\`\`texim  - \CB  - \ad- at\`\`\`tCh  - \CB  - \ad- at\`\`\`tex?  \`\`\`tex??  *P  - \CB ld  - \CB  - \OU- at\`\`\`tex?rr  - \CB  - \ti- at\`\`\`tex?  \`\`\`tex??  \`\`\o S  - \CB  bl  - \CB  - \OU- at\`\`\`tex?rr  - \CB  - \ti- at\`\`\`texng  - \CB  - \OU- at\`\`\`tex?rr  - \CB  - \ti- at\`\`\`tex?  \`\`\`tex??  \`\`\o -   ####
 de  - \CB  - \OU- at\`\`\`tex?rr  - \CB  - \tien de  - \CB  - \OU- at\`\`\`tex?rr  - \CB  - \ti- at\`\`\`texim  - \CB  - \ad- at\`\`\`tex?  \`er de  - \CB  - \OU- at\`\`\`tex?rr  - \CB  - \ti- at\`ge  - \CB  - \OU- at\`\`\`tex?rr  - \CB  - \ti- at\`\`\`tex?  \`\`\`tex??  \`\`\o S  - \Cut de  - \CB  - \OU- at\`\`\`tex?rr  - \CB  - \tien de  - \CB  - \OU- at\`\`\`tex?rr  - \CB  - \ti- at\`\`\`texim  - \CB  - \ad- at\`\`\`tex?  \`er de  - \CB  - \OU- at\`\`\`tex?rr  - \CB  - \ti- at\`ge  - \CB  - \OU- at\`\`\`tex?rr  - \CB  - \ti- at\`\`\`tex?  \`\`\`tex??  \`\`\o S  - \Cut de  - \CB  - \OU- at\`\`\`tex?rr  - \CB  - \tien de  - \CB  - \OU- at\`\`\`tex?rr  - \CB  - \ti- at\`\`\`texim  - \CB  - \ad- at\`\`\`tex?  \`er de  - \CB  - \OU- at\`\`\`tex?rr  - \CB  - \ti- at\`ge  - \CB  - \OU- at\`\`\`tex?rr  - \CB  - \ti- at\`\`\`tex?  \`\`\`tex??  \`\`\o S  - \Cut de  - \CB  - \OU- at\`\`\`tex?rr  - \CB  - \tien de  - \CB  - \OU- at\`\`\`tex?rr  - \CB  - \ti- at\`\`\`texim  - \CB  - \ad- at\`\`\`tex?  \`er de  - \CBding/list/code detection          |
| Format Score        | >= 0.85   | Bold/italic/bullet preservation      |
| Clippy Warnings     | 0         | cargo clippy --all-features          |
| Test Coverage       | >= 80%    | cargo tarpaulin                      |
| Documentation       | Complete  | All public APIs documented           |
| lopdf Deprecation   | Complete  | Clear migration path documented      |
