#!/usr/bin/env python3
"""Add skipUnlessLiveStack() beforeEach to backend-dependent e2e specs."""
from __future__ import annotations

import re
from pathlib import Path

E2E = Path(__file__).resolve().parent.parent / "e2e"
IMPORT = 'import { skipUnlessLiveStack } from "./helpers/live-stack";\n'
HOOK = """
test.beforeEach(() => {
  skipUnlessLiveStack();
});
"""

MARKERS = (
    "helpers/backend-url",
    "helpers/bootstrap-ui",
    "helpers/spec013-api",
    "waitForBackendHealthy",
    "createTenantWorkspaceViaApi",
    "bootstrapDeterministicUiContext",
)


def has_live_stack_hook(text: str) -> bool:
    return bool(re.search(r"skipUnlessLiveStack\s*\(\s*\)", text))


def needs_gate(text: str) -> bool:
    if has_live_stack_hook(text):
        return False
    return any(m in text for m in MARKERS)


def apply(path: Path) -> bool:
    text = path.read_text()
    if not needs_gate(text):
        return False

    lines = text.splitlines(keepends=True)
    insert_at = 0
    for i, line in enumerate(lines):
        if line.startswith("import ") or line.startswith("const ") and "require" in line:
            insert_at = i + 1
        elif line.strip() and not line.startswith("import ") and not line.startswith("//"):
            break

    if IMPORT.strip() not in text:
        lines.insert(insert_at, IMPORT)

    # Insert hook after last import block / before first test.describe
    joined = "".join(lines)
    if "test.beforeEach(() => {\n  skipUnlessLiveStack();" in joined:
        return False

    m = re.search(r"^test\.describe", joined, re.MULTILINE)
    if not m:
        return False
    joined = joined[: m.start()] + HOOK + "\n" + joined[m.start() :]
    path.write_text(joined)
    return True


def main() -> None:
    changed = []
    for path in sorted(E2E.glob("*.spec.ts")):
        if apply(path):
            changed.append(path.name)
    print(f"Applied live-stack gate to {len(changed)} spec(s):")
    for name in changed:
        print(f"  + {name}")


if __name__ == "__main__":
    main()
