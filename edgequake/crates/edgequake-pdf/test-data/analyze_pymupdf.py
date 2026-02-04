#!/usr/bin/env python3
"""Analyze PyMuPDF text extraction for author names."""

import sys

import fitz

pdf_path = (
    sys.argv[1]
    if len(sys.argv) > 1
    else "crates/edgequake-pdf/test-data/real_dataset/01_2512.25075v1.pdf"
)

doc = fitz.open(pdf_path)
page = doc[0]
text_dict = page.get_text("dict")

for i, block in enumerate(text_dict.get("blocks", [])[:6]):
    if block.get("type") == 0:  # text block
        x0, y0, x1, y1 = block["bbox"]
        print(f"Block {i}: ({x0:.0f}, {y0:.0f}) to ({x1:.0f}, {y1:.0f})")
        for line in block.get("lines", []):
            for span in line.get("spans", []):
                t = span.get("text", "")
                if t.strip():
                    origin = span.get("origin", (0, 0))
                    print(f"  {origin}: {t!r}")
        print()
