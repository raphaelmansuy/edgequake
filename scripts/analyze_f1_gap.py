#!/usr/bin/env python3
"""Analyze F1 gap for a specific PDF file."""

import os
import subprocess
import sys
from pathlib import Path


def main():
    pdf_name = sys.argv[1] if len(sys.argv) > 1 else "AlphaEvolve"

    pdf_dir = Path("edgequake/crates/edgequake-pdf/test-data/real_dataset")
    pdf_path = pdf_dir / f"{pdf_name}.pdf"
    gold_path = pdf_dir / f"{pdf_name}.md"

    if not pdf_path.exists():
        print(f"PDF not found: {pdf_path}")
        return
    if not gold_path.exists():
        print(f"Gold not found: {gold_path}")
        return

    # Get extracted output
    env = os.environ.copy()
    env["PDFIUM_DYNAMIC_LIB_PATH"] = str(pdf_dir.parent / "lib/lib")

    result = subprocess.run(
        [
            "cargo",
            "run",
            "--features",
            "pdfium",
            "-p",
            "edgequake-pdf",
            "--example",
            "convert_pdf_full",
            "--",
            str(pdf_path),
        ],
        cwd="edgequake",
        capture_output=True,
        text=True,
        timeout=120,
        env=env,
    )

    extracted = result.stdout
    gold = gold_path.read_text()

    # Normalize
    def normalize(text):
        words = text.lower().split()
        words = [w.strip("*_`#[]()") for w in words]
        return [w for w in words if w and len(w) > 1]

    extracted_words = normalize(extracted)
    gold_words = normalize(gold)

    extracted_set = set(extracted_words)
    gold_set = set(gold_words)

    extra = sorted(extracted_set - gold_set)
    missing = sorted(gold_set - extracted_set)

    print(f"=== Analysis for {pdf_name} ===")
    print(f"Extracted words: {len(extracted_words)} ({len(extracted_set)} unique)")
    print(f"Gold words: {len(gold_words)} ({len(gold_set)} unique)")
    print(f"Extra words: {len(extra)}")
    print(f"Missing words: {len(missing)}")
    print()
    print("Sample EXTRA (in extracted, not in gold):")
    print("  " + ", ".join(extra[:30]))
    print()
    print("Sample MISSING (in gold, not in extracted):")
    print("  " + ", ".join(missing[:30]))


if __name__ == "__main__":
    main()
