## Improve PDF → Markdown (OODA)

This directory contains reproducible OODA iterations for improving `edgequake-pdf` PDF → Markdown quality.

### Prerequisites

- Rust toolchain (stable)
- Python 3.9+
- `pandoc` >= 2.14 (used by the validator)

### Baseline / Evaluation Commands

- Run PDF crate tests: `cd edgequake && cargo test -p edgequake-pdf`
- Generate `*.mdf.gen` outputs: `cd edgequake && cargo run -p edgequake-pdf --example real_dataset_eval -- --write`
- Compute metrics: `python3 .github/skills/pdf-markdown-validator/scripts/validate.py --pdf-dir edgequake/crates/edgequake-pdf/test-data/real_dataset --gold-dir edgequake/crates/edgequake-pdf/test-data/real_dataset --output-report sessions/improve_pdf/metrics.json`
- Drift report: `python3 .github/skills/pdf-markdown-validator/scripts/batch_drift.py --pdf-dir edgequake/crates/edgequake-pdf/test-data/real_dataset --gold-dir edgequake/crates/edgequake-pdf/test-data/real_dataset --output-report sessions/improve_pdf/drift.json`

### Iterations

Each `NNN-iteration/` folder contains:

- `OBSERVE.md`: baseline measurements + concrete failures
- `ORIENT.md`: root-cause hypothesis linked to code
- `DECIDE.md`: minimal patch plan + acceptance checklist
- `ACT.md`: what changed + verification results
- `PATCH.diff`: the exact diff applied in that iteration

### Notes

- The spec references a “sequential-thinking” MCP tool. In this Codex CLI run, that tool is not available, so traceability is recorded in the iteration markdowns and `scratchpad_append_log.md` instead.

