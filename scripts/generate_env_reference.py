#!/usr/bin/env python3
"""Generate docs/operations/env-reference.md from edgequake/env_registry.toml."""
from __future__ import annotations

import pathlib
import sys

try:
    import tomllib
except ImportError:  # py<3.11
    import tomli as tomllib  # type: ignore

ROOT = pathlib.Path(__file__).resolve().parents[1]
REG = ROOT / "edgequake" / "env_registry.toml"
OUT = ROOT / "docs" / "operations" / "env-reference.md"


def main() -> int:
    data = tomllib.loads(REG.read_text())
    rows = data.get("var", [])
    lines = [
        "---",
        "title: Environment variable reference",
        "description: Generated from edgequake/env_registry.toml (SPEC-163).",
        "---",
        "",
        "> **Generated.** Do not edit by hand. `python3 scripts/generate_env_reference.py`",
        "",
        "| Variable | Purpose | Documented in |",
        "|----------|---------|---------------|",
    ]
    for row in rows:
        docs = ", ".join(f"[doc]({d})" if not str(d).startswith("docs/") else f"[{d}](../{pathlib.Path(d).name})" for d in row.get("docs", []))
        # Keep relative links inside docs/operations to sibling trees.
        doc_links = ", ".join(f"[`{d}`](../../{d})" for d in row.get("docs", []))
        lines.append(f"| `{row['name']}` | {row['purpose']} | {doc_links} |")
    lines.append("")
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text("\n".join(lines))
    print(f"wrote {OUT} ({len(rows)} vars)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
