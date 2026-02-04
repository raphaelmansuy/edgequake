#!/usr/bin/env python3
"""
Evaluate the pymupdf4llm-inspired pipeline output against gold standards.

Usage:
  python3 scripts/eval_pymupdf_pipeline.py

This script runs the Rust pipeline and compares output to *.pymupdf.gold.md files.
"""

import difflib
import os
import subprocess
import sys
from pathlib import Path


def run_pipeline(pdf_path: Path, lib_path: Path) -> str | None:
    """Run the Rust pipeline and capture output."""
    env = os.environ.copy()
    env["PDFIUM_DYNAMIC_LIB_PATH"] = str(lib_path)

    try:
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
            cwd=Path(__file__).parent.parent / "edgequake",
            capture_output=True,
            text=True,
            timeout=120,
            env=env,
        )

        if result.returncode != 0:
            print(f"  ERROR: {result.stderr[:200]}")
            return None

        return result.stdout

    except subprocess.TimeoutExpired:
        print(f"  TIMEOUT")
        return None
    except Exception as e:
        print(f"  EXCEPTION: {e}")
        return None


def calculate_f1(gold_text: str, extracted_text: str) -> dict:
    """Calculate F1 score based on word overlap."""

    # Normalize texts
    def normalize(text: str) -> set[str]:
        words = text.lower().split()
        # Remove markdown markers
        words = [w.strip("*_`#[]()") for w in words]
        words = [w for w in words if w and len(w) > 1]
        return set(words)

    gold_words = normalize(gold_text)
    extracted_words = normalize(extracted_text)

    if not gold_words:
        return {"precision": 0, "recall": 0, "f1": 0}

    true_positive = len(gold_words & extracted_words)
    precision = true_positive / len(extracted_words) if extracted_words else 0
    recall = true_positive / len(gold_words) if gold_words else 0

    if precision + recall == 0:
        f1 = 0
    else:
        f1 = 2 * precision * recall / (precision + recall)

    return {
        "precision": precision,
        "recall": recall,
        "f1": f1,
        "gold_words": len(gold_words),
        "extracted_words": len(extracted_words),
        "common_words": true_positive,
    }


def main():
    # Paths
    script_dir = Path(__file__).parent.parent
    test_data = script_dir / "edgequake/crates/edgequake-pdf/test-data/real_dataset"
    lib_path = script_dir / "edgequake/crates/edgequake-pdf/lib/lib/libpdfium.dylib"

    if not lib_path.exists():
        print(f"ERROR: libpdfium.dylib not found at {lib_path}")
        sys.exit(1)

    # Find gold standard pairs
    pdf_files = list(test_data.glob("*.pdf"))
    results = []

    print(f"\n{'='*60}")
    print("PyMuPDF4LLM-Inspired Pipeline Evaluation")
    print(f"{'='*60}\n")

    for pdf_path in sorted(pdf_files):  # Test all files
        stem = pdf_path.stem
        gold_path = test_data / f"{stem}.pymupdf.gold.md"

        if not gold_path.exists():
            print(f"⏭ {stem}: No pymupdf gold standard")
            continue

        print(f"📄 {stem}...")

        # Read gold
        gold_text = gold_path.read_text()

        # Run pipeline
        extracted = run_pipeline(pdf_path, lib_path)
        if not extracted:
            print(f"  ❌ Failed to extract")
            continue

        # Calculate F1
        scores = calculate_f1(gold_text, extracted)
        results.append({"file": stem, **scores})

        print(
            f"  ✓ F1={scores['f1']:.3f} (P={scores['precision']:.3f}, R={scores['recall']:.3f})"
        )

    # Summary
    if results:
        avg_f1 = sum(r["f1"] for r in results) / len(results)
        print(f"\n{'='*60}")
        print(f"Average F1: {avg_f1:.3f}")
        print(f"Files evaluated: {len(results)}")
        print(f"{'='*60}\n")
    else:
        print("\nNo files evaluated.")


if __name__ == "__main__":
    main()
