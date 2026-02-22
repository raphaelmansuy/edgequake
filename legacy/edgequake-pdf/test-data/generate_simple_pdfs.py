#!/usr/bin/env python3
"""
Simple PDF generator that converts markdown gold files into plain PDFs for evaluation.
This avoids a hard dependency on pandoc and ensures PDFs exist for evaluation runs.
It renders markdown as plain paragraphs (no sophisticated styling) using reportlab.

Usage: python3 generate_simple_pdfs.py
"""
import os
import sys
from pathlib import Path

try:
    from reportlab.lib.pagesizes import letter
    from reportlab.lib.styles import getSampleStyleSheet
    from reportlab.platypus import Paragraph, SimpleDocTemplate, Spacer
except Exception as e:
    print("Error: reportlab is required for this script.")
    print("Install with: pip install reportlab")
    sys.exit(1)

ROOT = Path(__file__).parent
GOLD = ROOT / "gold"
PDFS = ROOT / "pdfs"

PDFS.mkdir(parents=True, exist_ok=True)

for category in sorted(GOLD.iterdir()):
    if not category.is_dir():
        continue
    out_cat = PDFS / category.name
    out_cat.mkdir(parents=True, exist_ok=True)

    md_files = sorted(category.glob("*.md"))
    for md in md_files:
        out_pdf = out_cat / md.with_suffix(".pdf").name
        print(f"Generating {out_pdf}")
        try:
            text = md.read_text(encoding="utf-8")
            doc = SimpleDocTemplate(str(out_pdf), pagesize=letter)
            styles = getSampleStyleSheet()
            story = []
            for line in text.splitlines():
                if not line.strip():
                    story.append(Spacer(1, 6))
                else:
                    story.append(
                        Paragraph(
                            line.replace("&", "&amp;").replace("<", "&lt;"),
                            styles["Normal"],
                        )
                    )
            doc.build(story)
        except Exception as e:
            print(f"Failed to generate PDF for {md}: {e}")

print("Done generating simple PDFs.")
