#!/usr/bin/env python3
"""
Feature Registry Generator

Scans codebase for @implements FEATXXXX annotations and generates
markdown entries for features.md.

This is the "Code is Law" embodiment - generate documentation FROM code.

Usage:
    python3 generate_registry.py \
        --code-dir edgequake_webui/src \
        --output features_update.md
"""

import argparse
import json
import re
import sys
from collections import defaultdict
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import Optional


@dataclass
class FeatureEntry:
    """A feature discovered from code."""

    feat_id: str
    file_path: str
    line_number: int
    context_lines: list
    description: str = ""
    module: str = ""


# WHY: Namespace allocation prevents ID collisions
NAMESPACE_ALLOCATION = {
    "00": ("Core Engine", "Backend"),
    "01": ("Query Engine", "Backend"),
    "02": ("Graph Operations", "Backend"),
    "03": ("Streaming", "Backend"),
    "04": ("PDF Processing", "Backend"),
    "05": ("LLM Integration", "Backend"),
    "06": ("WebUI Core", "Frontend"),
    "07": ("API Client", "Frontend"),
    "08": ("Authentication", "Backend"),
    "085": ("Cost Management", "Frontend"),
    "086": ("WebUI Providers", "Frontend"),
    "10": ("Document Management", "Frontend"),
}


def get_namespace_info(feat_id: str) -> tuple:
    """Get module and team for a feature ID."""
    # Extract numeric part
    num = feat_id.replace("FEAT", "")

    # Check 3-digit prefixes first
    for prefix in ["085", "086"]:
        if num.startswith(prefix):
            return NAMESPACE_ALLOCATION.get(prefix, ("Unknown", "Unknown"))

    # Then check 2-digit prefix
    prefix = num[:2]
    return NAMESPACE_ALLOCATION.get(prefix, ("Unknown", "Unknown"))


def infer_description(context: list, file_path: str) -> str:
    """Infer a description from context and file path."""
    # Try to extract from JSDoc or Rust doc comments
    for line in context:
        # JSDoc: * @description Something
        match = re.search(r"\*\s*@description\s+(.+)", line)
        if match:
            return match.group(1).strip()

        # JSDoc: * Something descriptive
        match = re.search(r"^\s*\*\s+([A-Z][^@\*]+)", line)
        if match:
            desc = match.group(1).strip()
            if len(desc) > 10:
                return desc

        # Rust: /// Something
        match = re.search(r"///\s*(.+)", line)
        if match:
            return match.group(1).strip()

    # Fall back to file path inference
    path = Path(file_path)
    name = path.stem

    # Convert camelCase/PascalCase to words
    words = re.sub(r"([a-z])([A-Z])", r"\1 \2", name)
    words = words.replace("_", " ").replace("-", " ")

    return f"{words.title()} functionality"


def scan_features(code_dir: str) -> dict:
    """Scan codebase for @implements FEATXXXX annotations."""
    features = defaultdict(list)

    extensions = [".ts", ".tsx", ".rs", ".js", ".jsx"]

    code_path = Path(code_dir)
    if not code_path.exists():
        print(f"Error: Directory not found: {code_dir}", file=sys.stderr)
        return features

    for ext in extensions:
        for file_path in code_path.rglob(f"*{ext}"):
            try:
                content = file_path.read_text(encoding="utf-8")
                lines = content.splitlines()

                for i, line in enumerate(lines, 1):
                    match = re.search(r"@implements\s+(FEAT\d{4})", line)
                    if match:
                        feat_id = match.group(1)

                        # Get context (5 lines before and after)
                        start = max(0, i - 6)
                        end = min(len(lines), i + 5)
                        context = lines[start:end]

                        entry = FeatureEntry(
                            feat_id=feat_id,
                            file_path=str(
                                file_path.relative_to(code_path.parent.parent)
                            ),
                            line_number=i,
                            context_lines=context,
                        )

                        # Add inferred description
                        entry.description = infer_description(context, str(file_path))

                        # Add namespace info
                        module, team = get_namespace_info(feat_id)
                        entry.module = module

                        features[feat_id].append(entry)

            except Exception as e:
                print(f"Warning: Could not read {file_path}: {e}", file=sys.stderr)

    return features


def generate_markdown(features: dict, existing_feats: set = None) -> str:
    """Generate markdown entries for discovered features."""
    if existing_feats is None:
        existing_feats = set()

    # Find new features
    new_feats = {fid for fid in features.keys() if fid not in existing_feats}

    if not new_feats:
        return "# No New Features Found\n\nAll discovered features are already documented.\n"

    lines = [
        "# New Features Discovered from Code Scan",
        "",
        f"Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}",
        "",
        f"Total new features: {len(new_feats)}",
        "",
        "---",
        "",
    ]

    # Group by module
    by_module = defaultdict(list)
    for fid in sorted(new_feats):
        entries = features[fid]
        if entries:
            by_module[entries[0].module].append((fid, entries))

    for module in sorted(by_module.keys()):
        lines.append(f"## {module}")
        lines.append("")

        for feat_id, entries in by_module[module]:
            primary = entries[0]

            lines.append(f"### {feat_id} - {primary.description}")
            lines.append("")
            lines.append(f"**Module:** {primary.module}")
            lines.append("")
            lines.append(
                f"**Source:** `{primary.file_path}` (line {primary.line_number})"
            )
            lines.append("")

            if len(entries) > 1:
                lines.append("**Additional locations:**")
                for e in entries[1:5]:
                    lines.append(f"- `{e.file_path}` (line {e.line_number})")
                if len(entries) > 5:
                    lines.append(f"- ... and {len(entries) - 5} more")
                lines.append("")

            lines.append("**Status:** `ACTIVE`")
            lines.append("")
            lines.append("---")
            lines.append("")

    return "\n".join(lines)


def generate_index_update(features: dict, existing_feats: set = None) -> str:
    """Generate index table entries for new features."""
    if existing_feats is None:
        existing_feats = set()

    new_feats = {fid for fid in features.keys() if fid not in existing_feats}

    if not new_feats:
        return ""

    lines = [
        "## Index Update (paste into features.md index table)",
        "",
        "| ID | Name | Status | Updated |",
        "|----|----|--------|---------|",
    ]

    for fid in sorted(new_feats):
        entries = features[fid]
        if entries:
            desc = (
                entries[0].description[:40] + "..."
                if len(entries[0].description) > 40
                else entries[0].description
            )
            lines.append(
                f"| {fid} | {desc} | Active | {datetime.now().strftime('%Y-%m-%d')} |"
            )

    return "\n".join(lines)


def parse_existing_features(features_file: str) -> set:
    """Parse existing features.md to get already-documented IDs."""
    path = Path(features_file)
    if not path.exists():
        return set()

    content = path.read_text(encoding="utf-8")
    return set(re.findall(r"FEAT\d{4}", content))


def print_summary(features: dict, existing: set, new: set) -> None:
    """Print generation summary."""
    print("\n" + "=" * 60)
    print("Feature Registry Generation Summary")
    print("=" * 60)

    print(f"\nTotal features found in code: {len(features)}")
    print(f"Already documented:           {len(features.keys() & existing)}")
    print(f"New features to document:     {len(new)}")

    # Check for duplicates
    dups = [(fid, entries) for fid, entries in features.items() if len(entries) > 1]
    if dups:
        print(f"\n⚠️  Features with multiple implementations: {len(dups)}")
        for fid, entries in dups[:5]:
            print(f"    {fid}: {len(entries)} locations")

    print("\n" + "=" * 60)


def main():
    parser = argparse.ArgumentParser(
        description="Generate feature registry entries from code"
    )
    parser.add_argument(
        "--code-dir",
        "-c",
        required=True,
        help="Directory to scan for @implements annotations",
    )
    parser.add_argument(
        "--existing", "-e", help="Path to existing features.md (to detect new features)"
    )
    parser.add_argument("--output", "-o", help="Output file for generated markdown")
    parser.add_argument("--json", "-j", help="Output file for JSON registry")
    parser.add_argument(
        "--index-only", action="store_true", help="Only generate index table entries"
    )
    parser.add_argument(
        "--verbose", "-v", action="store_true", help="Show detailed output"
    )

    args = parser.parse_args()

    # Scan code
    print(f"Scanning {args.code_dir} for @implements annotations...")
    features = scan_features(args.code_dir)

    # Get existing features
    existing = set()
    if args.existing:
        existing = parse_existing_features(args.existing)
        print(f"Found {len(existing)} existing features in {args.existing}")

    # Find new features
    new_feats = set(features.keys()) - existing

    # Print summary
    print_summary(features, existing, new_feats)

    # Generate output
    if args.index_only:
        output = generate_index_update(features, existing)
    else:
        output = generate_markdown(features, existing)
        output += "\n\n" + generate_index_update(features, existing)

    # Write or print output
    if args.output:
        Path(args.output).write_text(output, encoding="utf-8")
        print(f"\n📄 Markdown saved to: {args.output}")
    else:
        print("\n" + output)

    # Save JSON if requested
    if args.json:
        registry = {
            "generated": datetime.now().isoformat(),
            "total_features": len(features),
            "new_features": len(new_feats),
            "features": {
                fid: [
                    {
                        "file": e.file_path,
                        "line": e.line_number,
                        "description": e.description,
                        "module": e.module,
                    }
                    for e in entries
                ]
                for fid, entries in features.items()
            },
        }
        with open(args.json, "w") as f:
            json.dump(registry, f, indent=2)
        print(f"📄 JSON registry saved to: {args.json}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
