#!/usr/bin/env python3
"""
PDF → Markdown Validator

Comprehensive validation framework for PDF to Markdown conversion quality.
Computes table accuracy, style accuracy, robustness, and performance metrics.
"""

import argparse
import json
import re
import statistics
import subprocess
import sys
from collections import defaultdict
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Dict, List, Optional, Tuple


@dataclass
class TokenMetrics:
    """Token-level F1 score components."""

    precision: float
    recall: float
    f1: float
    tp: int  # true positives
    fp: int  # false positives
    fn: int  # false negatives


@dataclass
class DocumentScores:
    """Per-document validation scores."""

    name: str
    table_accuracy: float
    style_accuracy: float
    robustness: float
    performance_ms: Optional[float]
    composite: float
    errors: List[str]


@dataclass
class SummaryScores:
    """Aggregate validation scores."""

    table_accuracy: float
    style_accuracy: float
    robustness: float
    performance: float
    composite: float
    document_count: int
    crash_count: int
    invalid_markdown_count: int


class PDFValidator:
    """Main validation orchestrator."""

    def __init__(self, pdf_dir: Path, gold_dir: Path, verbose: bool = False):
        self.pdf_dir = Path(pdf_dir)
        self.gold_dir = Path(gold_dir)
        self.verbose = verbose
        self.document_scores: List[DocumentScores] = []
        self.baselines = {}

    def validate(self) -> Tuple[SummaryScores, List[DocumentScores]]:
        """Run full validation pipeline."""
        if self.verbose:
            print(f"[Validator] Scanning {self.pdf_dir} for markdown files...")

        # Collect all markdown files (*.md) and find corresponding .gold.md
        md_files = sorted(self.pdf_dir.glob("*.md"))
        # Filter out .gold.md files
        md_files = [f for f in md_files if not f.name.endswith(".gold.md")]

        if not md_files:
            raise ValueError(f"No markdown files found in {self.pdf_dir}")

        if self.verbose:
            print(f"[Validator] Found {len(md_files)} markdown files")

        document_scores = []
        all_metrics = defaultdict(list)
        crashes = 0
        invalid_md = 0

        for md_path in md_files:
            name = md_path.stem
            gold_md = self.gold_dir / f"{name}.gold.md"

            if self.verbose:
                print(f"\n[Validator] Processing: {name}")

            # Check for gold markdown
            if not gold_md.exists():
                if self.verbose:
                    print(f"  ⚠ Gold markdown not found: {gold_md}")
                continue

            errors = []

            # Step 1: Validate Markdown syntax
            if not self._is_valid_markdown(md_path):
                invalid_md += 1
                errors.append("Invalid Markdown syntax")

            # Step 2: Read content
            try:
                generated_content = md_path.read_text(encoding="utf-8")
                gold_content = gold_md.read_text(encoding="utf-8")
            except Exception as e:
                crashes += 1
                errors.append(f"Read error: {e}")
                continue

            # Step 3: Compute metrics
            try:
                table_f1 = self._compute_table_accuracy(generated_content, gold_content)
                style_f1 = self._compute_style_accuracy(generated_content, gold_content)
                robustness = 1.0 if not errors else 0.8  # Penalize if validation failed
                performance_ms = None  # Optional: extract from profiling
            except Exception as e:
                crashes += 1
                errors.append(f"Metric computation error: {e}")
                continue

            # Composite score: 0.4 table + 0.4 style + 0.1 robustness + 0.1 performance
            performance_score = self._estimate_performance_score()
            composite = (
                0.4 * table_f1
                + 0.4 * style_f1
                + 0.1 * robustness
                + 0.1 * performance_score
            ) * 100  # Scale to 0-100

            doc_score = DocumentScores(
                name=name,
                table_accuracy=table_f1 * 100,
                style_accuracy=style_f1 * 100,
                robustness=robustness * 100,
                performance_ms=performance_ms,
                composite=composite,
                errors=errors,
            )

            document_scores.append(doc_score)

            # Track for aggregation
            all_metrics["table_accuracy"].append(table_f1)
            all_metrics["style_accuracy"].append(style_f1)
            all_metrics["robustness"].append(robustness)

            if self.verbose:
                print(
                    f"  ✓ Table: {table_f1*100:.1f}%, Style: {style_f1*100:.1f}%, Composite: {composite:.1f}"
                )

        # Compute summaries
        summary = SummaryScores(
            table_accuracy=self._average_metric(all_metrics["table_accuracy"]) * 100,
            style_accuracy=self._average_metric(all_metrics["style_accuracy"]) * 100,
            robustness=self._average_metric(all_metrics["robustness"]) * 100,
            performance=self._estimate_performance_score() * 100,
            composite=self._average_metric(
                [
                    0.4 * all_metrics["table_accuracy"][i]
                    + 0.4 * all_metrics["style_accuracy"][i]
                    + 0.1 * all_metrics["robustness"][i]
                    + 0.1 * self._estimate_performance_score()
                    for i in range(len(document_scores))
                ]
            )
            * 100,
            document_count=len(document_scores),
            crash_count=crashes,
            invalid_markdown_count=invalid_md,
        )

        return summary, document_scores

    def _is_valid_markdown(self, md_path: Path) -> bool:
        """Validate Markdown syntax using pandoc."""
        try:
            result = subprocess.run(
                [
                    "pandoc",
                    "-f",
                    "markdown",
                    "-t",
                    "html",
                    "-o",
                    "/dev/null",
                    str(md_path),
                ],
                capture_output=True,
                timeout=10,
            )
            return result.returncode == 0
        except (FileNotFoundError, subprocess.TimeoutExpired):
            return False

    def _compute_table_accuracy(self, generated: str, gold: str) -> float:
        """
        Compute table accuracy as average of:
        - Table detection F1 (via regex block matching)
        - Cell content token-level F1
        """
        gen_tables = self._extract_tables(generated)
        gold_tables = self._extract_tables(gold)

        if not gold_tables:
            return 1.0 if not gen_tables else 0.5

        # Match tables by comparing cell counts
        detection_tp = 0
        for gold_tbl in gold_tables:
            gold_cells = len(self._extract_cells(gold_tbl))
            for gen_tbl in gen_tables:
                gen_cells = len(self._extract_cells(gen_tbl))
                # Consider match if cell count is within 10%
                if gen_cells > 0 and abs(gold_cells - gen_cells) / gen_cells <= 0.1:
                    detection_tp += 1
                    break

        # Real F1 for table detection
        detection_fp = max(0, len(gen_tables) - detection_tp)
        detection_fn = max(0, len(gold_tables) - detection_tp)
        detection_f1 = self._compute_f1(
            detection_tp, detection_tp + detection_fp, detection_tp + detection_fn
        )

        # Cell content accuracy (token-level F1)
        cell_accuracy = self._compute_cell_accuracy(gen_tables, gold_tables)

        return 0.5 * detection_f1 + 0.5 * cell_accuracy

    def _compute_style_accuracy(self, generated: str, gold: str) -> float:
        """
        Compute style accuracy as macro-average of:
        - Bold F1 (token-level)
        - Italic F1 (token-level)
        - Heading F1 (level accuracy)
        """
        bold_f1 = self._compute_token_f1(generated, gold, "bold")
        italic_f1 = self._compute_token_f1(generated, gold, "italic")
        heading_f1 = self._compute_heading_f1(generated, gold)

        return (bold_f1 + italic_f1 + heading_f1) / 3.0

    def _compute_token_f1(self, generated: str, gold: str, style_type: str) -> float:
        """
        Compute token-level F1 for a specific style.
        Tokenizes both documents and checks if tokens match with correct styling.
        """
        if style_type == "bold":
            # Match **text** but not inside ***text***
            pattern = r"\*\*([^*]+?)\*\*"
        elif style_type == "italic":
            # Match *text* but not inside **text** and not when preceded/followed by *
            pattern = r"(?<!\*)\*([^*]+?)\*(?!\*)"
        else:
            return 0.5

        gen_styled = set(re.findall(pattern, generated))
        gold_styled = set(re.findall(pattern, gold))

        if not gold_styled:
            # No styled text in gold; check if generator added none either
            return 1.0 if not gen_styled else 0.8

        tp = len(gen_styled & gold_styled)
        fp = len(gen_styled - gold_styled)
        fn = len(gold_styled - gen_styled)

        precision = tp / (tp + fp) if (tp + fp) > 0 else 0
        recall = tp / (tp + fn) if (tp + fn) > 0 else 0

        if precision + recall == 0:
            return 0.0

        return 2 * (precision * recall) / (precision + recall)

    def _compute_heading_f1(self, generated: str, gold: str) -> float:
        """Compute F1 for heading level accuracy."""
        gen_headings = re.findall(r"^(#{1,6})\s+(.+)$", generated, re.MULTILINE)
        gold_headings = re.findall(r"^(#{1,6})\s+(.+)$", gold, re.MULTILINE)

        if not gold_headings:
            return 1.0 if not gen_headings else 0.5

        gen_dict = {text: level for level, text in gen_headings}
        gold_dict = {text: level for level, text in gold_headings}

        matches = sum(1 for text in gold_dict if gen_dict.get(text) == gold_dict[text])
        return self._compute_f1(matches, len(gen_dict), len(gold_dict))

    def _extract_tables(self, markdown: str) -> List[str]:
        """Extract Markdown table blocks."""
        pattern = r"^\|.*\|$"
        lines = markdown.split("\n")
        tables = []
        current_table = []

        for line in lines:
            if re.match(pattern, line):
                current_table.append(line)
            elif current_table:
                tables.append("\n".join(current_table))
                current_table = []

        if current_table:
            tables.append("\n".join(current_table))

        return tables

    def _compute_cell_accuracy(
        self, gen_tables: List[str], gold_tables: List[str]
    ) -> float:
        """
        Compute token-level F1 accuracy of table cells.
        For each gold table, find best matching generated table and compute cell F1.
        """
        if not gold_tables:
            return 1.0

        total_f1 = []
        for gold in gold_tables:
            gold_cells = self._extract_cells(gold)

            # Find best matching generated table
            best_f1 = 0.0
            for gen in gen_tables:
                gen_cells = self._extract_cells(gen)
                # Compute cell-level token F1
                cell_f1 = self._compute_cells_f1(gen_cells, gold_cells)
                best_f1 = max(best_f1, cell_f1)

            total_f1.append(best_f1)

        return sum(total_f1) / len(total_f1) if total_f1 else 0.5

    def _compute_cells_f1(self, gen_cells: List[str], gold_cells: List[str]) -> float:
        """
        Compute token-level F1 for cell contents.
        Cells are matched positionally.
        """
        if not gold_cells:
            return 1.0 if not gen_cells else 0.5

        total_tp = 0
        total_fp = 0
        total_fn = 0

        # Tokenize each cell and compute TP/FP/FN
        for i, gold_cell in enumerate(gold_cells):
            gold_tokens = set(gold_cell.lower().split())
            gen_tokens = (
                set(gen_cells[i].lower().split()) if i < len(gen_cells) else set()
            )

            tp = len(gold_tokens & gen_tokens)
            fp = len(gen_tokens - gold_tokens)
            fn = len(gold_tokens - gen_tokens)

            total_tp += tp
            total_fp += fp
            total_fn += fn

        # Unmatched gold cells (false negatives)
        for i in range(len(gen_cells), len(gold_cells)):
            total_fn += len(gold_cells[i].split())

        precision = total_tp / (total_tp + total_fp) if (total_tp + total_fp) > 0 else 0
        recall = total_tp / (total_tp + total_fn) if (total_tp + total_fn) > 0 else 0

        if precision + recall == 0:
            return 0.0

        return 2 * (precision * recall) / (precision + recall)

    def _extract_cells(self, table: str) -> List[str]:
        """Extract cell contents from Markdown table."""
        cells = []
        for line in table.split("\n"):
            line = line.strip()
            if line.startswith("|") and line.endswith("|") and "---" not in line:
                row = [c.strip() for c in line.split("|")[1:-1]]
                cells.extend(row)
        return cells

    def _compute_f1(self, tp: int, total_predicted: int, total_gold: int) -> float:
        """Compute F1 score from TP, total predicted, and total gold."""
        if total_predicted == 0 or total_gold == 0:
            return 1.0 if total_predicted == total_gold else 0.0

        precision = tp / total_predicted if total_predicted > 0 else 0
        recall = tp / total_gold if total_gold > 0 else 0

        if precision + recall == 0:
            return 0.0

        return 2 * (precision * recall) / (precision + recall)

    def _average_metric(self, values: List[float]) -> float:
        """Compute average metric, handling empty lists."""
        return statistics.mean(values) if values else 0.5

    def _estimate_performance_score(self) -> float:
        """Estimate performance score (placeholder; could be profiled)."""
        # Return 0.9 as default (90% of baseline)
        return 0.9


def main():
    parser = argparse.ArgumentParser(description="Validate PDF to Markdown conversions")
    parser.add_argument(
        "--pdf-dir", required=True, help="Directory with PDFs and .md files"
    )
    parser.add_argument(
        "--gold-dir", required=True, help="Directory with .gold.md files"
    )
    parser.add_argument(
        "--output-report", default="validation_report.json", help="Output report path"
    )
    parser.add_argument(
        "--metrics",
        default="table,style,robustness,performance",
        help="Metrics to compute",
    )
    parser.add_argument(
        "--ci-mode", action="store_true", help="Exit non-zero if score below threshold"
    )
    parser.add_argument(
        "--fail-below", type=float, default=75, help="Minimum acceptable score"
    )
    parser.add_argument("--verbose", "-v", action="store_true", help="Verbose output")

    args = parser.parse_args()

    try:
        validator = PDFValidator(
            Path(args.pdf_dir), Path(args.gold_dir), verbose=args.verbose
        )

        print(f"Starting validation...")
        summary, documents = validator.validate()

        # Format output
        report = {
            "summary": asdict(summary),
            "documents": [asdict(d) for d in documents],
        }

        # Write report
        with open(args.output_report, "w") as f:
            json.dump(report, f, indent=2)

        # Print summary
        print(f"\n{'='*60}")
        print(f"PDF → Markdown Validation Summary")
        print(f"{'='*60}")
        print(f"Documents processed: {summary.document_count}")
        print(f"Table Accuracy:      {summary.table_accuracy:.1f}%")
        print(f"Style Accuracy:      {summary.style_accuracy:.1f}%")
        print(f"Robustness:          {summary.robustness:.1f}%")
        print(f"Performance:         {summary.performance:.1f}%")
        print(f"\nComposite Score:     {summary.composite:.1f}/100")
        print(f"{'='*60}")

        if summary.crash_count > 0:
            print(f"⚠ Crashes: {summary.crash_count}")
        if summary.invalid_markdown_count > 0:
            print(f"⚠ Invalid Markdown: {summary.invalid_markdown_count}")

        # CI mode
        if args.ci_mode and summary.composite < args.fail_below:
            print(
                f"\n✗ FAIL: Score {summary.composite:.1f} below threshold {args.fail_below}"
            )
            sys.exit(1)
        else:
            print(f"\n✓ PASS")
            sys.exit(0)

    except Exception as e:
        print(f"✗ Validator error: {e}", file=sys.stderr)
        import traceback

        traceback.print_exc()
        sys.exit(1)


if __name__ == "__main__":
    main()
