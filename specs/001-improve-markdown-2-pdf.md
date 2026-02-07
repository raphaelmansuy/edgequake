# Mission: Improve PDF to Markdown Conversion Quality

## Task

Your mission is to improve the quality of PDF to Markdown conversion in `edgequake/crates/edgequake-pdf` by implementing algorithms and techniques from the gold standard reference implementation `pymupdf4llm` (located at `zz-explore/pymupdf4llm`), using First Principles thinking to achieve superior text extraction, structure preservation, table detection, and reading order accuracy.

FULLY Read this entire mission file at the start of EVERY OODA iteration to avoid alignment drift.

## ⚠️ CRITICAL SAFETY MANDATE ⚠️

**YOU MUST RE-READ THIS ENTIRE MISSION FILE AT THE START OF EVERY OODA ITERATION.**

Failure to re-read causes alignment drift → catastrophic safety issues → user frustration → system unreliability.


Don't stop until you reach 50+ OODA iterations with significant quality improvements and all tests passing. Each iteration must produce the 4 required files (observe.md, orient.md, decide.md, act.md) with specific content as outlined in the OODA Loop section below.

## Context

- **Current Implementation Location**: `edgequake/crates/edgequake-pdf`
- **Gold Standard Reference**: `zz-explore/pymupdf4llm/pymupdf4llm/pymupdf4llm`
- **Primary Algorithm File**: `zz-explore/pymupdf4llm/pymupdf4llm/pymupdf4llm/helpers/document_layout.py`
- **Test Directory**: `edgequake/crates/edgequake-pdf/test-data`
- **Documentation**: `edgequake/crates/edgequake-pdf/docs`

### Key Quality Dimensions

1. **Text Extraction Accuracy**: Character-level fidelity, font detection, style preservation
2. **Structure Recognition**: Tables, lists, headers, footnotes, code blocks
3. **Reading Order**: Multi-column layouts, proper flow detection
4. **Table Detection**: Lattice and stream modes, cell alignment
5. **Style Preservation**: Bold, italic, monospace, strikeout, superscript
6. **Image Handling**: OCR integration, image extraction, formula detection
7. **Edge Cases**: Malformed PDFs, Type3 fonts, PUA characters, hyphenation

---

## Process: OODA Loop (50 iterations minimum)

Execute iterative OODA cycles. Each iteration produces 4 files:

**⚠️ CRITICAL: You MUST re-read this entire mission file at the start of EVERY iteration to avoid alignment drift.**

Mission file: `./specs/001-improve-markdown-2-pdf.md`

You MUST always produce the 4 files per iteration, as shown below:

1. **observe.md** → Map the territory. Never make assumptions about code structure or function. Always verify against the actual codebase. When you don't know, search the web for answers and documentation.
2. **orient.md** → Analyze your findings and define possible solutions using First Principles as your north star. Assess risks and benefits of each approach.
3. **decide.md** → Prioritize specific changes based on signal value and impact.
4. **act.md** → Implement the decided changes with precision, update documentation, and reference specific file:line numbers and commit SHAs.

```
specs/001-improve-markdown-2-pdf/ooda_loop/
├── iteration_01/
│   ├── observe.md   # Data gathered: code, business rules, workflows
│   ├── orient.md    # Analysis of findings vs. current docs
│   ├── decide.md    # Prioritized action plan
│   └── act.md       # Changes made, with file:line references + commit hashes
├── iteration_02/
│   ├── observe.md
│   ├── orient.md
│   ├── decide.md
│   └── act.md
├── iteration_03/
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

1. **Re-read mission** every iteration: `specs/001-improve-markdown-2-pdf.md`
2. **Continue** from existing iterations—never restart
3. **Reference** real code: file paths, line numbers, commit SHAs
4. **Use ASCII diagrams** for architecture/flow visualization
5. **Split large files** for maintainability using Single Responsibility Principle (SRP)
6. **Optimize** Rust build speed (latest toolchain)
7. **Document amendments** with WHY, high signal value, and precise terms in comments. Use ASCII diagrams where applicable.
8. **You must perform tests** and deliver evidence that all tests are passing after your changes.

**YOU MUST re-read this entire mission file at the start of EVERY OODA iteration.**

Failure to re-read causes alignment drift → catastrophic safety issues → user frustration → system unreliability.

---

## Deliverables

### Per Iteration

- 4 OODA files (observe.md, orient.md, decide.md, act.md)
- Code changes committed with format: `OODA-XX: <brief summary>`
- Test evidence showing passing tests
- Documentation updates with file:line references

### Final Deliverables (After 50+ Iterations)

1. **Enhanced PDF Extraction Pipeline**
   - Superior text extraction accuracy
   - Robust table detection and rendering
   - Accurate multi-column reading order
   - Proper style preservation (bold, italic, code, etc.)

2. **Comprehensive Test Suite**
   - All existing tests passing
   - New tests for edge cases
   - Benchmark comparisons with pymupdf4llm

3. **Documentation**
   - Updated architecture diagrams
   - Algorithm explanations
   - Performance benchmarks
   - Migration guide

4. **Summary Report** (`specs/001-improve-markdown-2-pdf/ooda_loop/summary.md`)
   - Key improvements made
   - Performance metrics
   - Remaining gaps
   - Future recommendations

---

## Success Criteria

1. **Quantitative Metrics**
   - Text extraction accuracy: >98% character fidelity
   - Table detection rate: >95%
   - Reading order accuracy: >95%
   - Structure preservation: >90% (headers, lists, code blocks)
   - All existing tests pass

2. **Qualitative Metrics**
   - Code follows Rust best practices
   - Documentation is clear and comprehensive
   - Changes are atomic and well-explained
   - Architecture is maintainable and extensible

3. **Process Metrics**
   - Minimum 50 OODA iterations completed
   - Each iteration has all 4 required files
   - Each commit follows naming convention
   - Mission file re-read evidence in each observe.md

Focus on delivering high signal value in each change, with clear documentation of the WHY behind decisions. Use precise technical language and reference specific code locations to maximize clarity and maintainability. Use ASCII diagrams to illustrate complex flows, algorithms in code comments. Read the transformed markdown yourself  to compare with the gold standard reference implementation and ensure semantic equivalence, not just syntactic. Always prioritize quality over quantity in tests and iterations. Each change should have a measurable impact on the defined quality metrics.

Ensure quality tests are relevant and comprehensive, covering a wide range of PDF complexities and edge cases. Use real-world academic papers as test inputs to validate improvements. Focus to improve the quality of PDF to Markdown conversion, not just the quantity of tests or iterations. Each change should have a clear rationale and measurable impact on quality metrics.

Ensure test and compilation performance is top notch, with optimizations for Rust build times and test execution speed. Test time is important to maintain a fast feedback loop.

---

## First Principles Foundation

### Core Principles for PDF-to-Markdown Conversion

1. **Preserve Document Intent**: The goal is not pixel-perfect reproduction, but semantic equivalence in markdown format.

2. **Spatial Reasoning First**: Text position (x, y coordinates) determines reading order more reliably than PDF's internal structure.

3. **Progressive Enhancement**: Start with basic text extraction, then layer on structure detection (tables, lists, headers).

4. **Fail Gracefully**: When structure detection fails, fall back to simpler representations rather than omitting content.

5. **Style Follows Content**: Font properties (bold, italic, mono) should enhance readability, not obscure content.

6. **Test-Driven Quality**: Every improvement must be validated with concrete test cases showing before/after metrics.

---

## Key Algorithms from pymupdf4llm to Study

1. **Layout Box Classification** (document_layout.py:596-625)
   - LayoutBox dataclass with boxclass types
   - Text, picture, table, formula, list-item, footnote, etc.

2. **List Item Hierarchy Detection** (document_layout.py:97-151)
   - `create_list_item_levels()` function
   - Contiguous segment detection
   - Level assignment based on x0 coordinates

3. **Monospace Detection** (document_layout.py:154-169)
   - `is_monospaced()` function
   - Code block identification

4. **Styled Text Extraction** (document_layout.py:355-416)
   - `get_styled_text()` function
   - Font flags: superscript, mono, bold, italic, strikeout
   - Markdown prefix/suffix generation

5. **Table Structure Completion** (document_layout.py:1003-1012)
   - `complete_table_structure()` helper
   - Vector graphics detection for table boundaries

6. **Reading Order Detection** (document_layout.py:998-1000)
   - `find_reading_order()` from utils
   - Multi-column layout handling

7. **OCR Integration** (document_layout.py:940-988)
   - `should_ocr_page()` decision logic
   - Full-page vs text-only OCR strategies

8. **PUA Character Handling** (document_layout.py:83-94)
   - `omit_if_pua_char()` function
   - Private Use Area detection

---

## Notes

- Use `cargo test` to run all tests
- Use `cargo bench` for performance measurements
- Reference pymupdf4llm algorithms explicitly in commits
- Keep implementations idiomatic Rust (Result<T>, Option<T>, iterators)
- Document trade-offs and design decisions clearly
- Use `tracing` for logging, not `println!`

---

Ensure to respect SRP and DRY principles throughout development. Avoid code duplication and ensure each module/class has a single responsibility.

**Start Date**: 2026-02-06
**Target Completion**: After 50+ OODA iterations
**Status**: 🔄 In Progress
