# edgequake-pdf — scratchpad append-only log

This file is append-only.
It records OODA-loop observations, decisions, and outcomes while improving real-dataset PDF→Markdown quality.

## Session start

- Date: 2026-01-02
- Scope: edgequake/crates/edgequake-pdf + test-data/real_dataset/\*.pdf
- Note: This log intentionally avoids private chain-of-thought; it records observable results + decisions.

## OODA 01

- Observe:
  - `test-data/real_dataset/` contains paired `.pdf` plus `.md` and `.mdf` outputs.
  - The CLI `edgequake-pdf convert` (default `--page_numbers=false`) generates output matching existing `.mdf` (no `## Page N` sections).
  - The example outputs (`*.md`) appear to be generated with `include_page_numbers=true` (default `PdfConfig`).
- Orient:
  - Treat `.mdf` as the “final” baseline output to improve.
  - Focus areas: reading order (multi-column), header/footer filtering, caption placement, spacing/word-join artifacts.
- Decide:
  - Add a small evaluation harness to run extraction on all real-dataset PDFs and emit stable metrics.
- Implement:
  - Generated `ccn_2512.21804v1.mdf.new` via CLI; it matches current `ccn_2512.21804v1.mdf`.

## OODA 02

- Observe:
  - Markdown rendering prefers `block.spans` over `block.text` when spans are present.
  - `PostProcessor` was only applying concatenated-word/citation cleanup to `block.text`, not to spans.
  - Pattern counters suggested “camelCase joins” persisted even though `PostProcessor` existed.
- Orient:
  - If spans are not cleaned, renderer outputs will keep raw spacing/word-join artifacts.
  - Also, trimming span edges breaks word separation across spans.
- Decide:
  - Apply the same cleanup to spans as `block.text` (except for code-like spans), and preserve span boundary whitespace.
- Act:
  - Updated `PostProcessor` to:
    - add `normalize_span_text()` (collapse spaces/tabs without trimming edges)
    - apply concatenated-word split + citation cleanup to non-code spans
    - normalize double-spaces across span boundaries
  - Added unit tests around span cleanup + boundaries.
- Outcome (real_dataset_eval, no writes):
  - Dataset note: only `AlphaEvolve.pdf` and `one_tool_2512.20957v2.pdf` currently have `.mdf` gold files.
  - Observed substantial reductions in missing-space camel joins (qualitative), with stable test suite passing.

## OODA 03

- Observe:
  - Broad camelCase splitting can accidentally break legitimate tokens like `arXiv`.
- Decide:
  - Keep the broad splitting (it fixes real missing-space joins), but add a targeted repair for `arXiv`.
- Act:
  - Added a minimal `arXiv` repair (`ar Xiv` → `arXiv`) and a regression test.
- Outcome (real_dataset_eval, no writes):
  - Example snapshot (patterns):
    - `AlphaEvolve`: `camel_join` reduced from 201 → 54 (baseline → current), `arxiv_header` preserved.
    - `one_tool_2512.20957v2`: `camel_join` metric increased vs earlier baseline; this counter likely over-counts legitimate camel tokens (not just missing-space joins). `double_space` unchanged (likely tables/markdown artifacts).

### OODA 03 — correction

- The `camel_join` counter is a rough heuristic and can increase when legitimate camel tokens become more visible/preserved.
