# Mission: Exceed pymupdf4llm PDF-to-Markdown Conversion Quality

## Task

Your mission is to make `edgequake/crates/edgequake-pdf` **exceed pymupdf4llm** in PDF-to-Markdown conversion quality, as measured by unfalsifiable automated metrics against a diverse 22-document test corpus spanning academic papers, technical docs, manuals, presentations, and reference materials.


Use First Principles thinking, and inspiration from pymupdf4llm's key algorithms, to design and implement improvements in Rust. Focus on preserving document intent, spatial reasoning, progressive enhancement, and fail-gracefully approaches.

FULLY Read this entire mission file at the start of EVERY OODA iteration to avoid alignment drift.

## CRITICAL SAFETY MANDATE

**YOU MUST RE-READ THIS ENTIRE MISSION FILE AT THE START OF EVERY OODA ITERATION.**

Failure to re-read causes alignment drift.

Don't stop until you reach 50+ NEW OODA iterations (OODA-61 through OODA-110+) with significant quality improvements and all tests passing.

## Context

- **Current Implementation**: `edgequake/crates/edgequake-pdf`
- **Reference Implementation**: `zz-explore/pymupdf4llm/pymupdf4llm/pymupdf4llm`
- **Primary Algorithm File**: `zz-explore/pymupdf4llm/pymupdf4llm/pymupdf4llm/helpers/document_layout.py`
- **Column Detection**: `zz-explore/pymupdf4llm/pymupdf4llm/pymupdf4llm/helpers/multi_column.py`
- **Entry Point**: `zz-explore/pymupdf4llm/pymupdf4llm/pymupdf4llm/helpers/pymupdf_rag.py`

### Test Data Locations

| Corpus | Location | Purpose |
|--------|----------|---------|
| **Primary Corpus** | `zz_test_docs/` (22 PDFs, organized) | Main test suite with pymupdf4llm gold standards |
| **Legacy Corpus** | `edgequake/crates/edgequake-pdf/test-data/real_dataset/` (7 PDFs) | Original test papers |
| **Unit Test PDFs** | `edgequake/crates/edgequake-pdf/test-data/` | Synthetic unit test PDFs |

### Primary Test Corpus (`zz_test_docs/`)

```
zz_test_docs/
├── academic_papers/     (8 docs) - Research papers, AI studies
├── technical_docs/      (3 docs) - Architecture guides, security docs
├── manuals/             (5 docs) - Renault car manuals (French)
├── presentations/       (1 doc)  - Sales presentation
├── reference_materials/ (5 docs) - Conventions, guides, references
└── generated_output/    - Previously generated test outputs
```

Each PDF has a `.pymupdf.gold.md` file alongside it (generated via `pymupdf4llm.to_markdown()`).

---

## SOTA Quality Metrics Framework

### Unfalsifiable Automated Metrics

Quality must be measured by automated, reproducible metrics that cannot be gamed:

#### 1. Character-Level Fidelity (CLF)

```
CLF = 1 - (levenshtein_distance(extracted_words, gold_words) / max(len(extracted_words), len(gold_words)))
```

- Target: CLF > 0.95 on diverse document corpus
- Measured against pymupdf4llm gold standard outputs (`*.pymupdf.gold.md`)

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
Quick Metric Test: < 30s for representative subset
Full Corpus Test: < 5 min for all 22 documents
```

- Fast feedback loop is essential for rapid iteration

### Metric Collection

Metrics MUST be collected in Rust test code for reproducibility:

- **Quick feedback test**: `cargo test -p edgequake-pdf --test quality_zz_corpus -- --nocapture` (representative subset, <30s)
- **Full corpus test**: Same test with all 22 documents
- Results tracked in commit messages with `OODA-XX:` prefix

---

## Key Quality Dimensions

1. **Text Extraction Accuracy**: Character-level fidelity, font detection, style preservation
2. **Structure Recognition**: Tables, lists, headers, footnotes, code blocks
3. **Reading Order**: Multi-column layouts, proper flow detection
4. **Table Detection**: Lattice and stream modes, cell alignment
5. **Style Preservation**: Bold, italic, monospace, strikeout, superscript
6. **Edge Cases**: Malformed PDFs, Type3 fonts, PUA characters, hyphenation
7. **Noise Filtering**: Page numbers, headers/footers, copyright notices, watermarks
8. **Multi-Language**: French manuals, mixed-language documents

---

## Process: Rapid OODA Loop (50+ iterations)

Execute iterative OODA cycles. Speed is paramount - each iteration should take minutes, not hours.

**CRITICAL: You MUST re-read this entire mission file at the start of EVERY iteration.**

Mission file: `./specs/001-improve-markdown-2-pdf.md`

### Per-Iteration Workflow (Streamlined)

1. **Observe**: Run metrics, identify biggest gap
2. **Orient**: Root-cause the gap (read code, compare outputs)
3. **Decide**: Pick the highest-ROI fix
4. **Act**: Implement, test, commit as `OODA-XX: <brief summary>`

### Commit Format

```
OODA-XX: <brief description>

Metrics: CLF=X.XXX SPS=X.XXX ROA=X.XXX NR=X.XXX
Delta: CLF+X.XXX SPS+X.XXX ROA+X.XXX NR+X.XXX
```

### Constraints

1. **Re-read mission** every iteration
2. **Continue** from existing iterations (currently at OODA-60)
3. **Measure before/after**: Every change must show metric impact
4. **All tests must pass** after every change
5. **Speed**: Quick feedback loop - run targeted tests, not full suite every time
6. **Focus on real PDFs**: Test against diverse documents in `zz_test_docs/`
7. **No regressions**: Metrics must not decrease on existing test papers

---

## Key Algorithms from pymupdf4llm to Study and Exceed

### Reference Implementation Approaches

1. **Layout Box Classification** (document_layout.py:596-625): MuPDF `get_layout()` classifies boxes into text/picture/table/header/footer
2. **List Item Hierarchy** (document_layout.py:97-151): Groups contiguous list items, x0 offset >10pt = new level
3. **Monospace Detection** (document_layout.py:154-169): All spans have flag & 8
4. **Styled Text** (document_layout.py:355-416): Bold (flag 16), italic (flag 2), strikeout (char_flag 1), superscript (flag 1)
5. **Column Detection** (multi_column.py): Three-phase rectangle merging with 10pt vertical tolerance, 3pt alignment tolerance
6. **Reading Order** (multi_column.py:283-325): Find left-most vertically-overlapping block as sort key
7. **Header Detection** (pymupdf_rag.py): Font size histogram - most frequent = body, larger = headers (up to 6 levels)
8. **Table Detection** (pymupdf_rag.py): MuPDF TableFinder + vector graphics integration, min 2x2
9. **PUA Character Handling** (document_layout.py:83-94): Skip 0xE000-0xF8FF, 0xF0000-0xFFFFD, 0x100000-0x10FFFD

### Where To Exceed pymupdf4llm

- **Font name fallback**: PDFium weight unreliable for CM fonts - use font name patterns (already implemented OODA-58/60)
- **Reference splitting**: Detect [N] boundaries in reference sections (OODA-59)
- **Better noise filtering**: More aggressive page number/header/footer removal
- **Hyphenation**: Resolve mid-word line breaks without hyphens
- **Table reconstruction**: Use spatial analysis to reconstruct pipe-delimited tables
- **Figure text suppression**: Detect and suppress scattered character text from figure overlays

---

## Current Progress

### Completed OODA Iterations (01-60)

| Range | Focus Area | Key Metric Gains |
|-------|-----------|-----------------|
| 01-08 | Core pipeline: char->span->line->block | Foundation |
| 09-15 | Block classification, footnotes, bullets | SPS baseline |
| 16-22 | Markdown rendering, code fences, styles | CLF baseline |
| 23-29 | PUA filter, hyphenation, smart quotes | CLF improvement |
| 30-40 | Body font detection, captions, normalization | CLF +0.05 |
| 41-46 | Page filter, footnotes, code detection | NR improvement |
| 47-53 | Quality metric framework, test infrastructure | Metrics framework |
| 54-57 | Clippy, space threshold, reading order | SPS +0.10, ROA +0.03 |
| 58 | Bold font name fallback, multi-digit lists | SPS 0.682->0.954 |
| 59 | Reference entry splitting at [N] boundaries | ROA 0.526->0.582 |
| 60 | Italic font name fallback for CM/EC fonts | Metric neutral |

### Current Metrics (OODA-60, 7 legacy papers)

```
CLF=0.668  SPS=0.954  ROA=0.582  NR=0.019
```

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
|   +-- column_detector.rs    # Column detection
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

---

## Success Criteria

1. **Quantitative (Unfalsifiable)**
   - CLF > 0.95 averaged across 22-document corpus
   - SPS > 0.90 structure preservation
   - ROA > 0.95 reading order accuracy
   - NR < 0.05 noise ratio
   - 500+ tests passing
   - Quick metric test < 30s

2. **Qualitative**
   - Idiomatic Rust (Result<T>, Option<T>, iterators)
   - SRP and DRY throughout
   - Clear documentation with WHY rationale

3. **Process**
   - 50+ NEW OODA iterations (OODA-61 through OODA-110+)
   - Mission file re-read at start of each iteration
   - Metrics tracked per iteration in commit messages

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

**Start Date**: 2026-02-06
**Phase 2 Start**: 2026-02-07 (new corpus, 50+ iterations)
**Status**: In Progress (OODA-60 complete, 540 tests passing)

Ensure to use parallel in tests to speed up the feedback loop if possible. Ensure the conversion PDF -> Markdown is deterministic and reproducible for the same input PDF. Focus on the diverse 22-document corpus in `zz_test_docs/`.

**YOU MUST RE-READ THIS ENTIRE MISSION FILE AT THE START OF EVERY OODA ITERATION.**
