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
    (re.compile(r"default-workspace"), "stale default-workspace slug (use bootstrap slug)"),
]

TAG_RE = re.compile(r"@(?:audit|load|debug)")


def is_gated_spec(path: Path, text: str) -> bool:
    """True when spec runs in chromium project (integration gate)."""
    if TAG_RE.search(text):
        return False
    return True


BACKEND_MARKERS = (
    "helpers/backend-url",
    "helpers/bootstrap-ui",
    "helpers/spec013-api",
    "createTenantWorkspaceViaApi",
    "bootstrapDeterministicUiContext",
)


def check_screenshot_path_gate(path: Path, text: str) -> list[str]:
    """All specs: intentional captures must not use Playwright test-results/."""
    violations: list[str] = []
    for line_no, line in enumerate(text.splitlines(), 1):
        if re.search(r"""path:\s*["']test-results/""", line):
            violations.append(
                f"{path.name}:{line_no}: use e2e/screenshots/ or audit_ui/screenshots/ (screenshot-paths.ts)"
            )
    return violations


def check_query_response_gate(path: Path, text: str) -> list[str]:
    """Specs that wait for LLM chat must be @load (or @audit/@debug)."""
    if TAG_RE.search(text):
        return []
    if "waitForQueryResponse" not in text:
        return []
    return [
        f"{path.name}: uses waitForQueryResponse but missing @load/@audit/@debug tag"
    ]


def check_live_stack_gate(path: Path, text: str) -> list[str]:
    """Chromium specs that touch the API must call skipUnlessLiveStack()."""
    if TAG_RE.search(text):
        return []
    if not any(m in text for m in BACKEND_MARKERS):
        return []
    if re.search(r"skipUnlessLiveStack\s*\(\s*\)", text):
        return []
    return [f"{path.name}: missing skipUnlessLiveStack() for backend/bootstrap spec"]


def main() -> int:
    violations: list[str] = []
    for path in sorted(E2E.glob("*.spec.ts")):
        text = path.read_text()
        violations.extend(check_live_stack_gate(path, text))
        violations.extend(check_query_response_gate(path, text))
        violations.extend(check_screenshot_path_gate(path, text))
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
