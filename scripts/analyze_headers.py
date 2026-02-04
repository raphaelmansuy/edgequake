#!/usr/bin/env python3
"""Analyze font sizes for section headers in v2_2512."""
import fitz

doc = fitz.open(
    "edgequake/crates/edgequake-pdf/test-data/real_dataset/v2_2512.25072v1.pdf"
)

print("=== Font Size Analysis for Section Headers ===\n")

# First find body text size (most common)
size_counts = {}
for page in doc:
    blocks = page.get_text("dict")["blocks"]
    for block in blocks:
        if "lines" in block:
            for line in block["lines"]:
                for span in line["spans"]:
                    size = round(span["size"])
                    text_len = len(span["text"])
                    size_counts[size] = size_counts.get(size, 0) + text_len

body_size = max(size_counts, key=size_counts.get)
print(f"Body font size: {body_size}pt\n")

# Now find headers
print("=== Section Headers Found ===")
for page_num in range(len(doc)):
    page = doc[page_num]
    blocks = page.get_text("dict")["blocks"]

    for block in blocks:
        if "lines" in block:
            for line in block["lines"]:
                text = "".join(span["text"] for span in line["spans"]).strip()
                sizes = [span["size"] for span in line["spans"]]

                if not text or not sizes:
                    continue

                max_size = max(sizes)
                ratio = max_size / body_size

                # Look for Roman numeral section headers
                import re

                if re.match(r"^[IVX]+\.\s", text):  # Roman numeral
                    print(f"Page {page_num}: {text[:60]}")
                    print(f"  Size: {max_size:.1f}pt, Ratio: {ratio:.2f}x body")

                # Look for letter subsections
                elif re.match(r"^[A-Z]\.\s", text):  # Letter prefix
                    print(f"Page {page_num}: {text[:60]}")
                    print(f"  Size: {max_size:.1f}pt, Ratio: {ratio:.2f}x body")

doc.close()
