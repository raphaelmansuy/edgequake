#!/usr/bin/env python3
"""Analyze author line span positions."""

import fitz

pdf_path = "crates/edgequake-pdf/test-data/real_dataset/01_2512.25075v1.pdf"
doc = fitz.open(pdf_path)
page = doc[0]

# Get spans on the author line (y ~158-170)
text_dict = page.get_text("dict")
print("Looking for author block...")
for block in text_dict.get("blocks", []):
    if block.get("type") != 0:
        continue
    y0 = block["bbox"][1]
    if 155 < y0 < 175:
        print(f"Block at y={y0:.0f}:")
        for line in block.get("lines", []):
            for span in line.get("spans", []):
                orig = span.get("origin", (0, 0))
                t = span.get("text", "")
                print(f"  x={orig[0]:.1f}: {repr(t)}")
