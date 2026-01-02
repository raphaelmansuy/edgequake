#!/usr/bin/env python3
"""
Batch Drift Analysis for PDF → Markdown Validation

Analyze all drift/diff points across multiple gold/generated pairs.
Generate comprehensive drift report showing errors and divergences.
"""

import argparse
import json
import subprocess
import sys
from collections import defaultdict
from pathlib import Path
from typing import Dict, List


class BatchDriftAnalyzer:
    """Analyze drifts across multiple file pairs."""

    def __init__(self, pdf_dir: Path, gold_dir: Path, verbose: bool = False):
        self.pdf_dir = Path(pdf_dir)
        self.gold_dir = Path(gold_dir)
        self.verbose = verbose

    def analyze_all(self) -> Dict:
        """Analyze all gold/gen pairs."""
        md_files = sorted(self.pdf_dir.glob("*.md"))
        md_files = [f for f in md_files if not f.name.endswith(".gold.md")]

        all_drifts = []
        summary_stats = defaultdict(int)

        for md_path in md_files:
            name = md_path.stem
            gold_path = self.gold_dir / f"{name}.gold.md"

            if not gold_path.exists():
                continue

            if self.verbose:
                print(f"Analyzing: {name}", file=sys.stderr)

            # Run diff_analysis.py and parse JSON output
            result = self._run_diff_analysis(gold_path, md_path)
            if result:
                all_drifts.extend(result["drifts"])
                self._update_stats(summary_stats, result["summary"])

        return {
            "total_files": len(md_files),
            "analyzed_files": len(
                [f for f in md_files if (self.gold_dir / f"{f.stem}.gold.md").exists()]
            ),
            "total_drifts": len(all_drifts),
            "drifts_by_severity": self._group_by_severity(all_drifts),
            "drifts_by_category": self._group_by_category(all_drifts),
            "top_issues": self._identify_top_issues(all_drifts),
            "summary_stats": dict(summary_stats),
        }

    def _run_diff_analysis(self, gold_path: Path, gen_path: Path) -> Dict:
        """Run diff_analysis.py and return structured results."""
        try:
            cmd = [
                sys.executable,
                str(Path(__file__).parent / "diff_analysis.py"),
                str(gold_path),
                str(gen_path),
                "--output-json",
                "/tmp/drift_result.json",
            ]
            result = subprocess.run(cmd, capture_output=True, text=True, timeout=10)

            with open("/tmp/drift_result.json") as f:
                report = json.load(f)

            return {
                "file": report["file"],
                "drifts": report["drift_points"],
                "summary": report["summary"],
            }
        except Exception as e:
            if self.verbose:
                print(f"Error analyzing {gold_path.stem}: {e}", file=sys.stderr)
            return None

    def _update_stats(self, stats: dict, summary: dict):
        """Update summary statistics."""
        for key in [
            "missing_count",
            "extra_count",
            "mismatch_count",
            "critical_count",
            "major_count",
            "minor_count",
        ]:
            if key in summary:
                stats[key] += summary[key]

    def _group_by_severity(self, drifts: List[Dict]) -> Dict[str, List]:
        """Group drifts by severity."""
        groups = defaultdict(list)
        for drift in drifts:
            groups[drift["severity"]].append(drift)
        return dict(groups)

    def _group_by_category(self, drifts: List[Dict]) -> Dict[str, List]:
        """Group drifts by category."""
        groups = defaultdict(list)
        for drift in drifts:
            groups[drift["category"]].append(drift)
        return dict(groups)

    def _identify_top_issues(self, drifts: List[Dict], limit: int = 10) -> List[Dict]:
        """Identify most common issues."""
        issue_counts = defaultdict(int)
        issue_examples = {}

        for drift in drifts:
            key = f"{drift['category']}:{drift['type']}"
            issue_counts[key] += 1
            if key not in issue_examples:
                issue_examples[key] = drift

        # Sort by frequency
        sorted_issues = sorted(issue_counts.items(), key=lambda x: -x[1])[:limit]

        result = []
        for issue_key, count in sorted_issues:
            cat, dtype = issue_key.split(":")
            result.append(
                {
                    "issue": issue_key,
                    "count": count,
                    "category": cat,
                    "type": dtype,
                    "example": issue_examples[issue_key],
                }
            )

        return result


def main():
    parser = argparse.ArgumentParser(
        description="Batch drift analysis for PDF → Markdown validation"
    )
    parser.add_argument(
        "--pdf-dir", required=True, help="Directory with markdown files"
    )
    parser.add_argument("--gold-dir", required=True, help="Directory with gold files")
    parser.add_argument(
        "--output-report", default="drift_report.json", help="Output report path"
    )
    parser.add_argument("--verbose", "-v", action="store_true", help="Verbose output")

    args = parser.parse_args()

    try:
        analyzer = BatchDriftAnalyzer(
            Path(args.pdf_dir), Path(args.gold_dir), verbose=args.verbose
        )

        print("Analyzing drifts across all files...")
        results = analyzer.analyze_all()

        # Save report
        with open(args.output_report, "w") as f:
            json.dump(results, f, indent=2)

        # Print summary
        print(f"\n{'='*70}")
        print("Batch Drift Analysis Report")
        print(f"{'='*70}")
        print(f"Files analyzed: {results['analyzed_files']}/{results['total_files']}")
        print(f"Total drifts found: {results['total_drifts']}")

        if results["drifts_by_severity"]:
            print(f"\nBy Severity:")
            for severity in ["critical", "major", "minor"]:
                count = len(results["drifts_by_severity"].get(severity, []))
                icon = (
                    "🔴"
                    if severity == "critical"
                    else "🟠" if severity == "major" else "🟡"
                )
                print(f"  {icon} {severity.upper()}: {count}")

        if results["drifts_by_category"]:
            print(f"\nBy Category:")
            for cat in sorted(results["drifts_by_category"].keys()):
                count = len(results["drifts_by_category"][cat])
                print(f"  {cat}: {count}")

        if results["top_issues"]:
            print(f"\nTop Issues:")
            for i, issue in enumerate(results["top_issues"], 1):
                print(f"  {i}. {issue['issue']}: {issue['count']} occurrences")

        print(f"\n✓ Report saved to {args.output_report}")
        print(f"{'='*70}\n")

    except Exception as e:
        print(f"✗ Error: {e}", file=sys.stderr)
        import traceback

        traceback.print_exc()
        sys.exit(1)


if __name__ == "__main__":
    main()
