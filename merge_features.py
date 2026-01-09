#!/usr/bin/env python3
"""
Merge new feature entries into docs/features.md maintaining proper structure.
"""

import re
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Set


def parse_existing_features(content: str) -> Dict[str, str]:
    """Parse existing features.md and return dict of FEATXXXX -> full entry."""
    features = {}
    current_feat = None
    current_lines = []

    for line in content.split("\n"):
        # Detect start of new feature
        if line.startswith("### FEAT"):
            if current_feat:
                features[current_feat] = "\n".join(current_lines)
            match = re.match(r"### (FEAT\d{4})", line)
            if match:
                current_feat = match.group(1)
                current_lines = [line]
        elif current_feat:
            current_lines.append(line)
            # Check if feature entry ends (next section or summary)
            if line.startswith("##") and not line.startswith("###"):
                features[current_feat] = "\n".join(current_lines[:-1])
                current_feat = None
                current_lines = []

    if current_feat:
        features[current_feat] = "\n".join(current_lines)

    return features


def parse_new_features(content: str) -> Dict[str, str]:
    """Parse generated features from /tmp/new_features.md."""
    features = {}
    current_feat = None
    current_lines = []

    lines = content.split("\n")
    for i, line in enumerate(lines):
        if line.startswith("### FEAT"):
            if current_feat:
                features[current_feat] = "\n".join(current_lines)
            match = re.match(r"### (FEAT\d{4})", line)
            if match:
                current_feat = match.group(1)
                current_lines = [line]
        elif current_feat:
            current_lines.append(line)
            # Feature ends with ---
            if line.strip() == "---":
                features[current_feat] = "\n".join(current_lines)
                current_feat = None
                current_lines = []

    if current_feat:
        features[current_feat] = "\n".join(current_lines)

    return features


def update_index_table(content: str, new_count: int) -> str:
    """Update Quick Reference Index with new count."""
    lines = content.split("\n")
    result = []
    total_updated = False

    for line in lines:
        if "| **TOTAL**" in line and not total_updated:
            # Update total count
            match = re.search(r"\*\*(\d+)\*\*", line)
            if match:
                old_count = int(match.group(1))
                new_total = old_count + new_count
                line = re.sub(r"\*\*\d+\*\*", f"**{new_total}**", line)
                total_updated = True
        result.append(line)

    return "\n".join(result)


def merge_features():
    """Main merge function."""
    # Read existing features
    existing_path = Path("docs/features.md")
    existing_content = existing_path.read_text()

    # Read new features
    new_path = Path("/tmp/new_features.md")
    new_content = new_path.read_text()

    # Parse both
    existing_features = parse_existing_features(existing_content)
    new_features = parse_new_features(new_content)

    print(f"📊 Existing features: {len(existing_features)}")
    print(f"📊 New features: {len(new_features)}")

    # Check for conflicts
    conflicts = set(existing_features.keys()) & set(new_features.keys())
    if conflicts:
        print(f"⚠️  {len(conflicts)} conflicts found: {sorted(conflicts)[:10]}")
        # Keep existing entries for conflicts
        for feat_id in conflicts:
            new_features.pop(feat_id)

    print(f"✅ Merging {len(new_features)} truly new features")

    # Find insertion point (before "## Summary Statistics")
    lines = existing_content.split("\n")
    summary_idx = None
    for i, line in enumerate(lines):
        if line.startswith("## Summary Statistics"):
            summary_idx = i
            break

    if summary_idx is None:
        print("❌ Could not find Summary Statistics section")
        return

    # Insert new features before summary
    new_section = f"\n---\n\n## Newly Discovered Features (Auto-Generated)\n\n"
    new_section += f"**Added**: {datetime.now().strftime('%Y-%m-%d')}\n\n"

    # Group by range for better organization
    new_features_sorted = sorted(new_features.items(), key=lambda x: int(x[0][4:]))
    for feat_id, feat_content in new_features_sorted:
        new_section += feat_content + "\n\n"

    # Insert new section
    updated_lines = lines[:summary_idx] + [new_section] + lines[summary_idx:]
    updated_content = "\n".join(updated_lines)

    # Update index table
    updated_content = update_index_table(updated_content, len(new_features))

    # Update version
    updated_content = re.sub(
        r"\*\*Version\*\*: \d+\.\d+\.\d+", "**Version**: 1.4.0", updated_content
    )

    # Update last updated date
    updated_content = re.sub(
        r"\*\*Last Updated\*\*: \d{4}-\d{2}-\d{2}",
        f'**Last Updated**: {datetime.now().strftime("%Y-%m-%d")}',
        updated_content,
    )

    # Write back
    existing_path.write_text(updated_content)
    print(f"✅ Merged {len(new_features)} features into docs/features.md")
    print(f"📝 Version updated to 1.4.0")
    print(f"📅 Last updated: {datetime.now().strftime('%Y-%m-%d')}")


if __name__ == "__main__":
    merge_features()
