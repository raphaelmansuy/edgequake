#!/usr/bin/env python3
"""Validate the SPEC-181 pack.

Checks, in order:
  1. every relative markdown link resolves inside the pack (or exists outside it)
  2. LAW-181-1..16 exist in 01 and are referenced in 15
  3. F-181-01..12 exist in 02/04 and are referenced in 15
  4. EC-181-01..48 exist in 12 and each has at least one gate in 14
  5. every gate id used in 12 and 15 exists in 14
  6. J-01..J-12 exist in 06
  7. US-181-01..10 exist in 03
  8. every lenses/ file referenced from README exists

Exit code 0 = pass, 1 = errors. Usage:
    python3 specs/181-admin/scripts/validate-cross-ref.py [pack-dir]
"""

from __future__ import annotations

import re
import sys
from pathlib import Path
from urllib.parse import unquote

DEFAULT_ROOT = Path(__file__).resolve().parents[1]

ERRORS: list[str] = []
WARNINGS: list[str] = []

LINK_RE = re.compile(r"\[([^\]]+)\]\(([^)]+)\)")
LAW_RE = re.compile(r"LAW-181-(\d+)")
FINDING_RE = re.compile(r"F-181-(\d+)")
EC_RE = re.compile(r"EC-181-(\d+)")
JOURNEY_RE = re.compile(r"J-(\d{2})")
STORY_RE = re.compile(r"US-181-(\d+)")
# Gate ids: R-…, E-…, A-…, S-… — backticked or plain, with hyphens/colons inside.
GATE_RE = re.compile(r"`([REAS]-[a-z0-9][a-z0-9:_-]*)`")

EXPECTED_LAWS = set(range(1, 17))
EXPECTED_FINDINGS = set(range(1, 13))
EXPECTED_ECS = set(range(1, 49))
EXPECTED_JOURNEYS = set(range(1, 13))
EXPECTED_STORIES = set(range(1, 11))


def read(path: Path) -> str:
    if not path.exists():
        ERRORS.append(f"Missing file: {path.name}")
        return ""
    return path.read_text(encoding="utf-8")


def resolve_link(src: Path, target: str) -> Path | None:
    if target.startswith(("http://", "https://", "mailto:", "#")):
        return None
    path_part = target.split("#", 1)[0]
    if not path_part:
        return None
    return (src.parent / unquote(path_part)).resolve()


def check_links(root: Path) -> None:
    for md in sorted(root.rglob("*.md")):
        text = read(md)
        for _label, target in LINK_RE.findall(text):
            dest = resolve_link(md, target)
            if dest is None:
                continue
            if dest.exists():
                continue
            try:
                dest.relative_to(root)
                in_pack = True
            except ValueError:
                in_pack = False
            if in_pack:
                ERRORS.append(f"Broken in-pack link in {md.relative_to(root)}: ({target})")
            elif target.startswith(("../", "../../")):
                WARNINGS.append(
                    f"External/missing link in {md.relative_to(root)}: ({target})"
                )


def ids(pattern: re.Pattern[str], text: str) -> set[int]:
    return {int(n) for n in pattern.findall(text)}


def row_ids(pattern: re.Pattern[str], text: str) -> set[int]:
    """Ids that appear in a markdown table row (a line starting with `|`).

    Excluding prose and headings matters: a heading such as
    `# 12 — Edge cases (EC-181-01 … EC-181-48)` would otherwise satisfy the
    "EC-181-01 exists" check even after its row was deleted.
    """
    found: set[int] = set()
    for line in text.splitlines():
        if line.lstrip().startswith("|"):
            found |= ids(pattern, line)
    return found


def check_laws(root: Path) -> None:
    laws_doc = read(root / "01-first-principles.md")
    found = ids(LAW_RE, laws_doc)
    if missing := EXPECTED_LAWS - found:
        ERRORS.append(f"01-first-principles missing LAW-181: {sorted(missing)}")
    if extra := found - EXPECTED_LAWS:
        WARNINGS.append(f"01-first-principles has undeclared LAW-181: {sorted(extra)}")

    xref = read(root / "15-cross-ref.md")
    if missing := EXPECTED_LAWS - ids(LAW_RE, xref):
        ERRORS.append(f"15-cross-ref missing laws: {sorted(missing)}")


def check_findings(root: Path) -> None:
    surfaces = read(root / "02-surfaces.md")
    findings_doc = read(root / "04-findings.md")
    found = row_ids(FINDING_RE, surfaces + findings_doc)
    if missing := EXPECTED_FINDINGS - found:
        ERRORS.append(f"02/04 missing F-181: {sorted(missing)}")

    xref = read(root / "15-cross-ref.md")
    if missing := EXPECTED_FINDINGS - ids(FINDING_RE, xref):
        ERRORS.append(f"15-cross-ref missing findings: {sorted(missing)}")


def check_edge_cases_and_gates(root: Path) -> None:
    ec_doc = read(root / "12-edge-cases.md")
    matrix = read(root / "14-e2e-test-matrix.md")
    xref = read(root / "15-cross-ref.md")

    ecs = row_ids(EC_RE, ec_doc)
    if missing := EXPECTED_ECS - ecs:
        ERRORS.append(f"12-edge-cases missing EC-181: {sorted(missing)}")
    if extra := ecs - EXPECTED_ECS:
        WARNINGS.append(f"12-edge-cases has undeclared EC-181: {sorted(extra)}")

    gates_defined = set(GATE_RE.findall(matrix)) | set(
        re.findall(r"\|\s*`?([REAS]-[a-z0-9][a-z0-9:_-]*)`?\s*\|", matrix)
    )
    gates_referenced = set(GATE_RE.findall(ec_doc)) | set(GATE_RE.findall(xref))
    if unknown := gates_referenced - gates_defined:
        ERRORS.append(f"Gates referenced but not defined in 14: {sorted(unknown)}")

    # Every EC row must carry at least one gate in the same table row.
    for line in ec_doc.splitlines():
        if not EC_RE.search(line):
            continue
        if not line.strip().startswith("|"):
            continue
        if not GATE_RE.search(line):
            ERRORS.append(f"EC row has no gate: {line.strip()[:80]}…")

    # Coverage table in 14 must list every EC.
    coverage = ids(EC_RE, matrix)
    if missing := EXPECTED_ECS - coverage:
        ERRORS.append(f"14 coverage matrix missing EC: {sorted(missing)}")


def check_journeys(root: Path) -> None:
    doc = read(root / "06-user-journeys.md")
    found = ids(JOURNEY_RE, doc)
    if missing := EXPECTED_JOURNEYS - found:
        ERRORS.append(f"06-user-journeys missing J: {sorted(missing)}")
    xref = read(root / "15-cross-ref.md")
    if missing := EXPECTED_JOURNEYS - ids(JOURNEY_RE, xref):
        ERRORS.append(f"15-cross-ref missing journeys: {sorted(missing)}")


def check_stories(root: Path) -> None:
    doc = read(root / "03-product-spec.md")
    found = ids(STORY_RE, doc)
    if missing := EXPECTED_STORIES - found:
        ERRORS.append(f"03-product-spec missing US-181: {sorted(missing)}")


def check_lenses(root: Path) -> None:
    readme = read(root / "README.md")
    for match in re.findall(r"\((lenses/[^)]+)\)", readme):
        if not (root / match).exists():
            ERRORS.append(f"README links a missing lens: {match}")
    lens_dir = root / "lenses"
    if lens_dir.is_dir():
        for lens in sorted(lens_dir.glob("LENS-*.md")):
            text = read(lens)
            if "Parent:" not in text:
                ERRORS.append(f"{lens.name} has no Parent line")


def main() -> int:
    root = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else DEFAULT_ROOT

    if not (root / "README.md").exists():
        print(f"Not a spec pack: {root}", file=sys.stderr)
        return 1

    check_links(root)
    check_laws(root)
    check_findings(root)
    check_edge_cases_and_gates(root)
    check_journeys(root)
    check_stories(root)
    check_lenses(root)

    for w in WARNINGS:
        print(f"WARN  {w}")
    for e in ERRORS:
        print(f"ERROR {e}")

    if ERRORS:
        print(f"\nFAIL — {len(ERRORS)} error(s), {len(WARNINGS)} warning(s)")
        return 1

    print(
        "PASS — 16 laws, 12 findings, 48 edge cases, 12 journeys, 10 stories, "
        f"{len(set(GATE_RE.findall(read(root / '14-e2e-test-matrix.md'))))} gates, "
        f"{len(WARNINGS)} warning(s)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
