#!/usr/bin/env python3
"""
Traceability Chain Validator

Validates the complete FEAT ↔ BR ↔ UC traceability chain:
- FEAT references in BR/UC documents resolve correctly
- BR references in FEAT/UC documents resolve correctly
- UC references in FEAT/BR documents resolve correctly

Usage:
    python3 validate_traceability.py \
        --code-dir edgequake_webui/src \
        --features docs/features.md \
        --rules docs/business_rules.md \
        --usecases docs/use_cases.md
"""

import argparse
import json
import re
import sys
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path
from typing import Optional


@dataclass
class Reference:
    """A reference from one document to another."""

    source_file: str
    source_line: int
    ref_id: str
    ref_type: str  # "FEAT", "BR", "UC"


@dataclass
class BrokenRef:
    """A broken reference (target not found)."""

    ref: Reference
    reason: str


def extract_ids(content: str, pattern: str) -> set:
    """Extract all IDs matching a pattern from content."""
    return set(re.findall(pattern, content))


def extract_refs(content: str, file_path: str) -> list:
    """Extract all FEAT/BR/UC references from content."""
    refs = []

    patterns = [
        (r"FEAT\d{4}", "FEAT"),
        (r"BR\d{4}", "BR"),
        (r"UC\d{4}", "UC"),
    ]

    for line_num, line in enumerate(content.splitlines(), 1):
        for pattern, ref_type in patterns:
            for match in re.finditer(pattern, line):
                refs.append(
                    Reference(
                        source_file=file_path,
                        source_line=line_num,
                        ref_id=match.group(0),
                        ref_type=ref_type,
                    )
                )

    return refs


def parse_registry(file_path: str, id_pattern: str) -> set:
    """Parse a registry file and extract defined IDs."""
    path = Path(file_path)
    if not path.exists():
        print(f"Warning: File not found: {file_path}", file=sys.stderr)
        return set()

    content = path.read_text(encoding="utf-8")

    # Look for definition headers: ### BRXXXX - Name or ### FEATXXXX - Name
    header_pattern = rf"^###\s+({id_pattern})\s*[-–]"
    return set(re.findall(header_pattern, content, re.MULTILINE))


def validate_refs(refs: list, valid_feats: set, valid_brs: set, valid_ucs: set) -> list:
    """Validate all references against known IDs."""
    broken = []

    for ref in refs:
        if ref.ref_type == "FEAT":
            if ref.ref_id not in valid_feats:
                broken.append(BrokenRef(ref, f"FEAT not found in features.md"))
        elif ref.ref_type == "BR":
            if ref.ref_id not in valid_brs:
                broken.append(BrokenRef(ref, f"BR not found in business_rules.md"))
        elif ref.ref_type == "UC":
            if ref.ref_id not in valid_ucs:
                broken.append(BrokenRef(ref, f"UC not found in use_cases.md"))

    return broken


def calculate_coverage(
    features: set,
    rules: set,
    usecases: set,
    feat_refs: list,
    br_refs: list,
    uc_refs: list,
) -> dict:
    """Calculate cross-reference coverage metrics."""
    # Features referenced by BRs
    feats_in_brs = set(r.ref_id for r in br_refs if r.ref_type == "FEAT")
    # Features referenced by UCs
    feats_in_ucs = set(r.ref_id for r in uc_refs if r.ref_type == "FEAT")

    # BRs referenced by features
    brs_in_feats = set(r.ref_id for r in feat_refs if r.ref_type == "BR")
    # BRs referenced by UCs
    brs_in_ucs = set(r.ref_id for r in uc_refs if r.ref_type == "BR")

    # UCs referenced by features
    ucs_in_feats = set(r.ref_id for r in feat_refs if r.ref_type == "UC")
    # UCs referenced by BRs
    ucs_in_brs = set(r.ref_id for r in br_refs if r.ref_type == "UC")

    return {
        "features_with_br_refs": len(feats_in_brs & features),
        "features_total": len(features),
        "brs_with_feat_refs": len(brs_in_feats & rules),
        "brs_total": len(rules),
        "ucs_with_br_refs": len(ucs_in_brs & usecases),
        "ucs_total": len(usecases),
    }


def print_report(
    features: set,
    rules: set,
    usecases: set,
    broken_refs: list,
    coverage: dict,
    verbose: bool = False,
) -> None:
    """Print traceability report."""
    print("\n" + "=" * 60)
    print("Traceability Validation Report")
    print("=" * 60)

    print(f"\nDocumented Items:")
    print(f"  Features:     {len(features):>4}")
    print(f"  Business Rules: {len(rules):>4}")
    print(f"  Use Cases:    {len(usecases):>4}")

    print(f"\nCross-Reference Coverage:")
    if coverage["features_total"] > 0:
        feat_pct = (
            coverage["features_with_br_refs"] / coverage["features_total"]
        ) * 100
        print(
            f"  Features with BR refs:  {coverage['features_with_br_refs']}/{coverage['features_total']} ({feat_pct:.1f}%)"
        )

    if coverage["brs_total"] > 0:
        br_pct = (coverage["brs_with_feat_refs"] / coverage["brs_total"]) * 100
        print(
            f"  BRs with FEAT refs:     {coverage['brs_with_feat_refs']}/{coverage['brs_total']} ({br_pct:.1f}%)"
        )

    if coverage["ucs_total"] > 0:
        uc_pct = (coverage["ucs_with_br_refs"] / coverage["ucs_total"]) * 100
        print(
            f"  UCs with BR refs:       {coverage['ucs_with_br_refs']}/{coverage['ucs_total']} ({uc_pct:.1f}%)"
        )

    if broken_refs:
        print(f"\n{'─' * 60}")
        print(f"⚠️  BROKEN REFERENCES ({len(broken_refs)} found):")
        print("─" * 60)

        # Group by source file
        by_file = defaultdict(list)
        for br in broken_refs:
            by_file[br.ref.source_file].append(br)

        for file_path in sorted(by_file.keys()):
            print(f"\n  {file_path}:")
            for br in sorted(by_file[file_path], key=lambda x: x.ref.source_line)[:10]:
                print(f"    Line {br.ref.source_line}: {br.ref.ref_id} - {br.reason}")
            if len(by_file[file_path]) > 10:
                print(f"    ... and {len(by_file[file_path]) - 10} more")
    else:
        print(f"\n✅ No broken references found")

    print("\n" + "=" * 60)


def main():
    parser = argparse.ArgumentParser(
        description="Validate FEAT ↔ BR ↔ UC traceability chain"
    )
    parser.add_argument("--features", "-f", required=True, help="Path to features.md")
    parser.add_argument(
        "--rules", "-r", required=True, help="Path to business_rules.md"
    )
    parser.add_argument("--usecases", "-u", required=True, help="Path to use_cases.md")
    parser.add_argument("--output-report", "-o", help="Save report to JSON file")
    parser.add_argument(
        "--verbose", "-v", action="store_true", help="Show detailed output"
    )
    parser.add_argument(
        "--fail-on-broken",
        action="store_true",
        help="Exit with error if broken references found",
    )

    args = parser.parse_args()

    # Parse registries
    features = parse_registry(args.features, r"FEAT\d{4}")
    rules = parse_registry(args.rules, r"BR\d{4}")
    usecases = parse_registry(args.usecases, r"UC\d{4}")

    # Extract references from each file
    feat_content = Path(args.features).read_text(encoding="utf-8")
    br_content = Path(args.rules).read_text(encoding="utf-8")
    uc_content = Path(args.usecases).read_text(encoding="utf-8")

    feat_refs = extract_refs(feat_content, args.features)
    br_refs = extract_refs(br_content, args.rules)
    uc_refs = extract_refs(uc_content, args.usecases)

    all_refs = feat_refs + br_refs + uc_refs

    # Validate references
    broken_refs = validate_refs(all_refs, features, rules, usecases)

    # Calculate coverage
    coverage = calculate_coverage(
        features, rules, usecases, feat_refs, br_refs, uc_refs
    )

    # Print report
    print_report(features, rules, usecases, broken_refs, coverage, args.verbose)

    # Save JSON if requested
    if args.output_report:
        report = {
            "summary": {
                "features": len(features),
                "rules": len(rules),
                "usecases": len(usecases),
                "broken_refs": len(broken_refs),
            },
            "coverage": coverage,
            "broken_refs": [
                {
                    "file": br.ref.source_file,
                    "line": br.ref.source_line,
                    "ref": br.ref.ref_id,
                    "reason": br.reason,
                }
                for br in broken_refs
            ],
        }
        with open(args.output_report, "w") as f:
            json.dump(report, f, indent=2)
        print(f"\n📄 Report saved to: {args.output_report}")

    # Exit with error if broken refs and flag set
    if args.fail_on_broken and broken_refs:
        print(f"\n❌ FAILED: {len(broken_refs)} broken references found")
        sys.exit(1)

    return 0


if __name__ == "__main__":
    sys.exit(main())
