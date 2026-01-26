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
