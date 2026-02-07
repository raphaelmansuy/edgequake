# Mission: Improve PDF to Markdown Conversion Quality - SOTA Target

## Task

Your mission is to improve the quality of PDF to Markdown conversion in `edgequake/crates/edgequake-pdf` to achieve **state-of-the-art (SOTA) conversion quality** as measured by unfalsifiable, automated metrics against real-world academic papers and documents.

FULLY Read this entire mission file at the start of EVERY OODA iteration to avoid alignment drift.

## CRITICAL SAFETY MANDATE

**YOU MUST RE-READ THIS ENTIRE MISSION FILE AT THE START OF EVERY OODA ITERATION.**

Failure to re-read causes alignment drift.

Don't stop until you reach 70+ OODA iterations with significant quality improvements and all tests passing. Each iteration must produce the 4 required files (observe.md, orient.md, decide.md, act.md).

## Context

- **Current Implementation Location**: `edgequake/crates/edgequake-pdf`
- **Gold Standard Reference**: `zz-explore/pymupdf4llm/pymupdf4llm/pymupdf4llm`
- **Primary Algorithm File**: `zz-explore/pymupdf4llm/pymupdf4llm/pymupdf4llm/helpers/document_layout.py`
- **Test Directory**: `edgequake/crates/edgequake-pdf/test-data`
- **Real PDFs with Gold Standards**: `edgequake/crates/edgequake-pdf/test-data/real_dataset/`
- **Gold Standard Subsets**: `edgequake/crates/edgequake-pdf/test-data/gold/`
- **External Test PDFs**: `zz_test_docs/`

---

## SOTA Quality Metrics Framework

### Unfalsifiable Automated Metrics

Quality must be measured by automated, reproducible metrics that cannot be gamed:

#### 1. Character-Level Fidelity (CLF)

```
CLF = 1 - (levenshtein_distance(extracted, gold) / max(len(extracted), len(gold)))
```

- Target: CLF > 0.95 on real academic papers
- Measured against pymupdf4llm gold standard outputs in `test-data/real_dataset/*.pymupdf.gold.md`

#### 2. Structure Preservation Score (SPS)

```
SPS = (correct_headers + correct_lists + correct_code_blocks + correct_footnotes) / total_structural_elements
```

- Count markdown structural elements (# headers, - lists, ``` code blocks, footnote markers)
- Compare count and ordering against gold standard
- Target: SPS > 0.90

#### 3. Reading Order Accuracy (ROA)

```
ROA = longest_common_subsequence(extracted_paragraphs, gold_paragraphs) / len(gold_paragraphs)
```

- Split into paragraph blocks, measure ordering preservation
- Critical for multi-column PDF layouts
- Target: ROA > 0.95

#### 4. Noise Ratio (NR)

```
NR = (spurious_headers + spurious_footers + spurious_page_numbers + empty_blocks) / total_blocks
```

- Lower is better. Target: NR < 0.05
- Measures how much noise (repeated headers/footers, page numbers) leaks through

#### 5. Compilation Speed

```
Build Time: cargo build -p edgequake-pdf (cold) < 30s
Test Time: cargo test -p edgequake-pdf --lib < 60s
```

- Fast feedback loop is essential for rapid iteration

### Metric Collection

Metrics MUST be collected in Rust test code (not external scripts) for reproducibility:

- `#[test] fn test_quality_metrics_real_pdf()` - Tests that load real PDFs and measure CLF/SPS/ROA
- Results tracked in `specs/001-improve-markdown-2-pdf/ooda_loop/metrics.md`

---

## Key Quality Dimensions

1. **Text Extraction Accuracy**: Character-level fidelity, font detection, style preservation
2. **Structure Recognition**: Tables, lists, headers, footnotes, code blocks
3. **Reading Order**: Multi-column layouts, proper flow detection
4. **Table Detection**: Lattice and stream modes, cell alignment
5. **Style Preservation**: Bold, italic, monospace, strikeout, superscript
6. **Edge Cases**: Malformed PDFs, Type3 fonts, PUA characters, hyphenation
7. **Noise Filtering**: Page numbers, headers/footers, copyright notices, watermarks

---

## Process: OODA Loop (50 iterations minimum)

Execute iterative OODA cycles. Each iteration produces 4 files:

**CRITICAL: You MUST re-read this entire mission file at the start of EVERY iteration to avoid alignment drift.**

Mission file: `./specs/001-improve-markdown-2-pdf.md`

1. **observe.md** - Map the territory. Verify against actual codebase. Run metrics.
2. **orient.md** - Analyze findings, define solutions using First Principles.
3. **decide.md** - Prioritize changes by measurable metric impact.
4. **act.md** - Implement with precision. Include file:line references and commit SHAs.

```
specs/001-improve-markdown-2-pdf/ooda_loop/
+-- iteration_XX/
|   +-- observe.md
|   +-- orient.md
|   +-- decide.md
|   +-- act.md
+-- metrics.md        # Running metrics tracker
+-- summary.md        # Cross-iteration insights
```

### Per-Iteration Requirements

| Step        | Output                                                     |
| ----------- | ---------------------------------------------------------- |
| **Observe** | Code analysis, metric measurement, gap identification      |
| **Orient**  | Root cause analysis, metric impact estimation              |
| **Decide**  | Specific changes prioritized by metric improvement         |
| **Act**     | Implementation with commit (`OODA-XX: <decision summary>`) |

### Constraints

1. **Re-read mission** every iteration: `specs/001-improve-markdown-2-pdf.md`
2. **Continue** from existing iterations - never restart
3. **Reference** real code: file paths, line numbers, commit SHAs
4. **Measure before/after**: Every change must show metric impact
5. **Split large files** for maintainability using SRP
6. **Optimize** build and test speed for fast feedback loop
7. **Document** WHY behind decisions with precise technical language
8. **Run tests** and deliver passing evidence after every change
9. **Focus on real PDFs**: Test against papers in `test-data/real_dataset/`

---

## Deliverables

### Per Iteration

- 4 OODA files (observe.md, orient.md, decide.md, act.md)
- Code committed as: `OODA-XX: <brief summary>`
- Test evidence: all tests passing
- Metric delta: before/after for changed dimensions

### Final Deliverables (After 50+ Iterations)

1. **SOTA PDF Extraction Pipeline**
   - CLF > 0.95 on real academic papers
   - SPS > 0.90 for structure preservation
   - ROA > 0.95 for reading order
   - NR < 0.05 noise ratio

2. **Comprehensive Test Suite**
   - 500+ unit tests (all passing)
   - Quality metric tests against real PDFs
   - Regression tests for edge cases

3. **Metrics Dashboard** (`specs/001-improve-markdown-2-pdf/ooda_loop/metrics.md`)
   - Per-iteration metric snapshots
   - Trend analysis across iterations

4. **Summary Report** (`specs/001-improve-markdown-2-pdf/ooda_loop/summary.md`)

---

## Success Criteria

1. **Quantitative (Unfalsifiable)**
   - CLF > 0.95 on 5+ real academic papers
   - SPS > 0.90 structure preservation
   - ROA > 0.95 reading order accuracy
   - NR < 0.05 noise ratio
   - 500+ tests passing
   - Build time < 30s, test time < 60s

2. **Qualitative**
   - Idiomatic Rust (Result<T>, Option<T>, iterators)
   - SRP and DRY throughout
   - Clear documentation with WHY rationale

3. **Process**
   - 50+ OODA iterations completed
   - Mission file re-read evidence in each observe.md
   - Metrics tracked per iteration

---

## First Principles Foundation

1. **Preserve Document Intent**: Semantic equivalence, not pixel-perfect reproduction.
2. **Spatial Reasoning First**: Text position (x,y) determines reading order.
3. **Progressive Enhancement**: Basic text -> structure detection -> style preservation.
4. **Fail Gracefully**: Fall back to simpler representations when detection fails.
5. **Style Follows Content**: Font properties enhance readability, not obscure content.
6. **Test-Driven Quality**: Every improvement validated with before/after metrics.
7. **Speed is a Feature**: Fast compilation and tests enable rapid iteration.

---

## Key Algorithms from pymupdf4llm to Study

1. **Layout Box Classification** (document_layout.py:596-625)
2. **List Item Hierarchy Detection** (document_layout.py:97-151)
3. **Monospace Detection** (document_layout.py:154-169)
4. **Styled Text Extraction** (document_layout.py:355-416)
5. **Table Structure Completion** (document_layout.py:1003-1012)
6. **Reading Order Detection** (document_layout.py:998-1000)
7. **PUA Character Handling** (document_layout.py:83-94)

---

## Current Progress

### Completed OODA Iterations (01-46)

| Range | Focus Area                                   | Tests Added |
| ----- | -------------------------------------------- | ----------- |
| 01-08 | Core pipeline: char->span->line->block       | ~420        |
| 09-15 | Block classification, footnotes, bullets     | ~470        |
| 16-22 | Markdown rendering, code fences, styles      | ~500        |
| 23-29 | PUA filter, hyphenation, smart quotes        | ~520        |
| 30-40 | Body font detection, captions, normalization | ~525        |
| 41-46 | Page filter, footnotes, code detection       | ~528        |

### Architecture

```
RawChar --> Span --> Line --> Block --> Classified Block --> Markdown
  |          |        |        |            |                  |
  PDFium   font     y-pos   x-gap       header/list/       render()
  extract  match    group   detect      code/footnote      with styles
```

### Module Map

```
edgequake-pdf/src/
+-- layout/
|   +-- pymupdf_grouper.rs    # Char->Span->Line->Block pipeline
|   +-- pymupdf_renderer.rs   # Block->Markdown rendering
|   +-- block_classifier.rs   # Block type classification
|   +-- pymupdf_structs.rs    # Data structures (RawChar, Span, Line, Block)
|   +-- reading_order.rs      # Multi-column reading order
|   +-- column_detector.rs    # Column detection (exported from grouper)
|   +-- page_filter.rs        # Header/footer/page number filtering
|   +-- footnote.rs           # Footnote detection
|   +-- hyphenation.rs        # Hyphen resolution across line breaks
|   +-- mod.rs                # Module exports
+-- renderers/
|   +-- pua_filter.rs         # PUA/ligature/unicode normalization
+-- pipeline/
|   +-- pymupdf_pipeline.rs   # Orchestration and body font detection
+-- extractor.rs              # Top-level PDF extraction API
```

**Start Date**: 2026-02-06
**Target Completion**: After 70+ OODA iterations
**Status**: In Progress (OODA-46 complete, 528 tests passing)

---


Ensure to use parallel in tests to speed up the feedback loop if possible. Ensure the conversion PDF → Markdown is deterministic and reproducible for the same input PDF, improve for performance and quality. Focus on real-world academic papers with complex layouts, not just simple test cases.

**YOU MUST RE-READ THIS ENTIRE MISSION FILE AT THE START OF EVERY OODA ITERATION.**
