# Mission: Improve PDF to Markdown Extraction Pipeline

## Task

Your mission is to **consolidate and improve the PDF to Markdown extraction pipeline** in the EdgeQuake project by:

1. **Unifying the extraction architecture**: Remove the legacy lopdf-based pipeline and consolidate on the pdfium-rs backend
2. **Fixing font style detection and propagation**: Ensure bold, italic, and code styles are correctly detected from PDF fonts and properly rendered in Markdown output
3. **Applying SOLID principles**: Refactor using Single Responsibility Principle (SRP) and Don't Repeat Yourself (DRY)
4. **Zero clippy warnings**: All code must pass `cargo clippy` with no warnings
5. **High-signal documentation**: Add WHY comments and ASCII diagrams explaining algorithms
6. **Comprehensive testing**: Achieve high test coverage with edge case handling


FULLY Read this mission file and:

 ⚠️ CRITICAL SAFETY MANDATE ⚠️

**YOU MUST RE-READ THIS ENTIRE MISSION FILE AT THE START OF EVERY OODA ITERATION.**

Failure to re-read causes alignment drift → catastrophic safety issues → user frustration → system unreliability.


## Context

- **Location**: `edgequake/crates/edgequake-pdf/`
- **Primary Backend**: `src/backends/pdfium/` (pdfium-rs based extraction)
- **Legacy Backend**: `src/backends/lopdf/` (to be evaluated for removal)
- **Font Style Flow**: Font detection → TextSpan creation → Style inference → Markdown rendering

---

## Current Architecture Analysis

### Two Extraction Pipelines

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                     PDF EXTRACTION PIPELINES                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │  NEW PIPELINE (pdfium-rs) - RECOMMENDED                             │    │
│  │  ═══════════════════════════════════════════════════════════════    │    │
│  │                                                                     │    │
│  │  PDF File                                                           │    │
│  │     │                                                               │    │
│  │     ▼                                                               │    │
│  │  ┌──────────────────┐                                               │    │
│  │  │ PdfiumExtractor  │  Uses pdfium-rs bindings                      │    │
│  │  │ (extractor.rs)   │  Full font metadata access                    │    │
│  │  └────────┬─────────┘                                               │    │
│  │           │                                                         │    │
│  │           ▼                                                         │    │
│  │  ┌──────────────────┐                                               │    │
│  │  │ Font Detection   │  Analyzes font name patterns:                 │    │
│  │  │ (font.rs)        │  - "Bold", "Bd", "Heavy" → BOLD               │    │
│  │  └────────┬─────────┘  - "Italic", "It", "Oblique" → ITALIC         │    │
│  │           │            - "Mono", "Courier", "Consola" → CODE        │    │
│  │           ▼                                                         │    │
│  │  ┌──────────────────┐                                               │    │
│  │  │ TextSpan         │  Carries: text, bbox, font_size, style        │    │
│  │  │ (types.rs)       │  Style = Bold | Italic | BoldItalic | Code    │    │
│  │  └────────┬─────────┘                                               │    │
│  │           │                                                         │    │
│  │           ▼                                                         │    │
│  │  ┌──────────────────┐                                               │    │
│  │  │ Layout Engine    │  Groups spans into lines/blocks               │    │
│  │  │ (layout.rs)      │  Preserves style through grouping             │    │
│  │  └────────┬─────────┘                                               │    │
│  │           │                                                         │    │
│  │           ▼                                                         │    │
│  │  ┌──────────────────┐                                               │    │
│  │  │ Markdown Render  │  Applies: **bold**, *italic*, `code`          │    │
│  │  │ (renderer.rs)    │  Handles nested styles correctly              │    │
│  │  └──────────────────┘                                               │    │
│  │                                                                     │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │  LEGACY PIPELINE (lopdf) - CANDIDATE FOR REMOVAL                    │    │
│  │  ═══════════════════════════════════════════════════════════════    │    │
│  │                                                                     │    │
│  │  PDF File                                                           │    │
│  │     │                                                               │    │
│  │     ▼                                                               │    │
│  │  ┌──────────────────┐                                               │    │
│  │  │ LopdfExtractor   │  Uses lopdf crate                             │    │
│  │  │ (extractor.rs)   │  Limited font metadata                        │    │
│  │  └────────┬─────────┘                                               │    │
│  │           │                                                         │    │
│  │           ▼                                                         │    │
│  │  ┌──────────────────┐                                               │    │
│  │  │ ContentParser    │  Parses PDF content streams                   │    │
│  │  │ (content.rs)     │  Manual text extraction                       │    │
│  │  └────────┬─────────┘                                               │    │
│  │           │                                                         │    │
│  │           ▼                                                         │    │
│  │  ┌──────────────────┐                                               │    │
│  │  │ Basic Blocks     │  Limited style detection                      │    │
│  │  │                  │  No font weight/style metadata                │    │
│  │  └──────────────────┘                                               │    │
│  │                                                                     │    │
│  │  ⚠️ ISSUES:                                                         │    │
│  │  - No reliable font style detection                                 │    │
│  │  - Manual PDF parsing is error-prone                                │    │
│  │  - Duplicate code with pdfium pipeline                              │    │
│  │  - Maintenance burden                                               │    │
│  │                                                                     │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Font Style Detection Algorithm

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    FONT STYLE DETECTION ALGORITHM                           │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Input: Font Name String (e.g., "TimesNewRoman-BoldItalic")                 │
│                                                                             │
│  Step 1: Normalize font name                                                │
│  ─────────────────────────────                                              │
│  "TimesNewRoman-BoldItalic" → "timesnewroman-bolditalic"                    │
│                                                                             │
│  Step 2: Check for BOLD indicators                                          │
│  ─────────────────────────────────                                          │
│  Patterns: "bold", "bd", "heavy", "black", "semibold", "demibold"           │
│                                                                             │
│  Step 3: Check for ITALIC indicators                                        │
│  ──────────────────────────────────                                         │
│  Patterns: "italic", "it", "oblique", "slant"                               │
│                                                                             │
│  Step 4: Check for MONOSPACE/CODE indicators                                │
│  ───────────────────────────────────────────                                │
│  Patterns: "mono", "courier", "consola", "menlo", "source code",            │
│            "fira code", "jetbrains", "inconsolata"                          │
│                                                                             │
│  Step 5: Combine results                                                    │
│  ───────────────────────                                                    │
│  if bold && italic → BoldItalic                                             │
│  if bold          → Bold                                                    │
│  if italic        → Italic                                                  │
│  if monospace     → Code                                                    │
│  else             → Normal                                                  │
│                                                                             │
│  Output: TextStyle enum                                                     │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Current Issues to Address

| Issue                | Location        | Priority | Description                                       |
| -------------------- | --------------- | -------- | ------------------------------------------------- |
| Dual pipelines       | `src/backends/` | HIGH     | Two extraction backends create maintenance burden |
| Font style loss      | `layout.rs`     | HIGH     | Styles may be lost during text grouping           |
| Clippy warnings      | Various         | MEDIUM   | Need zero-warning build                           |
| Missing WHY comments | All modules     | MEDIUM   | Code lacks explanation of design decisions        |
| Test coverage gaps   | `tests/`        | MEDIUM   | Edge cases not covered                            |
| Code duplication     | Multiple files  | LOW      | DRY principle violations                          |

---

## Process: OODA Loop (50 iterations minimum)

Execute iterative OODA cycles. Each iteration produces 4 files:

**You Must absolutely read your mission every iteration! It is vital to avoid alignment drift. You can forget previous iterations, but never forget your mission.**

**Mission file**: `specs/006-improve-pdf-markdown.md`

You Must always produce the 4 files per iteration, as shown below:

1. **observe.md** → Map the territory. Never make assumptions about code structure or function. Always verify against the actual codebase. When you don't know, go check the code or search on the web for answers and documentation
2. **orient.md** → Analyze your findings and define possible solutions using First Principles as your north star. Assess risks and benefits of each approach.
3. **decide.md** → Prioritize specific changes to be made based on signal value and impact.
4. **act.md** → Implement the decided changes with precision, update the documentation, and reference specific file:line numbers and commit SHAs.

```
specs/006-improve-pdf-markdown/ooda/
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

1. **Re-read mission** every iteration: mission file `specs/006-improve-pdf-markdown.md`
2. **Continue** from existing iterations—never restart
3. **Reference** real code: file paths, line numbers, commit SHAs
4. **Use ASCII diagrams** for architecture/flow visualization
5. **Split large files** for maintainability, Use Single Responsibility Principle (SRP)
6. **Optimize** Rust build speed (latest toolchain)
7. **Document amendments** in WHY, high signal value, and precise terms in comments in the codebase. Use ASCII diagrams where applicable.
8. **You must perform tests** and deliver evidence that all tests are passing after your changes.

**YOU Must Read your mission every iteration! It is vital to avoid alignment drift. You can forget previous iterations, but never forget your mission.**

You must always map the territory you are documenting. Never make assumptions about code structure or function. Always verify against the actual codebase.

If you don't know make a search on the Web.

Always use First Principle Thinking as your north star.

---

## Specific Focus Areas

### 1. Font Style Detection & Propagation

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    FONT STYLE PROPAGATION CHAIN                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  PDF Font Metadata                                                          │
│       │                                                                     │
│       │ extract_text_objects()                                              │
│       ▼                                                                     │
│  ┌─────────────┐                                                            │
│  │ PdfFont     │ Raw font info from pdfium                                  │
│  │ .name()     │ e.g., "Arial-BoldMT"                                       │
│  │ .weight()   │ e.g., 700 (bold threshold)                                 │
│  │ .is_fixed() │ monospace detection                                        │
│  └──────┬──────┘                                                            │
│         │                                                                   │
│         │ infer_style_from_font()                                           │
│         ▼                                                                   │
│  ┌─────────────┐                                                            │
│  │ TextStyle   │ Enum: Normal | Bold | Italic | BoldItalic | Code           │
│  └──────┬──────┘                                                            │
│         │                                                                   │
│         │ TextSpan::new()                                                   │
│         ▼                                                                   │
│  ┌─────────────┐                                                            │
│  │ TextSpan    │ Carries style through pipeline                             │
│  │ .style      │                                                            │
│  └──────┬──────┘                                                            │
│         │                                                                   │
│         │ ⚠️ CRITICAL: Style must survive grouping!                         │
│         │                                                                   │
│         │ group_into_lines() / merge_spans()                                │
│         ▼                                                                   │
│  ┌─────────────┐                                                            │
│  │ TextLine    │ Grouped spans with preserved styles                        │
│  │ .spans[]    │                                                            │
│  └──────┬──────┘                                                            │
│         │                                                                   │
│         │ render_to_markdown()                                              │
│         ▼                                                                   │
│  ┌─────────────┐                                                            │
│  │ Markdown    │ **bold**, *italic*, `code`                                 │
│  │ Output      │ ***bold italic***                                          │
│  └─────────────┘                                                            │
│                                                                             │
│  KEY INSIGHT: Styles must be preserved when merging adjacent spans          │
│  with the same style. Different styles = separate spans in output.          │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 2. Pipeline Consolidation Strategy

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    PIPELINE CONSOLIDATION PLAN                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  CURRENT STATE:                                                             │
│  ═════════════                                                              │
│                                                                             │
│  src/backends/                                                              │
│  ├── lopdf/           ← LEGACY: Manual PDF parsing                          │
│  │   ├── mod.rs                                                             │
│  │   ├── extractor.rs                                                       │
│  │   └── content.rs                                                         │
│  │                                                                          │
│  └── pdfium/          ← NEW: pdfium-rs bindings                             │
│      ├── mod.rs                                                             │
│      ├── extractor.rs                                                       │
│      ├── font.rs                                                            │
│      └── page.rs                                                            │
│                                                                             │
│  TARGET STATE:                                                              │
│  ═════════════                                                              │
│                                                                             │
│  src/backends/                                                              │
│  └── pdfium/          ← SINGLE SOURCE OF TRUTH                              │
│      ├── mod.rs       # Public API exports                                  │
│      ├── extractor.rs # Main extraction logic                               │
│      ├── font.rs      # Font detection (SRP)                                │
│      ├── page.rs      # Page-level extraction                               │
│      └── text.rs      # Text object handling                                │
│                                                                             │
│  src/backends/lopdf/  ← ARCHIVED or DELETED                                 │
│                                                                             │
│  MIGRATION STEPS:                                                           │
│  ════════════════                                                           │
│  1. Audit lopdf usage across codebase                                       │
│  2. Ensure pdfium covers all lopdf features                                 │
│  3. Update all callers to use pdfium                                        │
│  4. Add deprecation warnings to lopdf                                       │
│  5. Remove lopdf after verification                                         │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 3. SRP Module Organization

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    SINGLE RESPONSIBILITY MODULES                            │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Each module should have ONE reason to change:                              │
│                                                                             │
│  ┌─────────────────┐                                                        │
│  │ extractor.rs    │ RESPONSIBILITY: Coordinate extraction pipeline         │
│  │                 │ - Orchestrate page iteration                           │
│  │                 │ - Delegate to specialized modules                      │
│  │                 │ - Handle errors and recovery                           │
│  └─────────────────┘                                                        │
│                                                                             │
│  ┌─────────────────┐                                                        │
│  │ font.rs         │ RESPONSIBILITY: Font analysis only                     │
│  │                 │ - Detect style from font name                          │
│  │                 │ - Parse font weight/flags                              │
│  │                 │ - Map to TextStyle enum                                │
│  └─────────────────┘                                                        │
│                                                                             │
│  ┌─────────────────┐                                                        │
│  │ page.rs         │ RESPONSIBILITY: Page-level extraction                  │
│  │                 │ - Extract text objects from page                       │
│  │                 │ - Handle page rotation/transform                       │
│  │                 │ - Return raw TextSpan list                             │
│  └─────────────────┘                                                        │
│                                                                             │
│  ┌─────────────────┐                                                        │
│  │ layout.rs       │ RESPONSIBILITY: Spatial organization                   │
│  │                 │ - Group spans into lines                               │
│  │                 │ - Detect paragraphs and blocks                         │
│  │                 │ - Handle multi-column layouts                          │
│  └─────────────────┘                                                        │
│                                                                             │
│  ┌─────────────────┐                                                        │
│  │ renderer.rs     │ RESPONSIBILITY: Markdown generation                    │
│  │                 │ - Convert TextBlocks to Markdown                       │
│  │                 │ - Apply style formatting                               │
│  │                 │ - Handle special elements (tables, lists)              │
│  └─────────────────┘                                                        │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Success Criteria

| Criterion            | Metric                                       | Target |
| -------------------- | -------------------------------------------- | ------ |
| Zero clippy warnings | `cargo clippy 2>&1 \| grep warning \| wc -l` | 0      |
| All tests passing    | `cargo test` exit code                       | 0      |
| Font style accuracy  | Manual review of test PDFs                   | >95%   |
| Code coverage        | `cargo tarpaulin`                            | >80%   |
| Single backend       | Number of extraction backends                | 1      |
| WHY comments         | High-signal comments per module              | ≥3     |
| ASCII diagrams       | Diagrams in complex algorithms               | ≥5     |

---

## OODA Loop Directory

```
specs/006-improve-pdf-markdown/ooda/
```

Execute at least **50 OODA Loop iterations**.

---

## ⚠️ CRITICAL SAFETY MANDATE ⚠️

**YOU MUST RE-READ THIS ENTIRE MISSION FILE AT THE START OF EVERY OODA ITERATION.**

Failure to re-read causes alignment drift → catastrophic safety issues → user frustration → system unreliability.

---

## Quick Reference Commands

```bash
# Run all PDF crate tests
cargo test -p edgequake-pdf

# Run clippy with all warnings
cargo clippy -p edgequake-pdf -- -W clippy::all

# Build PDF crate only
cargo build -p edgequake-pdf

# Run specific test
cargo test -p edgequake-pdf test_font_style_detection

# Check test coverage (requires cargo-tarpaulin)
cargo tarpaulin -p edgequake-pdf --out Html
```

---

## Appendix: Key Files

| File                               | Purpose               | Lines (approx) |
| ---------------------------------- | --------------------- | -------------- |
| `src/backends/pdfium/extractor.rs` | Main extraction logic | ~400           |
| `src/backends/pdfium/font.rs`      | Font style detection  | ~150           |
| `src/layout.rs`                    | Text grouping/layout  | ~600           |
| `src/renderer.rs`                  | Markdown generation   | ~500           |
| `src/types.rs`                     | Core data structures  | ~200           |
| `src/lib.rs`                       | Public API            | ~100           |

---


VERY IMPORTANT Optimize the tests, splits the tests, compilation time, be more focus on order accelerate the OODA Loop Process. To have a high velocity feedback loop is important. Ensure to have timeout for every command to avoid to be stuck.

_Mission created: 2026-02-05_
_Target completion: 50+ OODA iterations_
