# OBSERVE — Iteration 001
- Directory: `crates/edgequake-pdf/src/renderers`
- Date: 2026-01-02T01:54:29.799Z
- Dependency fix: Switched `lopdf` to crates.io (`0.32`, feature `nom_parser`) to replace broken local path symlink.
- Commands run:
  - `cargo test -p edgequake-pdf` → **fails** (15 failing tests in `tests/edge_cases_and_complex.rs`, e.g., `validate_02_multipage_content`, `validate_03_table_content` asserts empty markdown; others cover rotated text, mixed directions, etc.).
  - `cargo run -p edgequake-pdf --example real_dataset_eval -- --write` → **succeeds**; all five PDFs produce `.mdf.gen` with F1 = 0.000 and high camel-join/double-space counts (e.g., AlphaEvolve camel_join=54, double_space=248; one_tool_2512.20957v2 double_space=828).
- Consequence: Renderer output quality is extremely low (zero F1, many spacing artifacts). Baseline established but style/table metrics effectively zero.
- Next: Keep scope on `crates/edgequake-pdf/src/renderers`; diagnose why markdown generation drops all structure/styles and leaves spacing noise.
