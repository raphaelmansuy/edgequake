# Mission: Perfect PDF to Markdown Conversion

## Task

Your mission is to achieve **production-grade, high-fidelity PDF to Markdown conversion** for the edgequake-pdf crate. This involves:

1. **Diagnosis**: Analyze why test documents in `zz_test_docs/` produce poor Markdown output
2. **Test Strategy**: Design a comprehensive E2E testing framework with diverse PDF categories
3. **Quality Metrics**: Define measurable, objective quality metrics for conversion fidelity
4. **Algorithm Perfection**: Iteratively improve the PDF extraction pipeline until all metrics pass

Ensure to use First Principles thinking. Use your knowledge of PDF structures, text encoding, layout analysis, and Markdown syntax to guide your decisions. Use Donald Knuth knowledge about text composition and typesetting where applicable: space between letters, words, line breaks, paragraph structure, ligatures, font styles, etc. If you don't know something, research it thoroughly on the web: PDF, typesetting, Markdown, text extraction techniques.

Use markitdown mcp to compare extracted Markdown against gold standard references.

VERY IMPORTANT: Optimize for speed and efficiency:

Optimize compilation, and test times. Use incremental compilation, cargo check, and selective test runs to speed up iteration cycles.

VERY IMPORTANT:

RUST_LOG=info cargo run --example convert_test_docs 2>&1 | cat

Accelerate tests --> it takes too long, split in focus tests

## Context

- **Location**: `/Users/raphaelmansuy/Github/03-working/edgequake/edgequake/crates/edgequake-pdf/`
- **Test Documents**: `/Users/raphaelmansuy/Github/03-working/edgequake/zz_test_docs/`
- **Existing Test Data**: `/Users/raphaelmansuy/Github/03-working/edgequake/edgequake/crates/edgequake-pdf/test-data/`

### Comparison Tool: MCP Markitdown

Use `mcp_markitdown_convert_to_markdown` tool to generate reference markdown from PDFs. This provides a baseline for comparison against our extraction output.

### Test Documents in `zz_test_docs/`

| Document                                    | Pages | Type                     | Status                    |
| ------------------------------------------- | ----- | ------------------------ | ------------------------- |
| `Qwen.pdf`                                  | 1     | Type3 fonts, web capture | ✅ Fixed (OODA-01/02)     |
| `001-BEYONG-TRANFORMER-OUTLINE-V1_1.pdf`    | 10+   | Academic outline         | ✅ Extracts               |
| `AgenticPlatformReference Architecture.pdf` | 50+   | Architecture doc         | ✅ Extracts               |
| `Apple-Sandbox-Guide-v1.0.pdf`              | ?     | Technical guide          | 🔄 New - needs evaluation |
| `agentfail_2601.22984v1.pdf`                | ?     | arXiv paper              | 🔄 New - needs evaluation |
| `hotmess_2601.23045v1.pdf`                  | ?     | arXiv paper              | 🔄 New - needs evaluation |

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

## Technical Focus Areas

### Priority 1: Font Encoding (Qwen.pdf failure)

- ToUnicode CMap handling
- CID font decoding
- Adobe-Identity-H encoding
- Fallback strategies

### Priority 2: Table Detection

- Lattice vs stream table detection
- Cell boundary accuracy
- Header row detection
- Multi-page table continuation

### Priority 3: Layout Analysis

- Column detection thresholds
- Reading order algorithm
- Block merging heuristics
- Header/footer filtering

### Priority 4: Special Content

- ASCII art preservation
- Code block detection
- Formula recognition
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
