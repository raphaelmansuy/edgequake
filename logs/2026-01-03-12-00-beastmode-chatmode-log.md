Actions:

- Added PDF fallback lookup in `tests/comprehensive_evaluation.rs` to find PDFs when missing in `pdfs/<category>`.
- Wrote unified-style per-document diff outputs to `test-data/diffs/<category>/<doc>.diff`.
- Added `test-data/generate_simple_pdfs.py` to generate fallback PDFs using ReportLab.
- Added `test-data/run_full_evaluation.sh` to orchestrate PDF generation and the Rust evaluation run.
- Added `tests/test_gold_dataset_size.rs` to assert gold dataset >= 100 docs.
- Added `tests/smoke_evaluation.rs` (non-ignored) to smoke-test extraction on first available PDF.

Decisions:

- Use ReportLab as a fallback PDF generator to avoid requiring `pandoc` in CI.
- Produce simple unified diffs for human inspection; metrics are still computed in the evaluation report.

Next steps:

- Run `python3 test-data/generate_simple_pdfs.py` to create PDFs (requires reportlab).
- Run `test-data/run_full_evaluation.sh` to perform full evaluation (this runs the Rust ignored test with `--ignored`).

Lessons/insights:

- Gold dataset already met the >=100 requirement (110 documents across 10 categories).
- Some existing PDFs existed at the top-level; PDF fallback lookup helps use them without strict naming.
