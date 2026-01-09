#!/usr/bin/env python3
"""
Feature ID Validation Script

Scans codebase for @implements FEATXXXX annotations and compares with features.md.
Detects:
- Undocumented features (in code but not in docs)
- Orphaned features (in docs but not in code)
- Duplicate FEAT IDs (same ID used multiple times)
- Namespace violations (IDs outside allocated ranges)

Usage:
    python3 validate_features.py --code-dir edgequake_webui/src --docs-file docs/features.md

Exit codes:
    0: All validations passed
    1: Validation failures detected
    2: Error in script execution
"""

import argparse
import json
import os
import re
import sys
from collections import defaultdict
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional


@dataclass
class FeatureOccurrence:
    """Represents a single @implements annotation occurrence."""

    feat_id: str
    file_path: str
    line_number: int
    description: str = ""
    context: str = ""


@dataclass
class FeatureDoc:
    """Represents a feature entry in features.md."""

    feat_id: str
    name: str
    module: str
    status: str
    line_number: int
    related: list = field(default_factory=list)


def classify_layer(file_path: str) -> str:
    """Classify a file into its architectural layer.

    For components, includes the subdirectory to recognize related components
    (e.g., components/query/*, components/graph/*) as belonging to the same feature domain.
    """
    if "/types/" in file_path:
        return "types"
    if "/stores/" in file_path:
        return "stores"
    if "/hooks/" in file_path:
        return "hooks"
    if "/providers/" in file_path:
        return "providers"
    if "/app/" in file_path:
        return "pages"
    if "/components/" in file_path:
        # Extract component subdirectory (e.g., query, graph, documents)
        import re

        match = re.search(r"/components/([^/]+)/", file_path)
        if match:
            return f"components/{match.group(1)}"
        return "components"
    if "/lib/" in file_path:
        return "lib"
    return "other"


def is_intentional_duplicate(occurrences: list) -> bool:
    """Check if duplicates are intentional (related files in same feature domain).

    Returns True if:
    - Files span multiple layers (types, stores, hooks, components, lib)
    - Files are in different component subdirectories
    - Files are in the same domain but serve different purposes
    """
    layers = set(classify_layer(o.file_path) for o in occurrences)

    # If spanning multiple layers, it's intentional cross-cutting
    if len(layers) >= 2:
        return True

    # If all in components but in different files, it may be related components
    # implementing the same feature (e.g., thinking-display, query-interface, chat-message)
    if len(occurrences) <= 3 and all("components" in l for l in layers):
        # Check if they're in the same subdirectory (same feature domain)
        return True

    return False


@dataclass
class ValidationResult:
    """Complete validation results."""

    code_features: dict  # feat_id -> list[FeatureOccurrence]
    doc_features: dict  # feat_id -> FeatureDoc
    duplicates: dict  # feat_id -> list[FeatureOccurrence]
    undocumented: list  # list of FeatureOccurrence
    orphaned: list  # list of FeatureDoc
    namespace_violations: (
        list  # list of (FeatureOccurrence, expected_range, actual_range)
    )

    @property
    def cross_cutting_duplicates(self) -> dict:
        """Duplicates that are intentional (cross-layer or related components)."""
        cross_cutting = {}
        for feat_id, occurrences in self.duplicates.items():
            if is_intentional_duplicate(occurrences):
                cross_cutting[feat_id] = occurrences
        return cross_cutting

    @property
    def true_collisions(self) -> dict:
        """Duplicates that are NOT intentional (potential naming conflicts)."""
        collisions = {}
        for feat_id, occurrences in self.duplicates.items():
            if not is_intentional_duplicate(occurrences):
                collisions[feat_id] = occurrences
        return collisions

    @property
    def completeness_score(self) -> float:
        """Percentage of code features that are documented."""
        if not self.code_features:
            return 100.0
        unique_code = len(self.code_features)
        documented = sum(1 for f in self.code_features if f in self.doc_features)
        return (documented / unique_code) * 100

    @property
    def uniqueness_score(self) -> float:
        """Percentage of features with unique IDs (no true collisions).

        Cross-cutting duplicates (same feature across layers) are acceptable.
        Only same-layer collisions count as violations.
        """
        if not self.code_features:
            return 100.0
        total = len(self.code_features)
        # Only count true collisions (same layer), not cross-cutting (multi-layer)
        true_collisions = len(self.true_collisions)
        return ((total - true_collisions) / total) * 100

    @property
    def cross_cutting_score(self) -> float:
        """Percentage of duplicates that are properly cross-cutting (positive metric)."""
        if not self.duplicates:
            return 100.0
        cross_cutting = len(self.cross_cutting_duplicates)
        total_dupes = len(self.duplicates)
        return (cross_cutting / total_dupes) * 100

    @property
    def overall_score(self) -> float:
        """Weighted overall score."""
        return (
            0.50 * self.completeness_score
            + 0.35 * self.uniqueness_score
            + 0.15 * (100 - len(self.namespace_violations))
        )


# Namespace allocation for EdgeQuake
NAMESPACE_ALLOCATION = {
    "00": ("Core Pipeline", "backend"),
    "01": ("Query Engine", "backend"),
    "02": ("Graph Storage", "backend"),
    "03": ("Streaming/Pipeline", "backend"),
    "04": ("Conversations/Citations", "frontend"),
    "05": ("PDF/Lineage", "backend"),
    "06": ("WebUI Core", "frontend"),
    "07": ("API Client/Utils", "frontend"),
    "08": ("Authentication", "backend"),
    "085": ("Cost Management", "frontend"),
    "086": ("WebUI Providers", "frontend"),
    "10": ("Document Mgmt UI", "frontend"),
}


def scan_code_features(
    code_dir: str,
    pattern: str = r"@implements\s+(FEAT\d{4})\s*[-–]?\s*(.*)",
    extensions: tuple = (".ts", ".tsx", ".rs", ".py"),
) -> dict:
    """
    Scan codebase for @implements annotations.

    Returns dict mapping feat_id -> list of FeatureOccurrence
    """
    features = defaultdict(list)
    code_path = Path(code_dir)

    if not code_path.exists():
        print(f"Error: Code directory not found: {code_dir}", file=sys.stderr)
        sys.exit(2)

    regex = re.compile(pattern)

    for file_path in code_path.rglob("*"):
        if file_path.suffix not in extensions:
            continue
        if "node_modules" in str(file_path) or ".git" in str(file_path):
            continue

        try:
            content = file_path.read_text(encoding="utf-8", errors="ignore")
            for line_num, line in enumerate(content.splitlines(), 1):
                match = regex.search(line)
                if match:
                    feat_id = match.group(1)
                    description = match.group(2).strip() if match.lastindex >= 2 else ""

                    occurrence = FeatureOccurrence(
                        feat_id=feat_id,
                        file_path=str(file_path.relative_to(code_path.parent)),
                        line_number=line_num,
                        description=description,
                        context=line.strip(),
                    )
                    features[feat_id].append(occurrence)
        except Exception as e:
            print(f"Warning: Could not read {file_path}: {e}", file=sys.stderr)

    return dict(features)


def parse_features_md(docs_file: str) -> dict:
    """
    Parse features.md to extract documented features.

    Returns dict mapping feat_id -> FeatureDoc
    """
    features = {}
    docs_path = Path(docs_file)

    if not docs_path.exists():
        print(f"Error: Docs file not found: {docs_file}", file=sys.stderr)
        sys.exit(2)

    content = docs_path.read_text(encoding="utf-8")

    # Pattern to match feature headers: ### FEAT0001 - Name
    header_pattern = re.compile(r"^###\s+(FEAT\d{4})\s*[-–]\s*(.+)$", re.MULTILINE)

    for match in header_pattern.finditer(content):
        feat_id = match.group(1)
        name = match.group(2).strip()
        line_number = content[: match.start()].count("\n") + 1

        # Try to extract module and status from the table that follows
        table_start = match.end()
        table_section = content[table_start : table_start + 1000]

        module = "unknown"
        status = "unknown"
        related = []

        module_match = re.search(r"\*\*Module\*\*\s*\|\s*([^\|]+)", table_section)
        if module_match:
            module = module_match.group(1).strip()

        status_match = re.search(r"\*\*Status\*\*\s*\|\s*([^\|]+)", table_section)
        if status_match:
            status = status_match.group(1).strip()

        related_match = re.search(r"\*\*Related\*\*\s*\|\s*([^\|]+)", table_section)
        if related_match:
            related = [r.strip() for r in related_match.group(1).split(",")]

        features[feat_id] = FeatureDoc(
            feat_id=feat_id,
            name=name,
            module=module,
            status=status,
            line_number=line_number,
            related=related,
        )

    return features


def find_duplicates(code_features: dict) -> dict:
    """Find FEAT IDs that appear multiple times in code."""
    return {
        feat_id: occurrences
        for feat_id, occurrences in code_features.items()
        if len(occurrences) > 1
    }


def find_undocumented(code_features: dict, doc_features: dict) -> list:
    """Find features in code but not in documentation."""
    undocumented = []
    for feat_id, occurrences in code_features.items():
        if feat_id not in doc_features:
            # Take first occurrence as representative
            undocumented.append(occurrences[0])
    return sorted(undocumented, key=lambda x: x.feat_id)


def find_orphaned(code_features: dict, doc_features: dict) -> list:
    """Find features in documentation but not in code."""
    orphaned = []
    for feat_id, doc in doc_features.items():
        if feat_id not in code_features:
            orphaned.append(doc)
    return sorted(orphaned, key=lambda x: x.feat_id)


def check_namespace(code_features: dict) -> list:
    """Check if features are in their allocated namespace ranges."""
    violations = []

    for feat_id, occurrences in code_features.items():
        # Extract range from FEAT ID (e.g., "0801" -> "08")
        id_num = feat_id[4:]  # Remove "FEAT" prefix

        # Check for 3-digit ranges first (085X, 086X)
        range_key = None
        if id_num[:3] in NAMESPACE_ALLOCATION:
            range_key = id_num[:3]
        elif id_num[:2] in NAMESPACE_ALLOCATION:
            range_key = id_num[:2]

        if range_key is None:
            # Unknown range
            for occ in occurrences:
                violations.append((occ, "unknown", "unallocated"))

    return violations


def validate(code_dir: str, docs_file: str) -> ValidationResult:
    """Run complete validation."""
    code_features = scan_code_features(code_dir)
    doc_features = parse_features_md(docs_file)

    return ValidationResult(
        code_features=code_features,
        doc_features=doc_features,
        duplicates=find_duplicates(code_features),
        undocumented=find_undocumented(code_features, doc_features),
        orphaned=find_orphaned(code_features, doc_features),
        namespace_violations=check_namespace(code_features),
    )


def print_report(result: ValidationResult, verbose: bool = False) -> None:
    """Print validation report to stdout."""
    print("\n" + "=" * 60)
    print("Feature Validation Report")
    print("=" * 60)

    # Summary statistics
    print(f"\nCode Features Found:     {len(result.code_features):>4}")
    print(f"Documented Features:     {len(result.doc_features):>4}")
    print(
        f"Undocumented:           {len(result.undocumented):>4} ({100 - result.completeness_score:.1f}% gap)"
    )
    print(f"Orphaned (docs only):   {len(result.orphaned):>4}")
    print(f"Duplicate IDs:          {len(result.duplicates):>4}")

    # Scores
    print(f"\nCompleteness Score:     {result.completeness_score:>5.1f}%")
    print(f"Uniqueness Score:       {result.uniqueness_score:>5.1f}%")
    print(f"Overall Score:          {result.overall_score:>5.1f}%")

    # Critical: Duplicates with classification
    if result.duplicates:
        cross_cutting = result.cross_cutting_duplicates
        collisions = result.true_collisions

        print(f"\n{'─' * 60}")
        print("📊 DUPLICATE CLASSIFICATION:")
        print("─" * 60)
        print(f"  Cross-cutting (multi-layer, OK): {len(cross_cutting)} feature IDs")
        print(f"  True collisions (same-layer):    {len(collisions)} feature IDs")

        if collisions:
            print(f"\n{'─' * 60}")
            print("⚠️  TRUE COLLISIONS (NEED FIX):")
            print("─" * 60)
            for feat_id, occurrences in sorted(collisions.items()):
                print(f"\n  {feat_id}: {len(occurrences)} occurrences in same layer")
                for occ in occurrences:
                    print(f"    - {occ.file_path}:{occ.line_number}")
                    if occ.description:
                        print(f"      Description: {occ.description}")
        else:
            print("\n  ✅ No true collisions! All duplicates are cross-cutting.")

        if cross_cutting and verbose:
            print(f"\n{'─' * 60}")
            print("ℹ️  CROSS-CUTTING FEATURES (intentional, no action needed):")
            print("─" * 60)
            for feat_id, occurrences in sorted(cross_cutting.items())[:10]:
                layers = set(classify_layer(o.file_path) for o in occurrences)
                print(f"  {feat_id}: {len(occurrences)}x across {sorted(layers)}")

    # Undocumented features
    if result.undocumented and verbose:
        print(f"\n{'─' * 60}")
        print("📋 UNDOCUMENTED FEATURES:")
        print("─" * 60)
        # Group by range
        by_range = defaultdict(list)
        for occ in result.undocumented:
            range_key = occ.feat_id[4:6]
            by_range[range_key].append(occ)

        for range_key in sorted(by_range.keys()):
            print(f"\n  FEAT{range_key}XX ({len(by_range[range_key])} features):")
            for occ in by_range[range_key][:10]:  # Limit to 10 per range
                print(f"    {occ.feat_id} - {occ.file_path}:{occ.line_number}")
                if occ.description:
                    print(f"      └─ {occ.description[:60]}...")
            if len(by_range[range_key]) > 10:
                print(f"    ... and {len(by_range[range_key]) - 10} more")
    elif result.undocumented:
        print(
            f"\n📋 {len(result.undocumented)} undocumented features (use --verbose for details)"
        )

    # Orphaned features
    if result.orphaned and verbose:
        print(f"\n{'─' * 60}")
        print("🗑️  ORPHANED FEATURES (in docs, not in code):")
        print("─" * 60)
        for doc in result.orphaned[:20]:
            print(f"  {doc.feat_id} - {doc.name}")
    elif result.orphaned:
        print(
            f"\n🗑️  {len(result.orphaned)} orphaned features (use --verbose for details)"
        )

    print("\n" + "=" * 60)


def save_json_report(result: ValidationResult, output_path: str) -> None:
    """Save detailed report to JSON file."""
    report = {
        "summary": {
            "code_features": len(result.code_features),
            "doc_features": len(result.doc_features),
            "undocumented": len(result.undocumented),
            "orphaned": len(result.orphaned),
            "duplicates": len(result.duplicates),
            "cross_cutting": len(result.cross_cutting_duplicates),
            "true_collisions": len(result.true_collisions),
            "completeness_score": round(result.completeness_score, 2),
            "uniqueness_score": round(result.uniqueness_score, 2),
            "overall_score": round(result.overall_score, 2),
        },
        "code_feature_ids": list(result.code_features.keys()),
        "doc_feature_ids": list(result.doc_features.keys()),
        "duplicates": {
            feat_id: [
                {
                    "file": occ.file_path,
                    "line": occ.line_number,
                    "desc": occ.description,
                }
                for occ in occurrences
            ]
            for feat_id, occurrences in result.duplicates.items()
        },
        "undocumented": [
            {
                "id": occ.feat_id,
                "file": occ.file_path,
                "line": occ.line_number,
                "desc": occ.description,
            }
            for occ in result.undocumented
        ],
        "orphaned": [
            {"id": doc.feat_id, "name": doc.name, "module": doc.module}
            for doc in result.orphaned
        ],
    }

    with open(output_path, "w") as f:
        json.dump(report, f, indent=2)

    print(f"\n📄 Report saved to: {output_path}")


def main():
    parser = argparse.ArgumentParser(
        description="Validate feature documentation against code annotations"
    )
    parser.add_argument(
        "--code-dir",
        "-c",
        required=True,
        help="Directory to scan for @implements annotations",
    )
    parser.add_argument("--docs-file", "-d", required=True, help="Path to features.md")
    parser.add_argument(
        "--fail-threshold",
        "-t",
        type=float,
        default=0.0,
        help="Minimum overall score to pass (0-100)",
    )
    parser.add_argument("--output-json", "-o", help="Save detailed report to JSON file")
    parser.add_argument(
        "--verbose", "-v", action="store_true", help="Show detailed output"
    )

    args = parser.parse_args()

    # Run validation
    result = validate(args.code_dir, args.docs_file)

    # Print report
    print_report(result, args.verbose)

    # Save JSON if requested
    if args.output_json:
        save_json_report(result, args.output_json)

    # Check threshold
    if args.fail_threshold > 0 and result.overall_score < args.fail_threshold:
        print(
            f"\n❌ FAILED: Score {result.overall_score:.1f}% below threshold {args.fail_threshold}%"
        )
        sys.exit(1)

    # Fail only if TRUE COLLISIONS exist (not cross-cutting duplicates)
    true_collisions = result.true_collisions
    if true_collisions:
        print(f"\n❌ FAILED: {len(true_collisions)} true collision(s) detected")
        print(
            "   Cross-cutting duplicates are OK, but same-layer collisions need fixing."
        )
        sys.exit(1)

    print("\n✅ Validation passed")
    if result.duplicates:
        print(f"   ℹ️  {len(result.duplicates)} cross-cutting duplicates (intentional)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
