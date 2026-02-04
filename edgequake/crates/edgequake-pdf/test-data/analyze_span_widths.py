#!/usr/bin/env python3
"""Analyze span widths in author line."""

import fitz

pdf_path = "crates/edgequake-pdf/test-data/real_dataset/01_2512.25075v1.pdf"
doc = fitz.open(pdf_path)
page = doc[0]

text_dict = page.get_text("dict")
print("Author line spans with widths:")
for block in text_dict.get("blocks", []):
    if block.get("type") != 0:
        continue
    y0 = block["bbox"][1]
    if 155 < y0 < 175:
        for line in block.get("lines", []):
            for span in line.get("spans", []):
                bbox = span.get("bbox", [0, 0, 0, 0])
                orig = span.get("origin", (0, 0))
                t = span.get("text", "")
                width = bbox[2] - bbox[0]
                font = span.get("font", "unknown")
                size = span.get("size", 0)
                char_count = len(t)
                avg_char_width = width / char_count if char_count > 0 else 0
                ratio = avg_char_width / size if size > 0 else 0
                print(
                    f"  x={orig[0]:.1f} w={width:.1f} ({char_count} chars, size={size:.1f}, ratio={ratio:.2f}): {repr(t)}"
                )
