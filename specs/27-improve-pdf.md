You are tasked with improving the PDF to Markdown conversion quality of the Edgequake PDF processing library, specifically focusing on styles (bold, italic, headings) and tables. You will follow an OODA (Observe, Orient, Decide, Act) loop methodology to iteratively enhance the system in a measurable way.

**See also:** [PDF → Markdown Validator SKILL](.github/skills/pdf-markdown-validator/SKILL.md) — This specification now uses the production-ready validator skill for measurement. Refer to the SKILL documentation for metric definitions, validation workflows, and troubleshooting.

## Short goal

Improve PDF→Markdown quality for **styles** (bold/italic/heading detection) and **tables** (detection + cell extraction) using an OODA-driven, measurable, test-first loop.

## Single scalar objective (0–100)

We compute a single score in [0,100] using explicit sub-metrics and formulas:

**Note:** Use the PDF-Markdown Validator SKILL's `validate.py` script to compute these metrics automatically. See [.github/skills/pdf-markdown-validator/SKILL.md](.github/skills/pdf-markdown-validator/SKILL.md) for implementation details.

- **Table Accuracy (40% weight)**

  - _TableDetectionF1_: match predicted tables to gold via IoU >= 0.5 at the page level; compute precision/recall/F1.
  - _CellContentAccuracy_: mean token-level F1 across matched cells (unmatched cells count as zero).
  - TableAccuracy = 0.5 _ TableDetectionF1 + 0.5 _ CellContentAccuracy

- **Style Accuracy (40% weight)**

  - Evaluate tokens/spans for labels in {bold, italic, heading-level}. A heading is correct only if the predicted level equals the gold level.
  - StyleAccuracy = macro-average F1 across the label types (average F1 of bold, italic, heading-levels).

- **Robustness (10% weight)**

  - Percent of documents in the designated edge-case set (see `test-data/edge_cases/`) that process without crash and satisfy basic validity checks.

- **Performance (10% weight)**
  - Combine normalized median and P95 processing time relative to baseline:
    Performance = 0.5 _ min(1.0, baseline_median / run_median) + 0.5 _ min(1.0, baseline_p95 / run_p95)

**Aggregate score**

- WeightedSum = 0.40 _ TableAccuracy + 0.40 _ StyleAccuracy + 0.10 _ Robustness + 0.10 _ Performance
- FinalScore = round(WeightedSum \* 100) # sub-metrics are in [0,1]

**Hard gates (score = 0 if any fail)**

- `cargo test -p edgequake-pdf` must pass (exit code 0)
- No crashes (no panics or non-zero exit codes) when running the evaluator on `crates/edgequake-pdf/test-data/real_dataset/*`
- All generated Markdown files must be syntactically valid: `pandoc -f markdown -t html -o /dev/null file.md` must exit with code 0 for every output file (recommend `pandoc >= 2.14`). This check is performed automatically by the validator SKILL.

## Dataset & Execution

**Validation Setup:**
Use the PDF-Markdown Validator SKILL to measure quality across iterations. Prepare your test data as follows:

1. **Markdown Files:** Generate with `cargo run -p edgequake-pdf --example real_dataset_eval -- --write`
2. **Gold Files:** Create `.gold.md` reference files for each PDF (see SKILL documentation for annotation guidelines)
3. **Run Validation:** Use `validate.py` to compute table/style/robustness/performance scores
4. **Analyze Drifts:** Use `diff_analysis.py` and `batch_drift.py` to identify error patterns

- Dataset: `crates/edgequake-pdf/test-data/real_dataset/` (with `.gold.md` ground-truth files)
- Baseline harness: `examples/real_dataset_eval.rs` (extend to include table/style F1 and artifact counters)

**Note:** Manual metric formulas in this document are for transparency. Use the PDF-Markdown Validator SKILL's `validate.py` script to compute these metrics automatically; the SKILL is the source of truth for iterative measurement.

**Performance note:** The performance metric calculation is defined here for reference. The SKILL currently uses a placeholder for performance profiling; see the SKILL documentation for integration and P95/P50 collection details.
- Validation scripts: `.github/skills/pdf-markdown-validator/scripts/` (validate.py, analyze_failures.py, diff_analysis.py, batch_drift.py)
- Append-only log: `crates/edgequake-pdf/sessions/improve_pdf/scratchpad_append_log.md`
- Per-iteration artifacts: `OBSERVE.md`, `ORIENT.md`, `DECIDE.md`, `PATCH.diff`, `*.mdf.gen` in `crates/edgequake-pdf/sessions/improve_pdf/001-iteration/` etc.

## Reproducibility & Environment

- Document required tool versions in `crates/edgequake-pdf/sessions/improve_pdf/README.md` (recommended minimums: `rustup` stable toolchain, `cargo` from the same toolchain, `pandoc >= 2.14`).
- Make evaluator runs deterministic where possible: add a `--seed` flag to `real_dataset_eval.rs` and use it in CI runs when comparing metrics.
- Pin runtime environment in session README and mention any environment variables needed (e.g., `RUSTFLAGS`, `CARGO_HOME`).
- Provide a small wrapper script `scripts/run_eval.sh` that sets the recommended environment and runs the evaluator with reproducible flags.

## OODA Automation Contract (for each iteration)

**Scope rule — one directory per OODA loop**

- Each iteration MUST target a single directory (module) in the repository (e.g., `crates/edgequake-pdf/src/processors`, `crates/edgequake-pdf/src/backend`, `crates/edgequake-pdf/src/renderers`, `crates/edgequake-pdf/tests`, `examples/`). Declare the target directory at the start of the iteration (in `OBSERVE.md`) and keep the scope for Orient / Decide / Act limited to that directory. While the SKILL measures and reports at the file level, iterations should remain directory-scoped for code changes and diagnosis. Prioritize highest-impact directories first.

1. Observe

- Run (scoped to the chosen directory): `cargo test`, `cargo run -p edgequake-pdf --example real_dataset_eval -- --write`, perf timing
- Capture: failing tests, `*.md.gold.gen` diffs, error logs, perf deltas, and directory-scoped file diffs
- Produce: `OBSERVE.md` and append summary to `scratchpad_append_log.md` (include `Directory: <path>` header)

2. Orient

- Diagnose: locate failing subsystem within the selected directory (layout, `TJ` kerning, hyphenation, table lattice, spans vs. block.text)
- Research (scoped): fetch repo examples and papers that relate directly to the directory's concerns; summarize patterns
- Produce: `ORIENT.md` (diagnosis + citations + candidate approaches) that explicitly references the directory

3. Decide

- Propose 1–3 small patches (limited to the chosen directory and its tests) with predicted score impact and a specific acceptance checklist (map to CI signals)
- Select the smallest patch expected to improve the score
- Produce: `DECIDE.md` (patch plan + checklist) referencing the directory

4. Act

- Implement the patch within the chosen directory (and its tests)
- Add tests (unit, regression, property/fuzz where appropriate) in the same directory or `tests/`
- Run `cargo test` and the extended eval; record metrics and diffs scoped to the directory
- Commit small, well-documented changes; include `PATCH.diff` and mention `Directory: <path>` in commit message

Stop rule: stop after 20 OODA iterations or when the **average improvement over the last 3 iterations** is < 5 points (knee rule), or when there are 3 consecutive iterations with no statistically significant improvement. Document the rationale in `DECIDE.md` when stopping early.

## Sequential-thinking tool usage (MANDATORY)

Use mcp_sequentialthi_sequentialthinking for every planning and decision step and make the directory the primary unit of focus. Note: the tool does not accept a dedicated `directory` parameter, so include the directory at the start of the `thought` string.

Follow these rules:

- Each thought call must include (tool fields):
  - `thought`: 1–2 concise sentences describing the step or finding. **Prefix this string with** `Directory: <path>` so the directory is captured (e.g., `Directory: crates/edgequake-pdf/src/processors — Observe failing outputs...`).
  - `thoughtNumber`: sequential index (start at 1)
  - `totalThoughts`: estimate for the phase (adjustable)
  - `nextThoughtNeeded`: true if more analysis is required, false when ready to Act
  - Use `isRevision` and `revisesThought` when revising earlier thoughts
- Typical sequence per iteration (include directory in Thought 1):
  1. Thought 1: Declare the target `directory` in `OBSERVE.md` (nextThoughtNeeded=true)
  2. Thought 2: Observe outputs and failing examples scoped to the directory (nextThoughtNeeded=true)
  3. Thought 3: Orient — inspect processors/backend in the directory to find root cause (nextThoughtNeeded=true)
  4. Thought 4: Decide — pick minimal patch limited to the directory with acceptance checklist (nextThoughtNeeded=false)
  5. Thought 5: Act — implement patch, run tests, report results (nextThoughtNeeded=false)
- Always call the sequential-thinking tool before modifying code and after running tests to summarize outcomes and include the `Directory: <path>` prefix in the `thought` text. Use the SKILL for measurement and diagnostics; use the sequential-thinking tool for planning, decision records, and traceability.

Example call (illustrative):

```json
{
  "thought": "Directory: crates/edgequake-pdf/src/processors — Observe failing outputs for table lattice cases and collect diffs (nextThoughtNeeded=true)",
  "thoughtNumber": 1,
  "totalThoughts": 4,
  "nextThoughtNeeded": true
}
```

Call the sequential-thinking tool after the Observe step and again after running tests so each iteration has a compact, auditable thought trace.

## Patch acceptance checklist (per patch)

- New test demonstrates failure before patch and passes after
- `cargo test -p edgequake-pdf` passes
- Validator SKILL report shows targeted metric improvement and no regressions
- Append iteration summary to `scratchpad_append_log.md` and produce `OBSERVE/ORIENT/DECIDE/PATCH.diff`

## Example first patch (minimal, high ROI)

Patch: Span-level whitespace normalization + concatenated-word fixes (apply to `block.spans` as well as `block.text`), add unit tests for span boundary handling and concatenated-word cases.

- Why: `MarkdownRenderer` uses spans preferentially; unnormalized spans produce camel joins and double spaces
- Acceptance: Unit tests pass; validator SKILL confirms improvement in targeted style/cell metrics; `real_dataset_eval` shows decreased camel-join counter on `AlphaEvolve` and `one_tool_*.pdf`

_Tip:_ add a short `README.md` under `crates/edgequake-pdf/sessions/improve_pdf/` describing the OODA loop process, required tools/versions, and how to reproduce an iteration.

## Safety, IP & Research policy

- When mining GitHub, only extract high-level patterns/algorithms; quote code snippets only when license-compatible and record provenance
- Produce a short provenance note for each external repo or paper used

## Operational Commands (quick)

- Run tests: `cd edgequake && cargo test -p edgequake-pdf`
- Run evaluation: `cargo run -p edgequake-pdf --example real_dataset_eval -- --write`
- **Measure quality:** `python3 .github/skills/pdf-markdown-validator/scripts/validate.py --pdf-dir crates/edgequake-pdf/test-data --gold-dir crates/edgequake-pdf/test-data --output-report metrics.json`
- **Analyze failures:** `python3 .github/skills/pdf-markdown-validator/scripts/analyze_failures.py metrics.json --verbose`
- **Detailed drift analysis:** `python3 .github/skills/pdf-markdown-validator/scripts/batch_drift.py --pdf-dir crates/edgequake-pdf/test-data --gold-dir crates/edgequake-pdf/test-data --output-report drift_report.json`

---

Start by executing the Observe step now: run tests and the extended evaluator with `--write`, generate metrics using the validator SKILL, capture drifts and perf, append `OBSERVE.md`, then call mcp_sequentialthi_sequentialthinking with Thought 1 describing the observation and your next planned thought.

## Troubleshooting

- For common issues and solutions, see the PDF-Markdown Validator SKILL [README](.github/skills/pdf-markdown-validator/README.md).
- If metrics are unexpectedly low, use the drift analysis scripts (`diff_analysis.py`, `batch_drift.py`) to identify error patterns and root causes.
- For environment setup, gold annotation guidelines, and advanced workflows, see the SKILL documentation.
