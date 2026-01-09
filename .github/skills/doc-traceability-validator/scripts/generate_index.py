#!/usr/bin/env python3
"""
Generate a feature index from docs/features.md for quick lookup.
Creates a searchable index with feature ID, name, module, and status.
"""

import json
import re
from pathlib import Path


def generate_index(features_file: str, output_format: str = "markdown") -> str:
    """Parse features.md and generate an index."""
    content = Path(features_file).read_text(encoding="utf-8")

    # Pattern to match feature headers: ### FEAT0001 - Name
    header_pattern = re.compile(r"^###\s+(FEAT\d{4})\s*[-–]\s*(.+)$", re.MULTILINE)

    features = []

    for match in header_pattern.finditer(content):
        feat_id = match.group(1)
        name = match.group(2).strip()

        # Extract module and status from the table that follows
        table_start = match.end()
        table_section = content[table_start : table_start + 800]

        module = "unknown"
        status = "unknown"

        module_match = re.search(r"\*\*Module\*\*\s*\|\s*([^\|]+)", table_section)
        if module_match:
            module = module_match.group(1).strip()

        status_match = re.search(r"\*\*Status\*\*\s*\|\s*([^\|]+)", table_section)
        if status_match:
            status = status_match.group(1).strip()
            # Clean up status emoji
            if "✅" in status:
                status = "Stable"
            elif "🚧" in status:
                status = "In Progress"
            elif "📝" in status or "Planned" in status:
                status = "Planned"

        features.append(
            {"id": feat_id, "name": name, "module": module, "status": status}
        )

    if output_format == "json":
        return json.dumps(features, indent=2)

    # Markdown format
    lines = [
        "# Feature Index",
        "",
        f"> Auto-generated from docs/features.md | {len(features)} features",
        "",
        "| ID | Name | Module | Status |",
        "|----|------|--------|--------|",
    ]

    for f in sorted(features, key=lambda x: x["id"]):
        lines.append(
            f"| {f['id']} | {f['name'][:50]} | {f['module'][:20]} | {f['status']} |"
        )

    return "\n".join(lines)


def main():
    import argparse

    parser = argparse.ArgumentParser(description="Generate feature index")
    parser.add_argument(
        "--features", default="docs/features.md", help="Path to features.md"
    )
    parser.add_argument("--output", default="docs/feature-index.md", help="Output file")
    parser.add_argument("--format", default="markdown", choices=["markdown", "json"])

    args = parser.parse_args()

    index = generate_index(args.features, args.format)

    Path(args.output).write_text(index, encoding="utf-8")
    print(f"✅ Generated feature index: {args.output}")
    print(f"   Features indexed: {index.count('FEAT')}")


if __name__ == "__main__":
    main()
