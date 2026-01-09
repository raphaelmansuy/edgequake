#!/usr/bin/env python3
"""
Namespace Collision Checker

Validates that FEAT IDs in code follow the namespace allocation rules.
Detects potential collisions between teams/modules.

Usage:
    python3 check_namespace.py \
        --code-dir edgequake_webui/src \
        --docs-dir docs
"""

import argparse
import json
import re
import sys
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path
from typing import Optional

# WHY: Strict namespace boundaries prevent team conflicts
NAMESPACE_ALLOCATION = {
    # Backend namespaces
    "00": {"module": "Core Engine", "team": "backend", "range": (0, 99)},
    "01": {"module": "Query Engine", "team": "backend", "range": (100, 199)},
    "02": {"module": "Graph Operations", "team": "backend", "range": (200, 299)},
    "03": {"module": "Streaming", "team": "backend", "range": (300, 399)},
    "04": {"module": "PDF Processing", "team": "backend", "range": (400, 499)},
    "05": {"module": "LLM Integration", "team": "backend", "range": (500, 599)},
    "08": {
        "module": "Authentication (Backend)",
        "team": "backend",
        "range": (800, 809),
    },
    # Frontend namespaces
    "06": {"module": "WebUI Core", "team": "frontend", "range": (600, 699)},
    "07": {"module": "API Client", "team": "frontend", "range": (700, 799)},
    "085": {"module": "Cost Management", "team": "frontend", "range": (850, 859)},
    "086": {"module": "WebUI Providers", "team": "frontend", "range": (860, 869)},
    "087": {"module": "Auth UI", "team": "frontend", "range": (870, 879)},
    "09": {"module": "Dashboard", "team": "frontend", "range": (900, 999)},
    "10": {"module": "Document Management", "team": "frontend", "range": (1000, 1099)},
}


@dataclass
class IdOccurrence:
    feat_id: str
    file_path: str
    line_number: int
    expected_team: str
    actual_team: str


@dataclass
class NamespaceViolation:
    feat_id: str
    declared_in: str  # "backend" or "frontend"
    expected_namespace: str
    actual_prefix: str
    occurrences: list


def infer_team_from_path(file_path: str) -> str:
    """Infer whether a file belongs to backend or frontend."""
    path_lower = file_path.lower()

    # Frontend indicators
    frontend_patterns = [
        "webui",
        "frontend",
        "/src/",
        ".tsx",
        ".jsx",
        "components",
        "hooks",
        "providers",
        "pages",
    ]

    # Backend indicators
    backend_patterns = [
        "crates/",
        "/backend/",
        ".rs",
        "edgequake-core",
        "edgequake-llm",
        "edgequake-storage",
    ]

    for pattern in frontend_patterns:
        if pattern in path_lower:
            return "frontend"

    for pattern in backend_patterns:
        if pattern in path_lower:
            return "backend"

    return "unknown"


def get_expected_team(feat_id: str) -> tuple:
    """Get expected team and module for a feature ID."""
    num = feat_id.replace("FEAT", "")

    # Check 3-digit prefixes first
    for prefix in ["085", "086"]:
        if num.startswith(prefix):
            info = NAMESPACE_ALLOCATION.get(prefix)
            if info:
                return info["team"], info["module"]

    # Then check 2-digit prefix
    prefix = num[:2]
    info = NAMESPACE_ALLOCATION.get(prefix)
    if info:
        return info["team"], info["module"]

    return "unknown", "Unknown"


def scan_for_ids(code_dir: str) -> dict:
    """Scan codebase for @implements FEATXXXX annotations."""
    occurrences = defaultdict(list)

    extensions = [".ts", ".tsx", ".rs", ".js", ".jsx"]

    code_path = Path(code_dir)
    if not code_path.exists():
        print(f"Error: Directory not found: {code_dir}", file=sys.stderr)
        return occurrences

    for ext in extensions:
        for file_path in code_path.rglob(f"*{ext}"):
            try:
                content = file_path.read_text(encoding="utf-8")
                rel_path = str(file_path.relative_to(code_path.parent.parent))
                actual_team = infer_team_from_path(rel_path)

                for i, line in enumerate(content.splitlines(), 1):
                    match = re.search(r"@implements\s+(FEAT\d{4})", line)
                    if match:
                        feat_id = match.group(1)
                        expected_team, _ = get_expected_team(feat_id)

                        occurrences[feat_id].append(
                            IdOccurrence(
                                feat_id=feat_id,
                                file_path=rel_path,
                                line_number=i,
                                expected_team=expected_team,
                                actual_team=actual_team,
                            )
                        )
            except Exception as e:
                print(f"Warning: Could not read {file_path}: {e}", file=sys.stderr)

    return occurrences


def check_violations(occurrences: dict) -> list:
    """Check for namespace violations."""
    violations = []

    for feat_id, occs in occurrences.items():
        if not occs:
            continue

        expected_team, module = get_expected_team(feat_id)

        # Check if any occurrence is in wrong team's code
        mismatches = [
            o
            for o in occs
            if o.actual_team != "unknown" and o.actual_team != expected_team
        ]

        if mismatches:
            violations.append(
                NamespaceViolation(
                    feat_id=feat_id,
                    declared_in=mismatches[0].actual_team,
                    expected_namespace=f"{expected_team} ({module})",
                    actual_prefix=feat_id.replace("FEAT", "")[:2],
                    occurrences=mismatches,
                )
            )

    return violations


def check_range_conflicts(occurrences: dict) -> list:
    """Check for IDs that fall in overlapping ranges."""
    conflicts = []

    # Known conflict ranges
    conflict_zones = [
        (800, 849, "Auth (backend) vs potential Cost overlap"),
        (850, 869, "Cost/Providers (frontend) - ensure no backend use"),
    ]

    for feat_id, occs in occurrences.items():
        num = int(feat_id.replace("FEAT", ""))

        for start, end, desc in conflict_zones:
            if start <= num <= end:
                # Check if used by multiple teams
                teams = set(o.actual_team for o in occs if o.actual_team != "unknown")
                if len(teams) > 1:
                    conflicts.append(
                        {
                            "feat_id": feat_id,
                            "range": f"{start}-{end}",
                            "description": desc,
                            "teams": list(teams),
                            "occurrences": len(occs),
                        }
                    )

    return conflicts


def print_report(
    occurrences: dict,
    violations: list,
    conflicts: list,
    verbose: bool = False,
) -> None:
    """Print namespace check report."""
    print("\n" + "=" * 60)
    print("Namespace Collision Check Report")
    print("=" * 60)

    print(f"\nTotal unique FEAT IDs: {len(occurrences)}")

    # Count by team
    by_team = defaultdict(set)
    for feat_id, occs in occurrences.items():
        team, _ = get_expected_team(feat_id)
        by_team[team].add(feat_id)

    print(f"\nDistribution by namespace:")
    for team in sorted(by_team.keys()):
        print(f"  {team}: {len(by_team[team])} features")

    if violations:
        print(f"\n{'─' * 60}")
        print(f"❌ NAMESPACE VIOLATIONS ({len(violations)} found):")
        print("─" * 60)

        for v in violations[:10]:
            print(f"\n  {v.feat_id}:")
            print(f"    Expected: {v.expected_namespace}")
            print(f"    Found in: {v.declared_in} code")
            for occ in v.occurrences[:3]:
                print(f"      - {occ.file_path}:{occ.line_number}")

        if len(violations) > 10:
            print(f"\n  ... and {len(violations) - 10} more violations")
    else:
        print(f"\n✅ No namespace violations found")

    if conflicts:
        print(f"\n{'─' * 60}")
        print(f"⚠️  RANGE CONFLICTS ({len(conflicts)} found):")
        print("─" * 60)

        for c in conflicts:
            print(f"\n  {c['feat_id']}:")
            print(f"    Range: {c['range']} ({c['description']})")
            print(f"    Used by teams: {', '.join(c['teams'])}")
    else:
        print(f"\n✅ No range conflicts found")

    print("\n" + "=" * 60)


def main():
    parser = argparse.ArgumentParser(description="Check FEAT ID namespace allocation")
    parser.add_argument(
        "--code-dir", "-c", required=True, help="Root directory to scan"
    )
    parser.add_argument("--output-report", "-o", help="Save report to JSON file")
    parser.add_argument(
        "--verbose", "-v", action="store_true", help="Show detailed output"
    )
    parser.add_argument(
        "--fail-on-violation",
        action="store_true",
        help="Exit with error if violations found",
    )

    args = parser.parse_args()

    # Scan code
    print(f"Scanning {args.code_dir} for FEAT IDs...")
    occurrences = scan_for_ids(args.code_dir)

    # Check violations
    violations = check_violations(occurrences)
    conflicts = check_range_conflicts(occurrences)

    # Print report
    print_report(occurrences, violations, conflicts, args.verbose)

    # Save JSON if requested
    if args.output_report:
        report = {
            "total_features": len(occurrences),
            "violations": [
                {
                    "feat_id": v.feat_id,
                    "expected": v.expected_namespace,
                    "actual_team": v.declared_in,
                    "files": [o.file_path for o in v.occurrences],
                }
                for v in violations
            ],
            "conflicts": conflicts,
            "distribution": {},
        }
        # Calculate distribution by team
        for feat_id in occurrences.keys():
            team, _ = get_expected_team(feat_id)
            if team not in report["distribution"]:
                report["distribution"][team] = []
            report["distribution"][team].append(feat_id)
        with open(args.output_report, "w") as f:
            json.dump(report, f, indent=2)
        print(f"\n📄 Report saved to: {args.output_report}")

    # Exit with error if violations and flag set
    if args.fail_on_violation and violations:
        print(f"\n❌ FAILED: {len(violations)} namespace violations found")
        sys.exit(1)

    return 0


if __name__ == "__main__":
    sys.exit(main())
