# OODA Loop Prompt: Improve edgequake-pdf PDF-to-Markdown Accuracy (Styles & Tables)

## Core Objective

Generate a high-accuracy PDF-to-Markdown converter for **styles** (bold, italic, font sizes, headers) and **tables** (structure, cell content, alignment) using the `edgequake/crates/edgequake-pdf` crate.

**Measurable Score (0-100)**:

- **Table Accuracy (40%)**: Precision/recall of table detection + cell content extraction (use `test-data/real_dataset/*.pdf` with manual ground-truth annotations).
- **Style Accuracy (40%)**: F1-score for style spans (bold/italic/header levels) vs. ground-truth Markdown.
- **Robustness (10%)**: Pass rate on edge cases (empty tables, nested styles, multi-column tables).
- **Performance (10%)**: Processing time < 2x baseline on real_dataset PDFs.

**Hard Gates (Score = 0 if any fail)**:

- Unit tests pass (`cargo test -p edgequake-pdf`).
- No crashes on real_dataset PDFs.
- Output is valid Markdown (pandoc parseable).

## OODA Loop Contract

### Observe

- Run extraction on all `test-data/real_dataset/*.pdf`.
- Capture: table detection errors, style misclassifications, performance metrics.
- Generate `OBSERVE.md`: failing examples, error logs, score breakdown.

### Orient

- Diagnose root causes: layout analysis bugs, style heuristics failures, table lattice issues.
- Research (bounded): Pull 3-5 GitHub repos (e.g., pdf2md, tabula, camelot) for table/style extraction patterns.
- Summarize 2-3 papers on PDF parsing (e.g., layout analysis, font style detection).
- Produce `ORIENT.md`: diagnosis, citations, proposed improvements.

### Decide

- Propose 1-3 minimal patches with predicted score impact.
- Acceptance checklist: "After patch, table F1 > X, style F1 > Y".
- Choose smallest patch expected to improve score.
- Produce `DECIDE.md`: chosen plan, checklist.

### Act

- Implement patch in `edgequake/crates/edgequake-pdf/src/`.
- Add/update tests to lock behavior.
- Run full evaluation; update score.
- Produce `PATCH.diff`.

## Stop Criteria

- Stop after 20 iterations or when score gains < 5 points/iteration.
- Avoid over-complexity; prefer algorithmic fixes over hacks.

## System Setup

- Workspace: `edgequake/crates/edgequake-pdf`.
- Test data: `test-data/real_dataset/` (5 PDFs with existing `.mdf` outputs).
- CI: `cargo test`, `cargo run --example real_dataset_eval`.
- Memory: Append to `scratchpad_append_log.md` per iteration.

## Example Benchmarks

- Table: Detect 100% of tables in `AlphaEvolve.pdf`, render as proper Markdown tables.
- Styles: Correctly identify H1/H2 from font size, bold spans from font weight.

## Failure Modes to Avoid

- Overfitting: Use diverse PDFs, not just real_dataset.
- Regression: Require tests for new behavior.
- Local optima: Force research resets every 5 iterations.

## Next Steps

1. Run initial Observe to baseline score.
2. Implement first patch (e.g., improve table lattice detection).
3. Iterate until score plateaus.</content>
   <parameter name="filePath">/Users/raphaelmansuy/Github/03-working/edgequake/ooda_prompt_improve_pdf.md
