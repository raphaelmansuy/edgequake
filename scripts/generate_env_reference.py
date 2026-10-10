#!/usr/bin/env python3
"""Verify docs/operations/env-reference.md covers every registry variable.

The env reference is a hand-maintained, grouped page (name, default, purpose).
This script does NOT overwrite it. It reads edgequake/env_registry.toml and
fails if any registered variable name is missing from the page, so the
registry stays the source of truth for coverage.

Usage: python3 scripts/generate_env_reference.py   (exit 1 on missing names)
"""
from __future__ import annotations

import pathlib
import sys

try:
    import tomllib
except ImportError:  # py<3.11
    import tomli as tomllib  # type: ignore

ROOT = pathlib.Path(__file__).resolve().parents[1]
REG = ROOT / "edgequake" / "env_registry.toml"
PAGE = ROOT / "docs" / "operations" / "env-reference.md"


def main() -> int:
    rows = tomllib.loads(REG.read_text()).get("var", [])
    text = PAGE.read_text()
    missing = [row["name"] for row in rows if row["name"] not in text]
    for name in missing:
        print(f"MISSING {name} in {PAGE.relative_to(ROOT)}")
    if missing:
        print(f"FAIL: {len(missing)} registry variable(s) not documented")
        return 1
    print(f"PASS: all {len(rows)} registry variables appear in {PAGE.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
