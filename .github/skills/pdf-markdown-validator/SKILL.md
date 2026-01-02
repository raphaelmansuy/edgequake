---
name: pdf-markdown-validator
description: Validate PDF to Markdown conversion quality using multi-dimensional metrics. Assess table accuracy, style preservation (bold/italic/headings), robustness, and performance with standardized F1-scoring methodology.
license: Proprietary (repository internal)
compatibility: Requires Rust (1.70+), Python (3.9+), pandoc (2.14+), and EdgeQuake PDF crate
metadata:
  repo: raphaelmansuy/edgequake
  area: pdf-processing
  languages:
    - Rust
    - Markdown
    - Python
  frameworks:
    - edgequake-pdf
  patterns:
    - Quality metrics
    - Evaluation harness
    - Ground-truth comparison
---

# PDF → Markdown Validator Skill

Validate PDF to Markdown conversion quality using a comprehensive, multi-dimensional evaluation framework. This skill provides standardized metrics, evaluation harnesses, and reporting tools to assess conversion fidelity across table accuracy, style preservation, robustness, and performance.

## When to use

Use this skill when you need to:

- **Validate PDF extraction quality**: Measure how accurately PDFs convert to Markdown
- **Compare conversion approaches**: Benchmark different PDF processing implementations
- **Track quality improvements**: Quantify gains from processing enhancements
- **Automate quality gates**: Enforce minimum quality standards in CI/CD pipelines
- **Generate evaluation reports**: Create detailed analysis of conversion successes and failures
- **Identify failure patterns**: Discover systematic issues in specific PDF features
- **Measure performance regressions**: Track processing speed alongside quality metrics

## Core concepts

### Validation Framework

The validation framework computes a **composite quality score (0–100)** combining four independent metric dimensions:

```
FinalScore = (0.40 × TableAccuracy) + (0.40 × StyleAccuracy)
           + (0.10 × Robustness) + (0.10 × Performance)
```

Each dimension is independent and can be evaluated separately or together.

### 1. Table Accuracy (40% weight)

Measures how accurately tables are detected and their cell content extracted.

**Components:**

- **Table Detection F1**: IoU-based matching of predicted vs. gold tables (IoU ≥ 0.5 threshold)

  - Precision: correctly identified tables / all detected tables
  - Recall: correctly identified tables / all gold tables
  - F1: harmonic mean of precision and recall

- **Cell Content Accuracy**: Token-level F1 averaging across matched table cells
  - Matches cells by position within detected tables
  - Unmatched cells score 0.0
  - Aggregates to mean F1 across all cells

**Formula:**

```
TableAccuracy = (0.5 × TableDetectionF1) + (0.5 × CellContentAccuracy)
```

**Interpretation:**

- **90–100**: Excellent table handling; tables detected and content extracted accurately
- **70–89**: Good; minor detection misses or cell content variations
- **50–69**: Fair; tables detected but cells have significant content errors
- **Below 50**: Poor; systematic table detection failures or content corruption

### 2. Style Accuracy (40% weight)

Measures how accurately text formatting (bold, italic, heading levels) is preserved.

**Components:**

- **Bold F1**: Token-level F1 for bold detection
- **Italic F1**: Token-level F1 for italic detection
- **Heading F1**: Heading-level accuracy (prediction must match gold level exactly; e.g., H2 ≠ H3)

**Formula:**

```
StyleAccuracy = macro_average(BoldF1, ItalicF1, HeadingF1)
              = (BoldF1 + ItalicF1 + HeadingF1) / 3
```

**Interpretation:**

- **90–100**: Styles consistently preserved; formatting markup matches gold
- **70–89**: Good; minor style detection misses
- **50–69**: Fair; mixed style accuracy; some styles missed consistently
- **Below 50**: Poor; styles largely lost or misdocumented

### 3. Robustness (10% weight)

Measures system stability and validity across a test corpus, particularly edge cases.

**Components:**

- **Crash-free rate**: Percent of documents processed without panics or non-zero exit codes
- **Markdown validity**: Percent of generated Markdown files passing `pandoc` syntax validation
- **Completeness**: Percent of documents producing non-empty output

**Formula:**

```
Robustness = (CrashFreeRate + MarkdownValidityRate + CompletenessRate) / 3
```

**Interpretation:**

- **95–100**: Production-ready; system handles edge cases gracefully
- **85–94**: Stable; occasional edge-case issues but recovers
- **70–84**: Moderate stability; known failure patterns on specific input types
- **Below 70**: Fragile; systematic crashes or invalid output on subset of inputs

### 4. Performance (10% weight)

Measures processing speed relative to a baseline; targets 1-page PDF ≈ 200–500ms.

**Components:**

- **Median latency**: P50 processing time across corpus
- **P95 latency**: 95th percentile processing time (captures tail behavior)

**Formula:**

```
Performance = 0.5 × min(1.0, baseline_median / run_median)
            + 0.5 × min(1.0, baseline_p95 / run_p95)
```

**Interpretation:**

- **95–100**: Fast processing; meets or exceeds baseline
- **80–94**: Good; acceptable slowdown (≤20%)
- **60–79**: Acceptable; noticeable overhead (20–60%)
- **Below 60**: Slow; significant performance regression

## Quick start

### 1. Prepare ground-truth annotations

Create `.gold.md` files for each test PDF:

```bash
# Copy reference Markdown next to PDF with .gold.md extension
cp reference_output.md test.gold.md

# Format: Each section annotated with metadata
# Gold format example:
# # Heading 1
# **bold text** and *italic text*
#
# | Column A | Column B |
# |----------|---------|
# | cell 1   | cell 2  |
```

### 2. Run validation

```bash
# Evaluate against ground truth
cargo run -p edgequake-pdf --example real_dataset_eval -- \
  --input crates/edgequake-pdf/test-data/real_dataset \
  --gold \
  --metrics

# Generate detailed report
python3 .github/skills/pdf-markdown-validator/scripts/validate.py \
  --pdf-dir crates/edgequake-pdf/test-data/real_dataset \
  --gold-dir . \
  --output-report metrics_report.json
```

### 3. Interpret results

```bash
# View summary scores
cat metrics_report.json | jq '.summary'

# Analyze failures by category
python3 .github/skills/pdf-markdown-validator/scripts/analyze_failures.py \
  metrics_report.json
```

## Capabilities

### Core Capabilities

#### 1. Metric Computation

- **Token-level F1 calculation**: Boolean/classification metrics at granular text unit level
- **Spatial matching**: IoU-based table detection using bounding box coordinates
- **Aggregate averaging**: Macro-averaging (label-balanced) and weighted averaging strategies
- **Normalization**: Robust handling of missing annotations and edge cases

#### 2. Evaluation Harness

- **Batch evaluation**: Process entire corpora in single invocation
- **Incremental scoring**: Support partial annotations and progressive refinement
- **Performance profiling**: Automatically capture timing with statistical summaries (median, P95)
- **Error categorization**: Classify failures into systematic buckets (missed tables, style loss, etc.)

#### 3. Reporting & Visualization

- **JSON reports**: Machine-readable output for CI/CD integration
- **Markdown summaries**: Human-friendly reports with trend analysis
- **Per-document breakdown**: Detailed scores for each file with failure reasons
- **Comparison mode**: Side-by-side analysis of baseline vs. current runs

#### 4. Validation Gates

- **Hard gates**: Fail evaluation if Markdown is invalid or crashes occur
- **Soft gates**: Warn if quality drops below threshold but allow pass
- **Trend analysis**: Flag regressions even if absolute score passes
- **Baseline anchoring**: Detect performance slowdowns vs. previous runs

### Integration Points

#### With Cargo Testing

```bash
# Embed metrics in standard cargo test output
cargo test -p edgequake-pdf -- --nocapture

# Fail CI if composite score below threshold
cargo test -p edgequake-pdf --features ci-strict
```

#### With Python Evaluation Scripts

```python
from pdf_validator import PDFValidator

validator = PDFValidator(
    pdf_dir="test-data/real_dataset",
    gold_dir="test-data/gold",
    metrics=["table", "style", "robustness", "performance"]
)
score = validator.evaluate()
print(f"Composite Score: {score.composite}/100")
```

#### With CI/CD Pipelines

```yaml
# GitHub Actions example
- name: Validate PDF → Markdown
  run: |
    cargo run -p edgequake-pdf --example real_dataset_eval -- --metrics
    python .github/skills/pdf-markdown-validator/scripts/validate.py \
      --ci-mode --fail-below 75
```

## Metric Definitions & Examples

### Table Accuracy Example

**Input PDF:** 2×3 table with headers "Name, Age" and row "John, 25"

**Gold Markdown:**

```markdown
| Name | Age |
| ---- | --- |
| John | 25  |
```

**Generated Markdown (Perfect):**

```markdown
| Name | Age |
| ---- | --- |
| John | 25  |
```

**Scores:**

- TableDetectionF1: 1.0 (table detected correctly)
- CellContentAccuracy: 1.0 (all cells match token-for-token)
- TableAccuracy: **1.0**

**Generated Markdown (Partial Match):**

```markdown
| Name | Age  |
| ---- | ---- |
| John | 25.0 |
```

**Scores:**

- TableDetectionF1: 1.0 (table structure preserved)
- CellContentAccuracy: 0.8 (minor token variations: "25" vs "25.0")
- TableAccuracy: **0.9**

### Style Accuracy Example

**Gold Markdown:**

```markdown
# Main Heading

This is **bold** and _italic_ text.

## Sub Heading

More content here.
```

**Generated Markdown (Perfect):**

```markdown
# Main Heading

This is **bold** and _italic_ text.

## Sub Heading

More content here.
```

**Scores:**

- BoldF1: 1.0
- ItalicF1: 1.0
- HeadingF1: 1.0
- StyleAccuracy: **1.0**

**Generated Markdown (Partial):**

```markdown
# Main Heading

This is bold and italic text.

## Sub Heading

More content here.
```

**Scores:**

- BoldF1: 0.5 (missed "bold" marker)
- ItalicF1: 0.5 (missed "italic" marker)
- HeadingF1: 1.0 (headings correct)
- StyleAccuracy: **0.67**

### Robustness Example

**Test corpus:** 30 PDFs (including 5 edge cases: corrupted, multilingual, scanned, etc.)

**Results:**

- Crash-free rate: 29/30 = 96.7% (1 crash on scanned PDF)
- Markdown validity: 30/30 = 100% (all outputs valid)
- Completeness: 28/30 = 93.3% (2 near-empty outputs from obfuscated fonts)

**Robustness Score: (96.7 + 100 + 93.3) / 3 = 96.7%**

### Performance Example

**Baseline (previous release):**

- Median: 250ms per 1-page PDF
- P95: 800ms per 1-page PDF

**Current run:**

- Median: 280ms per 1-page PDF
- P95: 750ms per 1-page PDF

**Scores:**

- Median: min(1.0, 250/280) = 0.89
- P95: min(1.0, 800/750) = 1.0
- Performance: 0.5 × 0.89 + 0.5 × 1.0 = **0.95**

## Workflow: Running a Full Validation

### Step 1: Prepare Test Data

```bash
# Navigate to PDF crate
cd edgequake/crates/edgequake-pdf

# Ensure ground-truth annotations exist
# Files should be named: <pdf_name>.gold.md
ls -1 test-data/real_dataset/*.gold.md
```

### Step 2: Generate Markdown Output

```bash
# Convert all PDFs to Markdown
cargo run -p edgequake-pdf --example real_dataset_eval -- --write

# Outputs written to: test-data/real_dataset/*.md
```

### Step 3: Run Validation

```bash
# Compute all metrics
python3 ../../.github/skills/pdf-markdown-validator/scripts/validate.py \
  --pdf-dir test-data/real_dataset \
  --gold-dir test-data/real_dataset \
  --output-report validation_report.json \
  --verbose
```

### Step 4: Analyze Results

```bash
# View summary
cat validation_report.json | jq '.summary'

# Detailed per-document breakdown
cat validation_report.json | jq '.documents | .[] | {name, scores}'

# Identify failure patterns
python3 ../../.github/skills/pdf-markdown-validator/scripts/analyze_failures.py \
  validation_report.json --group-by failure_type
```

### Step 5: Iterative Improvement

1. **Identify low-scoring dimensions** (table, style, etc.)
2. **Focus on highest-impact directory** (processor, renderer, layout)
3. **Implement targeted fix** with tests
4. **Re-run validation** to measure improvement
5. **Record baseline** for next comparison

## Ground-Truth Annotation Format

The `.gold.md` files serve as reference implementations. Use this structure:

```markdown
# Document Title (H1)

## Section Heading (H2)

This paragraph contains **bold text** and _italic text_ and **_bold-italic text_**.

### Subsection (H3)

#### Sub-subsection (H4)

**Note:** Use standard Markdown syntax. Be precise with:

- Bold: **text**
- Italic: _text_
- Bold-Italic: **_text_**
- Headings: # through #### for H1–H4

### Tables

| Column 1 | Column 2 | Column 3 |
| -------- | -------- | -------- |
| Cell 1   | Cell 2   | Cell 3   |
| Cell 4   | Cell 5   | Cell 6   |

Ensure:

- Pipes align properly
- Headers separated by `---|---` row
- No trailing spaces (can affect parsing)

### Code Blocks

\`\`\`python
def hello():
print("world")
\`\`\`

Use triple backticks with language identifier.

### Lists

Bullet list:

- Item 1
- Item 2
  - Nested item
- Item 3

Numbered list:

1. First
2. Second
3. Third

### Edge Cases

- **Multi-line table cells**: Not standard Markdown; flatten to single line
- **Merged cells**: Not representable in Markdown tables; split into separate rows
- **Vertical headers**: Use first row convention (all cells with **bold**)
```

## CLI Commands & Scripts

### Validation Runner

```bash
# Full validation pipeline
python3 scripts/validate.py \
  --pdf-dir <path/to/pdfs> \
  --gold-dir <path/to/gold> \
  [--output-report <report.json>] \
  [--metrics table,style,robustness,performance] \
  [--ci-mode] \
  [--fail-below 75]
```

**Options:**

- `--pdf-dir`: Directory containing PDFs and generated `.md` files
- `--gold-dir`: Directory containing `.gold.md` reference files
- `--output-report`: JSON file for machine-readable results (default: validation_report.json)
- `--metrics`: Comma-separated metrics to compute (default: all)
- `--ci-mode`: Fail with non-zero exit code if score below threshold
- `--fail-below`: Minimum acceptable score (default: 75)

### Failure Analysis

```bash
# Identify and categorize failures
python3 scripts/analyze_failures.py \
  <report.json> \
  [--group-by failure_type|document|metric] \
  [--export <output.csv>]
```

### Comparison Tool

```bash
# Compare two validation runs
python3 scripts/compare_runs.py \
  <baseline_report.json> \
  <current_report.json> \
  [--show-improvements] \
  [--show-regressions]
```

## Integration with CI/CD

### GitHub Actions Example

```yaml
name: PDF Validation

on: [push, pull_request]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install dependencies
        run: |
          sudo apt-get install -y pandoc
          pip install -r .github/skills/pdf-markdown-validator/requirements.txt

      - name: Generate Markdown
        run: |
          cd edgequake/crates/edgequake-pdf
          cargo run --example real_dataset_eval -- --write

      - name: Validate conversion
        run: |
          python3 .github/skills/pdf-markdown-validator/scripts/validate.py \
            --pdf-dir edgequake/crates/edgequake-pdf/test-data/real_dataset \
            --gold-dir edgequake/crates/edgequake-pdf/test-data/real_dataset \
            --ci-mode \
            --fail-below 75

      - name: Upload report
        if: always()
        uses: actions/upload-artifact@v3
        with:
          name: validation_report
          path: validation_report.json
```

## Extending the Validator

### Adding Custom Metrics

```python
# In your evaluation extension
from pdf_validator import BaseMetric

class CustomMetric(BaseMetric):
    def compute(self, gold_md: str, generated_md: str) -> float:
        """Implement your metric logic."""
        # Return float in [0, 1]
        pass

# Register with validator
validator.register_metric("custom_metric", CustomMetric(), weight=0.1)
```

### Implementing Custom Scoring Functions

```python
# Use different aggregation strategy
def weighted_macro_f1(f1_scores: dict) -> float:
    weights = {"bold": 0.4, "italic": 0.3, "heading": 0.3}
    return sum(f1_scores[k] * weights[k] for k in f1_scores)

validator.set_style_aggregator(weighted_macro_f1)
```

## Troubleshooting

### Issue: "Markdown validation failed"

**Symptom:** Many documents fail pandoc validation.

**Diagnosis:**

```bash
# Run pandoc on output directly
pandoc -f markdown -t html -o /dev/null generated.md
```

**Common causes:**

- Unmatched backticks in code blocks
- Invalid table syntax (missing pipes, misaligned columns)
- Improper escaping of special characters

**Solution:** Check the `.md` file for syntax errors and regenerate if needed.

### Issue: "Cell content accuracy very low"

**Symptom:** TableAccuracy < 50% despite detecting tables.

**Diagnosis:** Table cells extracted but content is corrupted or misaligned.

**Solution:**

1. Check cell boundary detection in `edgequake-pdf/src/processors/`
2. Verify token normalization (whitespace, special chars)
3. Compare gold vs. generated side-by-side in a diff tool

### Issue: "Performance regressed significantly"

**Symptom:** Performance score < 60%.

**Diagnosis:** Processing time increased 2-3x over baseline.

**Steps:**

1. Profile with `cargo flamegraph -p edgequake-pdf`
2. Check for newly added allocations or loops
3. Review recent changes to layout or renderer logic

### Issue: "Robustness failures on specific PDFs"

**Symptom:** Crashes only on PDFs in `edge_cases/` subdirectory.

**Diagnosis:** Parser/processor doesn't handle malformed or unusual PDFs.

**Solution:**

1. Isolate failing PDF
2. Add regression test for that case
3. Fix parser to handle edge case gracefully (or document limitation)

## Best Practices

### For Agents Using This Skill

1. **Run validation after every significant change**: Don't let regressions accumulate
2. **Use version-pinned baseline**: Document which version is the baseline for comparisons
3. **Combine metrics holistically**: Don't optimize one dimension at expense of others
4. **Test edge cases explicitly**: Include corrupted, multilingual, scanned PDFs in test set
5. **Monitor trend, not just score**: Small regressions compound over time

### For Refining Annotations

1. **Be consistent**: Use identical formatting across all `.gold.md` files
2. **Document special cases**: Note why a PDF is difficult (e.g., "scanned, requires OCR")
3. **Update baselines carefully**: When improving gold annotations, remeasure all documents
4. **Version control artifacts**: Keep both `.pdf` and `.gold.md` in git

### For CI/CD Integration

1. **Set realistic thresholds**: Don't enforce 100%; account for inherent PDF variation
2. **Alert on regressions, not absolutes**: Flag when score drops, even if still passing
3. **Separate blocking vs. warning gates**: Report comprehensive metrics but gate on critical ones
4. **Archive reports**: Keep historical results for trend analysis

## References & Further Reading

- **EdgeQuake PDF Crate**: `edgequake/crates/edgequake-pdf/README.md`
- **Improvement Guide**: `specs/27-improve-pdf.md` (OODA loop methodology)
- **Evaluation Harness**: `edgequake/crates/edgequake-pdf/examples/real_dataset_eval.rs`
- **Test Data**: `edgequake/crates/edgequake-pdf/test-data/real_dataset/`

## Summary

This skill enables systematic, quantified validation of PDF → Markdown conversions. Use it to:

- **Measure quality** with standardized metrics
- **Track improvements** objectively
- **Automate gates** in CI/CD
- **Compare approaches** fairly
- **Guide optimization** efforts

Start with the **Quick Start** section, prepare ground-truth annotations, and run the validation pipeline. Iterate using the **Workflow** guide, focusing on lowest-scoring dimensions first.
