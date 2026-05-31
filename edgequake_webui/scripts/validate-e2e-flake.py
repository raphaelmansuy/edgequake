#!/usr/bin/env python3
"""
Validate chromium-gate e2e specs for known flake anti-patterns.
Exit 1 if any integration spec (non @audit/@load/@debug) violates rules.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

E2E = Path(__file__).resolve().parent.parent / "e2e"

BANNED_IN_CHROMIUM = [
    (re.compile(r'waitForLoadState\s*\(\s*["\']networkidle["\']'), "networkidle"),
    (re.compile(r'waitUntil:\s*["\']networkidle["\']'), "networkidle waitUntil"),
    (re.compile(r"localhost:8080"), "hardcoded :8080"),
    (re.compile(r'replace\s*\(\s*["\']:3001["\']'), "port hack :3001→:8080"),
    (re.compile(r'\$\{BASE_URL\}/'), "BASE_URL path join (use relative /path)"),
    (re.compile(r'\$\{FRONTEND_URL\}/'), "FRONTEND_URL path join"),
]

TAG_RE = re.compile(r"@(?:audit|load|debug)")


def is_gated_spec(path: Path, text: str) -> bool:
    """True when spec runs in chromium project (integration gate)."""
    if TAG_RE.search(text):
        return False
    return True


def main() -> int:
    violations: list[str] = []
    for path in sorted(E2E.glob("*.spec.ts")):
        text = path.read_text()
        if not is_gated_spec(path, text):
            continue
        for line_no, line in enumerate(text.splitlines(), 1):
            for pattern, label in BANNED_IN_CHROMIUM:
                if pattern.search(line):
                    violations.append(f"{path.name}:{line_no}: {label}")
        # waitForTimeout in gate specs — warn if > 2s (use expect/waitForResponse)
        for m in re.finditer(r"waitForTimeout\s*\(\s*(\d+)", text):
            ms = int(m.group(1))
            if ms > 2000:
                line_no = text[: m.start()].count("\n") + 1
                violations.append(
                    f"{path.name}:{line_no}: waitForTimeout({ms}) > 2000ms"
                )

    if violations:
        print("E2E flake validation FAILED (chromium gate specs):\n")
        for v in violations[:80]:
            print(f"  ✗ {v}")
        if len(violations) > 80:
            print(f"  ... and {len(violations) - 80} more")
        print(f"\nTotal: {len(violations)} violation(s)")
        print("Fix or tag spec with @audit / @load / @debug to exclude from PR gate.")
        return 1

    print("E2E flake validation PASSED (chromium gate specs clean)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
