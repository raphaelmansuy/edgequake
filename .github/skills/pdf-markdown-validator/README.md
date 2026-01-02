# PDF → Markdown Validator Skill

Configuration and setup guide for the PDF to Markdown validation skill.

## Directory Structure

```
pdf-markdown-validator/
├── SKILL.md                 # Main skill documentation
├── scripts/
│   ├── validate.py         # Main validation orchestrator
│   ├── analyze_failures.py # Failure categorization and analysis
│   ├── compare_runs.py     # Comparison of validation runs
│   └── requirements.txt    # Python dependencies
├── examples/
│   └── example.gold.md     # Example gold (reference) markdown
└── README.md               # This file
```

## Dependencies

### System Requirements

- **Rust**: 1.70+ (for building edgequake-pdf)
- **Python**: 3.9+ (for validation scripts)
- **pandoc**: 2.14+ (for Markdown validation)
- **cargo**: Latest stable

### Python Packages

The scripts have minimal dependencies (standard library only) for robustness. If you need data visualization or advanced analysis, create a virtual environment:

```bash
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
```

## Quick Start

### 1. Prepare Test PDFs and Ground Truth

Place your test PDFs in a directory with corresponding `.gold.md` files:

```
test-data/
├── document1.pdf
├── document1.gold.md
├── document2.pdf
├── document2.gold.md
└── ...
```

**Creating `.gold.md` files:**

The `.gold.md` file is your reference for what the PDF should convert to. Use standard Markdown with proper formatting:

```markdown
# Document Title

**Bold text** and _italic text_ demonstrate formatting.

| Table Header 1 | Table Header 2 |
| -------------- | -------------- |
| Cell 1         | Cell 2         |

See `examples/example.gold.md` for a complete template.
```

### 2. Generate Markdown from PDFs

```bash
cd edgequake/crates/edgequake-pdf
cargo run --example real_dataset_eval -- --write

# Output: test-data/real_dataset/*.md
```

### 3. Run Validation

```bash
python3 ../../.github/skills/pdf-markdown-validator/scripts/validate.py \
  --pdf-dir test-data/real_dataset \
  --gold-dir test-data/real_dataset \
  --output-report validation_report.json \
  --verbose
```

### 4. Analyze Results

```bash
# View summary
cat validation_report.json | jq '.summary'

# Analyze failures
python3 ../../.github/skills/pdf-markdown-validator/scripts/analyze_failures.py \
  validation_report.json \
  --group-by severity \
  --verbose
```

## Metric Explanation

### Table Accuracy (40%)

Combines two components:

- **Table Detection F1**: Can the system find tables in the PDF? (F1 score of detected vs. gold tables)
- **Cell Content Accuracy**: Are cell contents extracted correctly? (Token-level matching)

**Interpretation:**

- 90–100: Excellent table handling
- 70–89: Good; minor issues
- 50–69: Fair; significant cell content loss
- <50: Poor; systematic table failures

### Style Accuracy (40%)

Measures preservation of text formatting:

- **Bold F1**: Detection of **bold** text
- **Italic F1**: Detection of _italic_ text
- **Heading F1**: Correct heading levels (H1≠H2)

**Interpretation:**

- 90–100: Formatting consistently preserved
- 70–89: Good; minor style misses
- 50–69: Fair; some styles lost
- <50: Poor; formatting largely lost

### Robustness (10%)

System stability and validity:

- **Crash-free rate**: Percentage of PDFs processed without errors
- **Markdown validity**: Percentage passing `pandoc` syntax check
- **Completeness**: Percentage producing non-empty output

**Interpretation:**

- 95–100: Production-ready
- 85–94: Stable with minor issues
- 70–84: Moderate stability
- <70: Fragile; edge case failures

### Performance (10%)

Processing speed relative to baseline:

- **Median latency**: P50 processing time
- **P95 latency**: 95th percentile (tail behavior)

**Interpretation:**

- 95–100: Meets/exceeds baseline
- 80–94: Good; ≤20% slowdown
- 60–79: Acceptable; 20–60% slowdown
- <60: Significant regression

## Common Workflows

### Workflow 1: First-Time Setup

```bash
# 1. Create test directory with PDFs
mkdir -p test-data/my_pdfs
cp *.pdf test-data/my_pdfs/

# 2. Create reference markdown for each PDF
# Either manually or by annotating the current output:
cp test-data/my_pdfs/doc1.md test-data/my_pdfs/doc1.gold.md
# Then edit doc1.gold.md to fix any errors

# 3. Generate markdown from PDFs (if not already done)
cargo run --example real_dataset_eval -- --write

# 4. Run initial validation
python3 scripts/validate.py \
  --pdf-dir test-data/my_pdfs \
  --gold-dir test-data/my_pdfs \
  --verbose

# 5. Review report
cat validation_report.json | jq '.summary'
```

### Workflow 2: Iterative Improvement

```bash
# Baseline run
python3 scripts/validate.py \
  --pdf-dir test-data \
  --gold-dir test-data \
  --output-report baseline.json

# Make code changes to edgequake-pdf...

# Regenerate markdown
cd edgequake/crates/edgequake-pdf
cargo run --example real_dataset_eval -- --write
cd -

# New validation run
python3 scripts/validate.py \
  --pdf-dir test-data \
  --gold-dir test-data \
  --output-report current.json

# Compare results
python3 scripts/compare_runs.py baseline.json current.json \
  --show-improvements \
  --show-regressions
```

### Workflow 3: CI/CD Integration

In your GitHub Actions workflow:

```yaml
- name: Validate PDF → Markdown
  run: |
    python3 .github/skills/pdf-markdown-validator/scripts/validate.py \
      --pdf-dir edgequake/crates/edgequake-pdf/test-data \
      --gold-dir edgequake/crates/edgequake-pdf/test-data \
      --ci-mode \
      --fail-below 75
```

### Workflow 4: Identifying Failure Patterns

```bash
# Run validation
python3 scripts/validate.py \
  --pdf-dir test-data \
  --gold-dir test-data \
  --output-report report.json

# Analyze by severity
python3 scripts/analyze_failures.py report.json \
  --group-by severity \
  --export failures.csv

# Review critical failures
cat failures.csv | awk -F',' '$5 < 30 {print $1}'
```

### Workflow 5: Detailed Drift Analysis (NEW)

Identify EXACTLY WHERE and HOW conversions differ:

```bash
# Single file diff analysis
python3 scripts/diff_analysis.py \
  test-data/document1.gold.md \
  test-data/document1.md \
  --show-full-diff \
  --verbose

# Batch analysis across all files
python3 scripts/batch_drift.py \
  --pdf-dir test-data/real_dataset \
  --gold-dir test-data/real_dataset \
  --output-report drift_report.json \
  --verbose

# View drift categories
cat drift_report.json | jq '.drifts_by_category'

# Identify top issues
cat drift_report.json | jq '.top_issues'
```

The drift analysis shows:

- **Line-by-line differences** with categorization (style, content, heading, table, list, code)
- **Severity assessment** (🔴 critical, 🟠 major, 🟡 minor)
- **Grouped statistics** across all files for trend analysis
- **JSON export** for CI/CD integration and dashboards

## Ground-Truth Best Practices

1. **Use actual reference implementations**: If the PDF has a "correct" conversion elsewhere, use that as gold
2. **Be consistent**: Use identical Markdown style across all `.gold.md` files
3. **Document edge cases**: Add comments about why a PDF is difficult
4. **Update baselines carefully**: When improving gold annotations, re-validate all documents
5. **Keep in version control**: Commit both `.pdf` and `.gold.md` files together

Example `.gold.md` header with documentation:

```markdown
---
source: Original publication
difficulty: medium # easy, medium, hard, edge-case
notes: Contains 3-column layout with merged cells in table
---

# Document Title

...
```

## Troubleshooting

### Error: "Markdown validation failed"

**Check:**

```bash
pandoc -f markdown -t html -o /dev/null generated.md 2>&1
```

**Common issues:**

- Unmatched backticks in code
- Missing pipes in table
- Improperly escaped special characters

### Error: "Cell content accuracy very low"

**Debug:** Visually compare gold vs. generated:

```bash
diff -y test-data/doc.gold.md test-data/doc.md | less
```

### Issue: Performance regressed

**Profile the code:**

```bash
cd edgequake/crates/edgequake-pdf
cargo flamegraph --example real_dataset_eval
```

## Extending the Validator

### Add a Custom Metric

Extend `scripts/validate.py`:

```python
def _compute_custom_metric(self, generated: str, gold: str) -> float:
    """Your metric logic here."""
    return 0.9  # Return float in [0, 1]
```

### Use Different Weighting

Modify aggregation in `validate.py`:

```python
# Change weights (must sum to 1.0)
composite = (
    0.35 * table_f1 +      # Reduced from 0.4
    0.45 * style_f1 +      # Increased from 0.4
    0.10 * robustness +
    0.10 * performance_score
) * 100
```

### Integrate with Your Tool

```python
from pathlib import Path
import json

# Load validation results
with open("validation_report.json") as f:
    report = json.load(f)

# Access summary scores
score = report["summary"]["composite"]
if score >= 80:
    print("✓ Validation passed")
```

## References

- **SKILL Documentation**: See [SKILL.md](SKILL.md) for comprehensive feature guide
- **EdgeQuake PDF Crate**: `edgequake/crates/edgequake-pdf/README.md`
- **Test Data**: `edgequake/crates/edgequake-pdf/test-data/`
- **Improvement Process**: `specs/27-improve-pdf.md` (OODA methodology)

## Support

For issues or feature requests related to this skill, refer to:

1. The comprehensive [SKILL.md](SKILL.md) documentation
2. Example workflows in this README
3. The test data and examples in the `examples/` directory
4. The EdgeQuake PDF crate documentation
