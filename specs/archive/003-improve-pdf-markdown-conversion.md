# Mission: Improve PDF to Markdown Conversion Quality

## Task

Your mission is to achieve near-perfect PDF to Markdown conversion by:

1. Analyzing the current conversion issues with `zz-explore/001-BEYONG-TRANFORMER-OUTLINE-V1_1.pdf`
2. Defining robust E2E test strategy with quality metrics
3. Building a comprehensive test corpus representing diverse PDF types
4. Perfecting the PDF conversion algorithm based on test feedback and PDF internals and text layout principles, ensuring high fidelity in structure, text, and semantics. Remember Donald Knuth's maxim: "Beware of bugs in the above code; I have only proved it correct, not tried it." Remember Donald Knuth text layout principles: character spacing, word spacing, line height, paragraph spacing, reading order, font encoding, glyph mapping, etc.

Try to optimize tests and multi threading tests run as much as possible as it increase your productivity during OODA loops.

## Context

- **Location**: `edgequake/crates/edgequake-pdf/`
- **Problem PDF**: `zz-explore/001-BEYONG-TRANFORMER-OUTLINE-V1_1.pdf`
- **Target**: Production-grade PDF → Markdown conversion with measurable quality

Awlays reason from First Principles. Avoid assumptions. Verify everything against the actual codebase.

Use high signal ASCII diagrams to illustrate architecture, algorithms, and data flows in comments and documentation.


New additional context for PDF internals:

Always find PDF specification references when dealing with PDF internals, think always about how letter spacing, font encoding, and text positioning affect extraction quality and is implemented in PDF --> space between characters, words, lines, paragraphs are all determined by font metrics and positioning operators and complex rendering rules ---> check PDF spec for details and font spacing calculation formulas.

---

## Process: OODA Loop (50 iterations minimum)

Execute iterative OODA cycles. Each iteration produces 4 files:

You Must absolutely read your mission every iteration! It is vital to avoid alignment drift. You can forget previous iterations, but never forget your mission.

Mission file: `specs/003-improve-pdf-markdown-conversion.md`

You Must always produce the 4 files per iteration, as shown below:

1. `observe.md` → Map the territory. Never make assumptions about code structure or function. Always verify against the actual codebase. When you don't know, go check the code or search on the web for answers and documentation
2. `orient.md` → Analyze your findings and define possible solutions using First Principles as your north star. Assess risks and benefits of each approach.
3. `decide.md` → Prioritize specific changes to be made based on signal value and impact.
4. `act.md` → Implement the decided changes with precision, update the documentation, and reference specific file:line numbers and commit SHAs.

```
specs/003-improve-pdf-markdown-conversion/ooda_loop/
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

1. **Re-read mission** every iteration: mission file `specs/003-improve-pdf-markdown-conversion.md`
2. **Continue** from existing iterations—never restart
3. **Reference** real code: file paths, line numbers, commit SHAs
4. **Use ASCII diagrams** for architecture/flow visualization
5. **Split large files** for maintainability, Use Simple Responsibility Principle (SRP)
6. **Optimize** Rust build speed (latest toolchain)
7. **Document amendments** in WHY, high signal value, and precise terms in comments in the codebase. Use ASCII diagrams where applicable.
8. **You must perform tests** and deliver evidence that all tests are passing after your changes.

---

## Quality Metrics Framework

### Core Metrics (to be refined during OODA iterations)

| Metric                     | Description                               | Target |
| -------------------------- | ----------------------------------------- | ------ |
| **Structure Preservation** | Headers, lists, tables detected correctly | ≥95%   |
| **Text Fidelity**          | Character-level accuracy                  | ≥99%   |
| **Layout Accuracy**        | Column detection, reading order           | ≥90%   |
| **Semantic Integrity**     | Meaning preserved across conversion       | ≥95%   |

### Test Corpus Categories

1. **Simple Documents**: Plain text, single column
2. **Academic Papers**: Multi-column, figures, citations
3. **Technical Manuals**: Tables, code blocks, diagrams
4. **Reports**: Headers, footers, page numbers
5. **Forms**: Input fields, checkboxes
6. **Slides/Presentations**: Bullets, graphics-heavy

---

## ⚠️ CRITICAL SAFETY MANDATE ⚠️

**YOU MUST RE-READ THIS ENTIRE MISSION FILE AT THE START OF EVERY OODA ITERATION.**

Failure to re-read causes alignment drift → catastrophic safety issues → user frustration → system unreliability.

---

## Deliverables

1. **Fixed PDF conversion** for the problem document
2. **Quality metrics framework** with automated scoring
3. **E2E test suite** covering diverse PDF types
4. **Improved conversion algorithm** with documented changes
5. **Gold standard corpus** for regression testing
