#!/usr/bin/env python3
"""
PDF → Markdown Validation Failure Analyzer

Categorize and analyze validation failures by type, severity, and pattern.
Helpful for identifying systematic issues and prioritizing fixes.
"""

import argparse
import csv
import json
import sys
from collections import defaultdict
from pathlib import Path
from typing import Dict, List


class FailureAnalyzer:
    """Analyze validation failures."""

    def __init__(self, report_path: Path):
        with open(report_path) as f:
            self.report = json.load(f)

    def analyze(self) -> Dict:
        """Run failure analysis."""
        failures_by_type = self._categorize_failures()
        failures_by_severity = self._categorize_by_severity()
        patterns = self._identify_patterns()

        return {
            "by_type": failures_by_type,
            "by_severity": failures_by_severity,
            "patterns": patterns,
            "total_failures": sum(len(f) for f in failures_by_type.values()),
            "critical_count": len(failures_by_severity.get("critical", [])),
        }

    def _categorize_failures(self) -> Dict[str, List]:
        """Categorize failures by type."""
        categories = defaultdict(list)

        for doc in self.report["documents"]:
            for error in doc.get("errors", []):
                if "Invalid Markdown" in error:
                    categories["invalid_markdown"].append(doc["name"])
                elif "Read error" in error:
                    categories["read_error"].append(doc["name"])
                elif "Metric" in error:
                    categories["metric_computation"].append(doc["name"])
                else:
                    categories["other"].append(doc["name"])

        return dict(categories)

    def _categorize_by_severity(self) -> Dict[str, List]:
        """Categorize by impact severity."""
        severity = defaultdict(list)

        for doc in self.report["documents"]:
            score = doc["composite"]

            if score < 30:
                severity["critical"].append((doc["name"], score))
            elif score < 60:
                severity["high"].append((doc["name"], score))
            elif score < 80:
                severity["medium"].append((doc["name"], score))
            else:
                severity["low"].append((doc["name"], score))

        return dict(severity)

    def _identify_patterns(self) -> List[Dict]:
        """Identify systematic failure patterns."""
        patterns = []

        # Pattern 1: Consistent table failures
        table_scores = [d["table_accuracy"] for d in self.report["documents"]]
        if table_scores and min(table_scores) < 50:
            patterns.append(
                {
                    "type": "Low table accuracy",
                    "severity": "high",
                    "recommendation": "Review table detection and cell extraction logic",
                }
            )

        # Pattern 2: Style accuracy issues
        style_scores = [d["style_accuracy"] for d in self.report["documents"]]
        if style_scores and min(style_scores) < 50:
            patterns.append(
                {
                    "type": "Low style accuracy",
                    "severity": "high",
                    "recommendation": "Check bold/italic/heading detection patterns",
                }
            )

        # Pattern 3: Robustness issues
        robustness_issues = [
            d for d in self.report["documents"] if d["robustness"] < 100
        ]
        if len(robustness_issues) > len(self.report["documents"]) * 0.2:
            patterns.append(
                {
                    "type": "Robustness issues",
                    "severity": "medium",
                    "recommendation": "Add error handling for edge cases; test on corrupted PDFs",
                }
            )

        return patterns

    def export_csv(self, output_path: Path):
        """Export analysis to CSV."""
        with open(output_path, "w", newline="") as f:
            writer = csv.writer(f)
            writer.writerow(
                [
                    "Document",
                    "Table Accuracy",
                    "Style Accuracy",
                    "Robustness",
                    "Composite Score",
                    "Errors",
                ]
            )

            for doc in self.report["documents"]:
                writer.writerow(
                    [
                        doc["name"],
                        f"{doc['table_accuracy']:.1f}%",
                        f"{doc['style_accuracy']:.1f}%",
                        f"{doc['robustness']:.1f}%",
                        f"{doc['composite']:.1f}",
                        "; ".join(doc.get("errors", [])),
                    ]
                )


def main():
    parser = argparse.ArgumentParser(
        description="Analyze PDF → Markdown validation failures"
    )
    parser.add_argument("report", help="Path to validation_report.json")
    parser.add_argument(
        "--group-by",
        default="failure_type",
        choices=["failure_type", "severity", "pattern"],
        help="How to group failures",
    )
    parser.add_argument("--export", help="Export to CSV")
    parser.add_argument("--verbose", "-v", action="store_true")

    args = parser.parse_args()

    try:
        analyzer = FailureAnalyzer(Path(args.report))
        analysis = analyzer.analyze()

        print(f"\n{'='*60}")
        print("PDF → Markdown Validation Failure Analysis")
        print(f"{'='*60}\n")

        # Group by type
        print("Failures by Type:")
        for fail_type, docs in analysis["by_type"].items():
            print(f"  {fail_type}: {len(docs)} documents")
            if args.verbose:
                for doc in docs[:3]:
                    print(f"    - {doc}")

        # Group by severity
        print("\nFailures by Severity:")
        for severity, docs in analysis["by_severity"].items():
            count = len(docs)
            print(f"  {severity}: {count} documents")
            if args.verbose and docs:
                for doc_name, score in docs[:2]:
                    print(f"    - {doc_name} ({score:.1f})")

        # Patterns
        if analysis["patterns"]:
            print("\nIdentified Patterns:")
            for pattern in analysis["patterns"]:
                print(f"  • {pattern['type']} (severity: {pattern['severity']})")
                print(f"    → {pattern['recommendation']}")

        # Summary
        print(f"\nSummary:")
        print(f"  Total failures: {analysis['total_failures']}")
        print(f"  Critical issues: {analysis['critical_count']}")

        # Export if requested
        if args.export:
            analyzer.export_csv(Path(args.export))
            print(f"\nExported to: {args.export}")

    except Exception as e:
        print(f"✗ Error: {e}", file=sys.stderr)
        import traceback

        traceback.print_exc()
        sys.exit(1)


if __name__ == "__main__":
    main()
