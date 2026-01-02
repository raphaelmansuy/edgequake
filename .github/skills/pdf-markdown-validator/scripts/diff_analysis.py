#!/usr/bin/env python3
"""
PDF → Markdown Validation Diff & Drift Analysis

Show detailed differences between gold (reference) and generated markdown.
Identifies specific errors, drift points, and missing content.
"""

import argparse
import difflib
import json
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, List, Tuple


@dataclass
class DriftPoint:
    """A specific point where content diverges."""

    line_num: int
    gold_line: str
    gen_line: str
    diff_type: str  # missing, extra, mismatch
    severity: str  # critical, major, minor
    category: str  # style, content, structure


class DiffAnalyzer:
    """Analyze differences between gold and generated markdown."""

    def __init__(self, verbose: bool = False):
        self.verbose = verbose

    def analyze_file(self, gold_path: Path, gen_path: Path) -> Dict:
        """Analyze a single gold/gen pair."""
        if not gold_path.exists() or not gen_path.exists():
            return None

        gold_content = gold_path.read_text(encoding="utf-8")
        gen_content = gen_path.read_text(encoding="utf-8")

        gold_lines = gold_content.split("\n")
        gen_lines = gen_content.split("\n")

        drift_points = self._find_drifts(gold_lines, gen_lines)
        summary = self._compute_summary(drift_points, gold_lines, gen_lines)

        return {
            "file": gold_path.stem,
            "gold_lines": len(gold_lines),
            "gen_lines": len(gen_lines),
            "drift_points": drift_points,
            "summary": summary,
            "diff_preview": self._get_diff_preview(gold_lines, gen_lines),
        }

    def _find_drifts(self, gold_lines: List[str], gen_lines: List[str]) -> List[Dict]:
        """Find all divergence points."""
        drifts = []

        # Use difflib to find differences
        diff = difflib.SequenceMatcher(None, gold_lines, gen_lines)

        for tag, i1, i2, j1, j2 in diff.get_opcodes():
            if tag == "replace":
                # Content mismatch
                for offset in range(max(i2 - i1, j2 - j1)):
                    gold_idx = i1 + offset if i1 + offset < i2 else None
                    gen_idx = j1 + offset if j1 + offset < j2 else None

                    gold_line = gold_lines[gold_idx] if gold_idx is not None else ""
                    gen_line = gen_lines[gen_idx] if gen_idx is not None else ""

                    drift = {
                        "type": "mismatch",
                        "line_num": (gold_idx or gen_idx) + 1,
                        "gold": gold_line,
                        "generated": gen_line,
                        "category": self._categorize_drift(gold_line, gen_line),
                        "severity": self._assess_severity(gold_line, gen_line),
                    }
                    drifts.append(drift)

            elif tag == "delete":
                # Lines in gold but missing from generated
                for idx in range(i1, i2):
                    drift = {
                        "type": "missing",
                        "line_num": idx + 1,
                        "gold": gold_lines[idx],
                        "generated": "",
                        "category": self._categorize_missing(gold_lines[idx]),
                        "severity": "major",
                    }
                    drifts.append(drift)

            elif tag == "insert":
                # Extra lines in generated
                for idx in range(j1, j2):
                    drift = {
                        "type": "extra",
                        "line_num": idx + 1,
                        "gold": "",
                        "generated": gen_lines[idx],
                        "category": self._categorize_extra(gen_lines[idx]),
                        "severity": "minor",
                    }
                    drifts.append(drift)

        return drifts

    def _categorize_drift(self, gold: str, gen: str) -> str:
        """Categorize the type of mismatch."""
        if "**" in gold or "*" in gold:
            return "style"
        if "#" in gold:
            return "heading"
        if "|" in gold:
            return "table"
        if "-" in gold or "*" in gold or "+" in gold:
            return "list"
        if "```" in gold:
            return "code"
        return "content"

    def _categorize_missing(self, line: str) -> str:
        """Categorize missing content."""
        line = line.strip()
        if not line:
            return "whitespace"
        if "**" in line or "*" in line:
            return "style"
        if "#" in line:
            return "heading"
        if "|" in line:
            return "table"
        if line.startswith(("-", "*", "+")):
            return "list"
        if "```" in line:
            return "code"
        return "content"

    def _categorize_extra(self, line: str) -> str:
        """Categorize extra content."""
        return self._categorize_missing(line)

    def _assess_severity(self, gold: str, gen: str) -> str:
        """Assess severity of mismatch."""
        if gold.strip() == gen.strip():
            return "minor"  # Just whitespace
        if len(gold) > 0 and len(gen) == 0:
            return "critical"  # Lost content
        if abs(len(gold) - len(gen)) > len(gold) * 0.5:
            return "major"  # Significant content loss
        return "minor"

    def _compute_summary(
        self, drifts: List[Dict], gold_lines: List[str], gen_lines: List[str]
    ) -> Dict:
        """Compute drift summary statistics."""
        if not drifts:
            return {
                "total_drifts": 0,
                "missing_count": 0,
                "extra_count": 0,
                "mismatch_count": 0,
                "critical_count": 0,
                "major_count": 0,
                "minor_count": 0,
                "by_category": {},
            }

        summary = {
            "total_drifts": len(drifts),
            "missing_count": sum(1 for d in drifts if d["type"] == "missing"),
            "extra_count": sum(1 for d in drifts if d["type"] == "extra"),
            "mismatch_count": sum(1 for d in drifts if d["type"] == "mismatch"),
            "critical_count": sum(1 for d in drifts if d["severity"] == "critical"),
            "major_count": sum(1 for d in drifts if d["severity"] == "major"),
            "minor_count": sum(1 for d in drifts if d["severity"] == "minor"),
            "by_category": {},
        }

        # Count by category
        for drift in drifts:
            cat = drift["category"]
            if cat not in summary["by_category"]:
                summary["by_category"][cat] = 0
            summary["by_category"][cat] += 1

        return summary

    def _get_diff_preview(
        self, gold_lines: List[str], gen_lines: List[str], context: int = 2
    ) -> List[str]:
        """Generate unified diff preview."""
        diff = difflib.unified_diff(
            gold_lines,
            gen_lines,
            fromfile="gold",
            tofile="generated",
            lineterm="",
            n=context,
        )
        return list(diff)[:50]  # Limit to first 50 lines


def main():
    parser = argparse.ArgumentParser(
        description="Analyze differences between gold and generated markdown"
    )
    parser.add_argument("gold_file", help="Path to gold (reference) markdown file")
    parser.add_argument("gen_file", help="Path to generated markdown file")
    parser.add_argument("--output-json", help="Output JSON report")
    parser.add_argument(
        "--show-full-diff", action="store_true", help="Show full unified diff"
    )
    parser.add_argument(
        "--filter-severity",
        choices=["critical", "major", "minor"],
        help="Show only drifts of this severity",
    )
    parser.add_argument(
        "--filter-category",
        choices=["style", "content", "structure", "heading", "table", "list", "code"],
        help="Show only drifts of this category",
    )
    parser.add_argument("--verbose", "-v", action="store_true")

    args = parser.parse_args()

    try:
        analyzer = DiffAnalyzer(verbose=args.verbose)
        gold_path = Path(args.gold_file)
        gen_path = Path(args.gen_file)

        if not gold_path.exists():
            print(f"✗ Gold file not found: {gold_path}", file=sys.stderr)
            sys.exit(1)
        if not gen_path.exists():
            print(f"✗ Generated file not found: {gen_path}", file=sys.stderr)
            sys.exit(1)

        result = analyzer.analyze_file(gold_path, gen_path)

        if not result:
            print("✗ Failed to analyze files", file=sys.stderr)
            sys.exit(1)

        # Output JSON if requested
        if args.output_json:
            with open(args.output_json, "w") as f:
                json.dump(result, f, indent=2)
            print(f"✓ Report saved to {args.output_json}")

        # Print report
        print(f"\n{'='*70}")
        print(f"Drift Analysis: {result['file']}")
        print(f"{'='*70}")
        print(f"Gold lines: {result['gold_lines']}")
        print(f"Generated lines: {result['gen_lines']}")

        summary = result["summary"]
        print(f"\nDrift Summary:")
        print(f"  Total drifts: {summary['total_drifts']}")
        print(
            f"  Missing: {summary['missing_count']} | Extra: {summary['extra_count']} | Mismatches: {summary['mismatch_count']}"
        )
        print(
            f"  Critical: {summary['critical_count']} | Major: {summary['major_count']} | Minor: {summary['minor_count']}"
        )

        if summary["by_category"]:
            print(f"\nBy Category:")
            for cat, count in sorted(
                summary["by_category"].items(), key=lambda x: -x[1]
            ):
                print(f"  {cat}: {count}")

        # Show drifts
        drifts = result["drift_points"]
        if drifts:
            print(f"\n{'─'*70}")
            print("Drift Points:")
            print(f"{'─'*70}")

            filtered_drifts = drifts
            if args.filter_severity:
                filtered_drifts = [
                    d for d in filtered_drifts if d["severity"] == args.filter_severity
                ]
            if args.filter_category:
                filtered_drifts = [
                    d for d in filtered_drifts if d["category"] == args.filter_category
                ]

            for i, drift in enumerate(filtered_drifts[:20], 1):  # Show first 20
                severity_icon = (
                    "🔴"
                    if drift["severity"] == "critical"
                    else "🟠" if drift["severity"] == "major" else "🟡"
                )
                print(
                    f"\n{i}. [{severity_icon} {drift['severity'].upper()}] Line {drift['line_num']} ({drift['category']})"
                )
                print(f"   Type: {drift['type']}")

                if drift["gold"]:
                    print(f"   Gold:      {drift['gold'][:60]}")
                if drift["generated"]:
                    print(f"   Generated: {drift['generated'][:60]}")

            if len(filtered_drifts) > 20:
                print(f"\n... and {len(filtered_drifts) - 20} more drifts")

        # Show unified diff preview
        if args.show_full_diff and result["diff_preview"]:
            print(f"\n{'─'*70}")
            print("Unified Diff Preview (first 50 lines):")
            print(f"{'─'*70}")
            for line in result["diff_preview"]:
                if line.startswith("+"):
                    print(f"\033[92m{line}\033[0m")  # Green
                elif line.startswith("-"):
                    print(f"\033[91m{line}\033[0m")  # Red
                else:
                    print(line)

        print(f"\n{'='*70}\n")

    except Exception as e:
        print(f"✗ Error: {e}", file=sys.stderr)
        import traceback

        traceback.print_exc()
        sys.exit(1)


if __name__ == "__main__":
    main()
