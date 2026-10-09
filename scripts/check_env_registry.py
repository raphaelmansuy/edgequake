#!/usr/bin/env python3
"""Fail if env_registry.toml names are missing from their listed docs."""
from __future__ import annotations

import pathlib
import sys

try:
    import tomllib
except ImportError:
    import tomli as tomllib  # type: ignore

ROOT = pathlib.Path(__file__).resolve().parents[1]
REG = ROOT / "edgequake" / "env_registry.toml"


def main() -> int:
    data = tomllib.loads(REG.read_text())
    failed = 0
    for row in data.get("var", []):
        name = row["name"]
        for doc in row.get("docs", []):
            path = ROOT / doc
            if not path.is_file():
                print(f"MISSING FILE {doc} (for {name})")
                failed += 1
                continue
            text = path.read_text()
            if name not in text:
                print(f"MISSING {name} in {doc}")
                failed += 1
    if failed:
        print(f"FAIL: {failed} env-registry mismatches")
        return 1
    print("PASS: env registry names appear in listed docs")
    return 0


if __name__ == "__main__":
    sys.exit(main())
