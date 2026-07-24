#!/usr/bin/env python3
"""SPEC-086 LAW-27 — density gate for MD↔PDF golden pairs (entities / 1k chars).

Usage (CLI numbers):
  python3 scripts/ingestion_density_gate.py \\
    --md-chars 12000 --md-entities 48 \\
    --pdf-chars 180000 --pdf-entities 1100 \\
    --min-ratio 0.25

Usage (checked-in golden pair):
  python3 scripts/ingestion_density_gate.py \\
    --fixture specs/086-improve-ingestion-ux/fixtures/density-golden-pair-v1.json

Fails only on density cliff (md_density / pdf_density < min_ratio).
Absolute entity equality is NOT required.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


def density(entities: float, chars: float) -> float:
    return entities / max(chars / 1000.0, 1e-9)


def load_fixture(path: Path) -> dict:
    data = json.loads(path.read_text(encoding="utf-8"))
    md = data["md"]
    pdf = data["pdf"]
    thresholds = data.get("thresholds") or {}
    return {
        "md_chars": float(md["chars"]),
        "md_entities": float(md["entities"]),
        "pdf_chars": float(pdf["chars"]),
        "pdf_entities": float(pdf["entities"]),
        "min_ratio": float(thresholds.get("min_ratio", 0.25)),
        "md_section_pct": (
            float(md["section_pct"]) if md.get("section_pct") is not None else None
        ),
        "min_section_pct": float(thresholds.get("min_section_pct", 0.0)),
        "fixture_id": data.get("id", path.name),
    }


def main() -> int:
    p = argparse.ArgumentParser(description="SPEC-086 ingestion density gate")
    p.add_argument(
        "--fixture",
        type=Path,
        default=None,
        help="Path to density-golden-pair JSON (overrides numeric flags)",
    )
    p.add_argument("--md-chars", type=float, default=None)
    p.add_argument("--md-entities", type=float, default=None)
    p.add_argument("--pdf-chars", type=float, default=None)
    p.add_argument("--pdf-entities", type=float, default=None)
    p.add_argument(
        "--min-ratio",
        type=float,
        default=0.25,
        help="Minimum md_density/pdf_density (default 0.25)",
    )
    p.add_argument(
        "--md-section-pct",
        type=float,
        default=None,
        help="Optional %% of MD chunks with section breadcrumbs",
    )
    p.add_argument(
        "--min-section-pct",
        type=float,
        default=0.0,
        help="Floor for section coverage when --md-section-pct is set",
    )
    args = p.parse_args()

    if args.fixture is not None:
        loaded = load_fixture(args.fixture)
        md_chars = loaded["md_chars"]
        md_entities = loaded["md_entities"]
        pdf_chars = loaded["pdf_chars"]
        pdf_entities = loaded["pdf_entities"]
        min_ratio = loaded["min_ratio"]
        md_section_pct = loaded["md_section_pct"]
        min_section_pct = loaded["min_section_pct"]
        print(f"fixture={loaded['fixture_id']}")
    else:
        missing = [
            n
            for n, v in [
                ("--md-chars", args.md_chars),
                ("--md-entities", args.md_entities),
                ("--pdf-chars", args.pdf_chars),
                ("--pdf-entities", args.pdf_entities),
            ]
            if v is None
        ]
        if missing:
            print(
                "ERROR: provide --fixture or all of "
                + ", ".join(missing),
                file=sys.stderr,
            )
            return 2
        md_chars = args.md_chars
        md_entities = args.md_entities
        pdf_chars = args.pdf_chars
        pdf_entities = args.pdf_entities
        min_ratio = args.min_ratio
        md_section_pct = args.md_section_pct
        min_section_pct = args.min_section_pct

    md_d = density(md_entities, md_chars)
    pdf_d = density(pdf_entities, pdf_chars)
    ratio = md_d / max(pdf_d, 1e-9)

    print(f"md_density={md_d:.4f} entities/1k chars")
    print(f"pdf_density={pdf_d:.4f} entities/1k chars")
    print(f"ratio={ratio:.4f} (floor={min_ratio})")

    if md_section_pct is not None:
        print(f"md_section_pct={md_section_pct:.1f} (floor={min_section_pct})")
        if md_section_pct < min_section_pct:
            print("FAIL: section breadcrumb coverage below floor", file=sys.stderr)
            return 1

    if ratio < min_ratio:
        print(
            f"FAIL: density cliff ratio {ratio:.4f} < {min_ratio}",
            file=sys.stderr,
        )
        return 1

    print("PASS: density within band")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
