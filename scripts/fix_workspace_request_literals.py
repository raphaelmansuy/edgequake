#!/usr/bin/env python3
"""Insert `..Default::default()` into workspace request struct literals missing new fields."""

from __future__ import annotations

import re
from pathlib import Path

STRUCTS = ("CreateWorkspaceRequest", "UpdateWorkspaceRequest")
ROOT = Path(__file__).resolve().parents[1] / "edgequake"


def patch_content(content: str) -> tuple[str, int]:
    changed = 0
    for struct in STRUCTS:
        pattern = re.compile(rf"{struct}\s*\{{", re.MULTILINE)
        pos = 0
        out: list[str] = []
        for match in pattern.finditer(content):
            # Skip struct definitions (`pub struct CreateWorkspaceRequest {`).
            line_start = content.rfind("\n", 0, match.start()) + 1
            if "pub struct" in content[line_start:match.start()]:
                continue
            out.append(content[pos : match.start()])
            brace_start = match.end() - 1
            depth = 0
            i = brace_start
            while i < len(content):
                ch = content[i]
                if ch == "{":
                    depth += 1
                elif ch == "}":
                    depth -= 1
                    if depth == 0:
                        block = content[match.start() : i + 1]
                        if "..Default::default()" not in block:
                            close_idx = i
                            line_start = content.rfind("\n", match.start(), close_idx) + 1
                            indent = re.match(r"[ \t]*", content[line_start:close_idx]).group(0)
                            insertion = f"\n{indent}    ..Default::default()"
                            out.append(content[match.start() : close_idx])
                            out.append(insertion)
                            out.append(content[close_idx : i + 1])
                            changed += 1
                        else:
                            out.append(block)
                        pos = i + 1
                        break
                i += 1
            else:
                out.append(content[match.start() :])
                pos = len(content)
                break
        else:
            out.append(content[pos:])
            content = "".join(out)
            continue
        content = "".join(out)
    return content, changed


def main() -> None:
    total = 0
    skip_src_handlers = (
        "src/handlers/",
        "src/types/",
        "src/providers/resolver.rs",
    )
    for path in ROOT.rglob("*.rs"):
        rel = path.as_posix()
        if "/tests/" not in rel and not rel.endswith("/tests.rs"):
            if any(part in rel for part in skip_src_handlers):
                continue
            if "/src/" in rel and "tests" not in rel:
                continue
        original = path.read_text(encoding="utf-8")
        updated, n = patch_content(original)
        if n:
            path.write_text(updated, encoding="utf-8")
            total += n
            print(f"patched {n} block(s) in {path.relative_to(ROOT.parent)}")
    print(f"done: {total} struct literal(s) updated")


if __name__ == "__main__":
    main()
