# Mission: Improve PDF-to-Markdown Conversion Quality

## Task

Your mission is to systematically improve the quality of PDF-to-Markdown conversion in the `edgequake-pdf` crate to match or exceed the quality of PyMuPDF4LLM, the industry gold standard for LLM-optimized PDF extraction.


If image is discovered in the PDF they should be extracted in ./assets/ subfolder and linked as image in the transformed markdown as a Markdown image, use png or jpeg for images, use unique names for image extracted.


Ensure to always use First Principle logic not poor heuristics that use keywords or content of a specific document (it is cheating). always use timeout for tests (don't be blocked)

Ensure to remove lopdf legacy code in other to improve the quality of the code: dead code is not good for maintenance: only keep one extraction , conversion pipeline for clarity.


Read files to compare gold vs converted at the end of each iteration to evaluate the quality.

FULLY READ this file !!!!


## ⚠️ CRITICAL SAFETY MANDATE ⚠️

**YOU MUST RE-READ THIS ENTIRE MISSION FILE AT THE START OF EVERY OODA ITERATION.**

Failure to re-read causes alignment drift → catastrophic safety issues → user frustration → system unreliability.

## Context

- **Location**: `edgequake/crates/edgequake-pdf/`
- **Test Documents**: `zz_test_docs/` (real-world PDFs for evaluation)
- **Gold Standard Baseline**: `zz-explore/pymupdf4llm/` (PyMuPDF4LLM reference implementation)
- **Existing Gold Tests**: `edgequake/crates/edgequake-pdf/test-data/gold/` (105 categorized test cases)
- **Test Protocol**: `edgequake/crates/edgequake-pdf/TEST_PROTOCOL.md`

Always check zz-explore/pymupdf4llm  for reference algorithms inspiration

### Current Architecture

```
edgequake-pdf/src/
├── backend/          # PDF parsing backend (lopdf)
├── bin.rs            # CLI binary entry point
├── config.rs         # Configuration management
├── error.rs          # Error types
├── extractor.rs      # Main extraction orchestration
├── formula/          # Math formula handling
├── image_extraction.rs
├── image_ocr.rs      # Vision AI image description
├── layout/           # Layout analysis (columns, reading order)
├── lib.rs            # Library entry point
├── pipeline/         # Processing pipeline
├── processors/       # Text processors (formatting, tables)
├── progress.rs       # Progress reporting
├── renderers/        # Markdown rendering
├── rendering.rs      # Core rendering logic
├── schema/           # Data structures
└── vision.rs         # Vision model integration
```

### Quality Targets

Based on `TEST_PROTOCOL.md` and PyMuPDF4LLM capabilities:

| Category               | Current Score | Target Score | Priority     |
| ---------------------- | ------------- | ------------ | ------------ |
| Basic text extraction  | 85/100        | 95/100       | Medium       |
| Bold/Italic formatting | 80/100        | 95/100       | High         |
| Headers (H1-H6)        | 75/100        | 90/100       | Medium       |
| Multi-column layouts   | 60/100        | 85/100       | **Critical** |
| Tables                 | 50/100        | 80/100       | **Critical** |
| Code blocks            | 70/100        | 90/100       | High         |
| Lists (nested)         | 55/100        | 85/100       | High         |
| Unicode handling       | 70/100        | 90/100       | Medium       |

### Key Improvement Areas

1. **Multi-Column Reading Order** - Three-column layouts broken (score: 0-39)
2. **Table Detection** - Spanning issues, complex headers fail
3. **Nested Lists** - Indentation flattened
4. **Code vs Table Disambiguation** - False positives
5. **Footnotes/References** - Not properly extracted

### Test Documents in `zz_test_docs/`

Real-world PDFs for validation:

- `AI_Services__Elitizon.pdf` - Business document with tables
- `AgenticPlatformReference Architecture.pdf` - Technical architecture
- `Scottish SMEs Delegation*.pdf` - Multi-column newsletter
- `SEAL_U_DM-i-0225-FR-V5.pdf` - French technical document
- `lighrag_2410.05779v3.pdf` - Academic paper (2+ columns)
- `agentfail_2601.22984v1.pdf` - Research paper
- `hotmess_2601.23045v1.pdf` - Research paper
- Various Renault PDFs - Complex formatted documents

### PyMuPDF4LLM Reference (Gold Standard)

Located at `zz-explore/pymupdf4llm/`, key modules:

- `helpers/multi_column.py` - Multi-column detection algorithm
- `helpers/document_layout.py` - Layout analysis
- `helpers/pymupdf_rag.py` - RAG-optimized extraction
- `helpers/get_text_lines.py` - Line-level text extraction

---

## Process: OODA Loop (50 iterations minimum)

Execute iterative OODA cycles. Each iteration produces 4 files:

**⚠️ You MUST absolutely read your mission every iteration! It is vital to avoid alignment drift. You can forget previous iterations, but never forget your mission.**

Mission file: `specs/007-improve-pdf-markdown-quality.md`

You MUST always produce the 4 files per iteration, as shown below:

```
specs/007-improve-pdf-markdown-quality/ooda/
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

| Step        | Output                                                     |
| ----------- | ---------------------------------------------------------- |
| **Observe** | Code analysis, feature inventory, dependency mapping       |
| **Orient**  | Gap analysis, documentation quality assessment             |
| **Decide**  | Specific changes prioritized by signal value               |
| **Act**     | Implementation with commit (`OODA-XX: <decision summary>`) |

### Constraints

1. **Re-read mission** every iteration: `specs/007-improve-pdf-markdown-quality.md`
2. **Continue** from existing iterations—never restart
3. **Reference** real code: file paths, line numbers, commit SHAs
4. **Use ASCII diagrams** for architecture/flow visualization
5. **Split large files** for maintainability, Use Single Responsibility Principle (SRP)
6. **Optimize** Rust build speed (latest toolchain)
7. **Document amendments** in WHY, high signal value, and precise terms in comments in the codebase. Use ASCII diagrams where applicable.
8. **You must perform tests** and deliver evidence that all tests are passing after your changes.
9. **Ensure test timeouts** - All tests must have timeouts to avoid blocking CI/CD pipelines
10. **Apply DRY principle** - Don't Repeat Yourself; extract common patterns
11. **Eliminate dead code** - Remove unused functions, imports, and modules
12. **Accelerate tests** - Optimize test execution time, use parallel execution where safe

### Quality Principles

```
┌─────────────────────────────────────────────────────────────┐
│                    QUALITY NORTH STAR                       │
├─────────────────────────────────────────────────────────────┤
│  1. SRP: Each module does ONE thing well                    │
│  2. DRY: Extract common patterns into reusable components   │
│  3. DEAD CODE: If it's not used, delete it                  │
│  4. FAST TESTS: < 100ms per unit test, timeout at 5s max    │
│  5. FIRST PRINCIPLES: Understand WHY before implementing    │
└─────────────────────────────────────────────────────────────┘
```

Use high signal command + ASCII explantion of algorithms in comments / focus on WHY 

Always use First Principle Thinking

### Test Execution Requirements

```bash
# Run all PDF crate tests with timeout
cargo test --package edgequake-pdf -- --test-threads=4

# Run quality evaluation
cd edgequake/crates/edgequake-pdf/test-data && ./eval.sh

# Compare against PyMuPDF4LLM baseline
python3 zz-explore/pymupdf4llm/examples/compare.py
```

### Deliverables

1. **Improved conversion quality** matching PyMuPDF4LLM for core features
2. **Clean codebase** following SRP/DRY principles
3. **Fast test suite** with all tests passing and timeouts enforced
4. **Documentation** of changes and rationale in OODA files
5. **Metrics improvement** tracked in `test-data/evaluation_results.json`

---

## ⚠️ CRITICAL SAFETY MANDATE ⚠️

**YOU MUST RE-READ THIS ENTIRE MISSION FILE AT THE START OF EVERY OODA ITERATION.**

Failure to re-read causes alignment drift → catastrophic safety issues → user frustration → system unreliability.

### Iteration Checklist

Before each iteration, verify:

- [ ] Read `specs/007-improve-pdf-markdown-quality.md` completely
- [ ] Review previous iteration's `act.md` for context
- [ ] Check current test status with `cargo test --package edgequake-pdf`
- [ ] Identify specific improvement target for this iteration
- [ ] Plan changes that follow SRP/DRY/First Principles

### Success Criteria

An iteration is successful when:

1. All 4 OODA files are created with substantive content
2. Tests pass (or failing tests are documented with fix plan)
3. Code changes follow established principles
4. Progress is measurable against quality targets

---

## Reference Commands

```bash
# Build PDF crate
cargo build --package edgequake-pdf --release

# Run tests with output
cargo test --package edgequake-pdf -- --nocapture

# Run clippy for code quality
cargo clippy --package edgequake-pdf -- -W clippy::all

# Format code
cargo fmt --package edgequake-pdf

# Generate baseline with PyMuPDF4LLM
python3 -c "import pymupdf4llm; print(pymupdf4llm.to_markdown('zz_test_docs/AI_Services__Elitizon.pdf'))"
```

## Appendix: PyMuPDF4LLM Key Features to Match

1. **Clean Markdown output** - Structured, LLM-optimized
2. **Document hierarchy** - Headers, lists, tables preserved
3. **Multi-column handling** - Proper reading order
4. **Image extraction** - Optional inline or separate
5. **Page chunks** - Configurable chunking for RAG
6. **Format support** - PDF, XPS, eBooks


Image extracted png, svg, jpeg, tiff, bmp, gif, webp, heif, avif, pdf (for vector graphics) and linked as markdown image in the output markdown file.


Ensure image name is idopotent (e.g., hash of content) to avoid duplicates and enable caching.
