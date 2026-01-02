#!/usr/bin/env python3
"""
PDF → Markdown Validation Report Comparator

Compare two validation runs to identify improvements, regressions, and trends.
"""

import argparse
import json
import sys
from pathlib import Path
from typing import Dict, Tuple


class ReportComparator:
    """Compare two validation reports."""

    def __init__(self, baseline_path: Path, current_path: Path):
        with open(baseline_path) as f:
            self.baseline = json.load(f)
        with open(current_path) as f:
            self.current = json.load(f)

    def compare(self) -> Dict:
        """Compare baseline and current reports."""
        baseline_summary = self.baseline["summary"]
        current_summary = self.current["summary"]

        comparison = {
            "table_accuracy": self._compare_metric(
                baseline_summary["table_accuracy"], current_summary["table_accuracy"]
            ),
            "style_accuracy": self._compare_metric(
                baseline_summary["style_accuracy"], current_summary["style_accuracy"]
            ),
            "robustness": self._compare_metric(
                baseline_summary["robustness"], current_summary["robustness"]
            ),
            "performance": self._compare_metric(
                baseline_summary["performance"], current_summary["performance"]
            ),
            "composite": self._compare_metric(
                baseline_summary["composite"], current_summary["composite"]
            ),
            "document_improvements": self._compare_documents(),
            "document_regressions": self._compare_documents(regression=True),
        }

        return comparison

    def _compare_metric(self, baseline: float, current: float) -> Dict:
        """Compare a single metric."""
        delta = current - baseline
        percent_change = (delta / baseline * 100) if baseline != 0 else 0

        return {
            "baseline": baseline,
            "current": current,
            "delta": delta,
            "percent_change": percent_change,
            "status": (
                "✓ improved"
                if delta > 1
                else ("↔ same" if abs(delta) <= 1 else "✗ regressed")
            ),
        }

    def _compare_documents(self, regression: bool = False) -> Dict[str, float]:
        """Compare document-level scores."""
        baseline_docs = {d["name"]: d["composite"] for d in self.baseline["documents"]}
        current_docs = {d["name"]: d["composite"] for d in self.current["documents"]}

        changes = {}
        for name in baseline_docs:
            if name in current_docs:
                delta = current_docs[name] - baseline_docs[name]
                if (delta > 2 and not regression) or (delta < -2 and regression):
                    changes[name] = delta

        return changes

    def print_report(
        self, show_improvements: bool = False, show_regressions: bool = False
    ):
        """Print comparison report."""
        comparison = self.compare()

        print(f"\n{'='*70}")
        print("PDF → Markdown Validation Comparison Report")
        print(f"{'='*70}\n")

        print("Metric Changes:")
        print(
            f"{'Metric':<20} {'Baseline':<12} {'Current':<12} {'Change':<12} {'Status'}"
        )
        print("-" * 70)

        for metric in [
            "table_accuracy",
            "style_accuracy",
            "robustness",
            "performance",
            "composite",
        ]:
            data = comparison[metric]
            baseline = data["baseline"]
            current = data["current"]
            delta = data["delta"]
            status = data["status"]

            print(
                f"{metric:<20} {baseline:<12.1f} {current:<12.1f} {delta:+.1f} ({data['percent_change']:+.1f}%) {status}"
            )

        # Document improvements
        if show_improvements and comparison["document_improvements"]:
            print(f"\nDocument Improvements (>2 points):")
            for doc, delta in sorted(
                comparison["document_improvements"].items(),
                key=lambda x: x[1],
                reverse=True,
            ):
                print(f"  + {doc}: +{delta:.1f}")

        # Document regressions
        if show_regressions and comparison["document_regressions"]:
            print(f"\nDocument Regressions (<-2 points):")
            for doc, delta in sorted(
                comparison["document_regressions"].items(), key=lambda x: x[1]
            ):
                print(f"  - {doc}: {delta:.1f}")

        # Summary
        composite_change = comparison["composite"]
        if composite_change["percent_change"] > 1:
            print(
                f"\n✓ Overall improvement: {composite_change['composite']:.1f} (+{composite_change['delta']:.1f})"
            )
        elif composite_change["percent_change"] < -1:
            print(
                f"\n✗ Overall regression: {composite_change['composite']:.1f} ({composite_change['delta']:.1f})"
            )
        else:
            print(f"\n↔ No significant change: {composite_change['composite']:.1f}")

        print(f"{'='*70}\n")


def main():
    parser = argparse.ArgumentParser(
        description="Compare two PDF → Markdown validation reports"
    )
    parser.add_argument("baseline", help="Baseline report.json")
    parser.add_argument("current", help="Current report.json")
    parser.add_argument(
        "--show-improvements", action="store_true", help="Show document improvements"
    )
    parser.add_argument(
        "--show-regressions", action="store_true", help="Show document regressions"
    )

    args = parser.parse_args()

    try:
        comparator = ReportComparator(Path(args.baseline), Path(args.current))
        comparator.print_report(
            show_improvements=args.show_improvements,
            show_regressions=args.show_regressions,
        )
    except Exception as e:
        print(f"✗ Error: {e}", file=sys.stderr)
        import traceback

        traceback.print_exc()
        sys.exit(1)


if __name__ == "__main__":
    main()
