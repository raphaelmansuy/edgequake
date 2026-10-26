#!/usr/bin/env python3
"""Validate SPEC-180 documentation; standard library only, no network writes."""

from collections import Counter
import json
from pathlib import Path
import re
import sys
from urllib.parse import unquote, urlsplit


ROOT = Path(__file__).resolve().parents[1]
LINK = re.compile(r"\[[^\]]*\]\(([^)]+)\)")
FENCE = re.compile(r"^```([^\n]*)\n(.*?)^```[ \t]*$", re.M | re.S)
DEFINITION = re.compile(r"^#{2,3} ((?:R|LAW|EC|T)-180-\d{2}|WP-\d{2}|W-180-\d{2})\s*$", re.M)
REFERENCE = re.compile(r"\b(?:R|LAW|EC|T)-180-\d{2}\b")


def slug(text):
    text = re.sub(r"[^\w -]", "", text.lower())
    return text.replace(" ", "-")


def heading_anchors(text):
    counts = Counter()
    anchors = set()
    plain = FENCE.sub("", text)
    for match in re.finditer(r"^#{1,6}\s+(.+?)\s*$", plain, re.M):
        base = slug(match.group(1))
        duplicate = counts[base]
        counts[base] += 1
        anchors.add(base if not duplicate else f"{base}-{duplicate}")
    return anchors


def validate():
    files = sorted(ROOT.rglob("*.md"))
    texts = {path: path.read_text() for path in files}
    errors = []
    definitions = Counter()
    links = diagrams = grids = examples = 0
    for path, text in texts.items():
        relative = path.relative_to(ROOT)
        definitions.update(DEFINITION.findall(text))
        if sum(line.startswith("```") for line in text.splitlines()) % 2:
            errors.append(f"{relative}: unmatched code fence")
        for match in LINK.finditer(text):
            destination = match.group(1).strip("<>")
            parsed = urlsplit(destination)
            if parsed.scheme or destination.startswith("//"):
                continue
            links += 1
            target = (path.parent / unquote(parsed.path)).resolve() if parsed.path else path
            if not target.exists():
                errors.append(f"{relative}: missing link target {destination}")
            elif parsed.fragment and target.suffix == ".md":
                anchors = heading_anchors(target.read_text())
                if unquote(parsed.fragment) not in anchors:
                    errors.append(f"{relative}: missing anchor {destination}")
        for match in FENCE.finditer(text):
            language, body = match.groups()
            if language == "mermaid":
                diagrams += 1
            elif language == "json":
                examples += 1
                try:
                    json.loads(body)
                except json.JSONDecodeError as error:
                    errors.append(f"{relative}: invalid JSON example {error}")
            elif language == "ascii":
                if not body.isascii():
                    errors.append(f"{relative}: non-ASCII character in wireframe")
                lines = body.splitlines()
                widths = [len(line) for line in lines if re.match(r"^\+-+\+$", line) or line.startswith("|")]
                if widths:
                    grids += 1
                    if len(set(widths)) != 1:
                        errors.append(f"{relative}: unequal ASCII grid widths {sorted(set(widths))}")
                if any("\t" in line or line.rstrip() != line for line in lines):
                    errors.append(f"{relative}: tabs/trailing whitespace in ASCII")
        if re.search(r"\b(?:TBD|TODO|FIXME)\b", text):
            errors.append(f"{relative}: unresolved placeholder")
    for identifier, count in definitions.items():
        if count != 1:
            errors.append(f"{identifier}: {count} canonical definitions")
    all_text = "\n".join(texts.values())
    for identifier in set(REFERENCE.findall(all_text)):
        if identifier not in definitions:
            errors.append(f"{identifier}: referenced but undefined")
    expected = {"R": 24, "LAW": 10, "EC": 40, "T": 40, "WP": 8, "W": 9}
    for prefix, count in expected.items():
        for number in range(1, count + 1):
            identifier = f"{prefix}-180-{number:02}" if prefix != "WP" else f"WP-{number:02}"
            if definitions[identifier] != 1:
                errors.append(f"{identifier}: missing or duplicate expected definition")
    trace = texts[ROOT / "15-traceability.md"]
    for number in range(1, 25):
        requirement = f"R-180-{number:02}"
        rows = [line for line in trace.splitlines() if line.startswith(f"| [{requirement}]")]
        if not any("LAW-180-" in row and "EC-180-" in row and "T-180-" in row and "WP-" in row for row in rows):
            errors.append(f"{requirement}: incomplete traceability row")
    for number in range(1, 41):
        if f"T-180-{number:02}" not in trace or f"EC-180-{number:02}" not in trace:
            errors.append(f"case {number}: missing from traceability matrix")
    result = dict(markdown_files=len(files), local_links=links, canonical_ids=len(definitions),
                  mermaid_diagrams=diagrams, ascii_grids=grids, json_examples=examples,
                  errors=errors)
    print(json.dumps(result, indent=2))
    return bool(errors)


if __name__ == "__main__":
    sys.exit(validate())
