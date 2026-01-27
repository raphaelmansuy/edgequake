# OBSERVE.md - Iteration 003

**Directory:** `edgequake/crates/edgequake-pdf/src`

## Baseline Commands

- `cd edgequake && cargo test -p edgequake-pdf`
- `cd edgequake && cargo run -p edgequake-pdf --example real_dataset_eval -- --write`
- `python3 .github/skills/pdf-markdown-validator/scripts/validate.py --pdf-dir edgequake/crates/edgequake-pdf/test-data/real_dataset --gold-dir edgequake/crates/edgequake-pdf/test-data/real_dataset --output-report sessions/improve_pdf/metrics_baseline.json`
- `python3 .github/skills/pdf-markdown-validator/scripts/batch_drift.py --pdf-dir edgequake/crates/edgequake-pdf/test-data/real_dataset --gold-dir edgequake/crates/edgequake-pdf/test-data/real_dataset --output-report sessions/improve_pdf/drift_baseline.json`

## Baseline Metrics (Real Dataset)

- Table Accuracy: 3.5%
- Style Accuracy: 16.9%
- Robustness: 100.0%
- Performance: 90.0%
- Composite: 27.2/100

## Concrete Failures (examples)

### Headings

- `one_tool_2512.20957v2`: title emitted as `###` instead of `#`; numeric section headers (e.g., `1. Introduction`) often remain plain text.

### Styles

- Inline bold/italic from the PDFs is largely missing. Many lines are emitted as a single span so mixed-style runs cannot be preserved.

### Tables

- Caption-adjacent reconstruction frequently chooses the wrong nearby lines and/or creates “Value” fallback tables containing unrelated prose (e.g. Table 3 in `one_tool_2512.20957v2`).

