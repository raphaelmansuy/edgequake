#!/usr/bin/env python3
"""
Compare edgequake-pdf output against pymupdf4llm gold standards.

This provides an objective comparison using a well-established
reference implementation (pymupdf4llm) rather than hand-crafted gold.

Usage:
    python compare_against_pymupdf.py --pdf-dir ./test-data/real_dataset
"""

import argparse
import difflib
import re
import subprocess
import sys
from pathlib import Path
from typing import Dict, List, Tuple


def normalize_text(text: str) -> str:
    """Normalize text for comparison.

    - Lowercase
    - Collapse whitespace
    - Remove extra newlines
    - Strip markdown formatting
    """
    # Collapse whitespace
    text = re.sub(r"\s+", " ", text)
    # Remove markdown bold/italic markers for comparison
    text = re.sub(r"\*+", "", text)
    text = re.sub(r"_+", "", text)
    # Lowercase
    text = text.lower()
    return text.strip()


def extract_sentences(text: str) -> List[str]:
    """Extract sentences from text for comparison."""
    # Normalize text first
    normalized = normalize_text(text)
    # Split on sentence boundaries
    sentences = re.split(r"[.!?]+", normalized)
    # Filter empty and very short
    return [s.strip() for s in sentences if len(s.strip()) > 10]


def compute_f1_score(pred_sentences: List[str], gold_sentences: List[str]) -> Dict:
    """Compute F1 score based on sentence matching."""
    pred_set = set(pred_sentences)
    gold_set = set(gold_sentences)

    # Exact matches
    matches = pred_set & gold_set

    # Also check for fuzzy matches (>90% similarity)
    fuzzy_matches = 0
    for p in pred_set - matches:
        for g in gold_set - matches:
            ratio = difflib.SequenceMatcher(None, p, g).ratio()
            if ratio > 0.9:
                fuzzy_matches += 1
                break

    true_positives = len(matches) + fuzzy_matches

    precision = true_positives / len(pred_set) if pred_set else 0
    recall = true_positives / len(gold_set) if gold_set else 0
    f1 = (
        2 * precision * recall / (precision + recall) if (precision + recall) > 0 else 0
    )

    return {
        "precision": precision,
        "recall": recall,
        "f1": f1,
        "pred_count": len(pred_sentences),
        "gold_count": len(gold_sentences),
        "exact_matches": len(matches),
        "fuzzy_matches": fuzzy_matches,
    }


def convert_pdf_with_edgequake(
    pdf_path: Path, output_path: Path, crate_dir: Path = None
) -> bool:
    """Convert PDF using edgequake-pdf."""
    if crate_dir is None:
        crate_dir = (
            Path(__file__).parent.parent / "edgequake" / "crates" / "edgequake-pdf"
        )

    try:
        result = subprocess.run(
            [
                "cargo",
                "run",
                "-r",
                "--bin",
                "edgequake-pdf",
                "--",
                "convert",
                "-i",
                str(pdf_path.absolute()),
                "-o",
                str(output_path.absolute()),
            ],
            capture_output=True,
            text=True,
            cwd=crate_dir,
        )
        if result.returncode != 0:
            print(f"  Cargo error: {result.stderr[:200]}")
        return result.returncode == 0
    except Exception as e:
        print(f"Error running edgequake-pdf: {e}")
        return False


def evaluate_pdf(pdf_path: Path, gold_path: Path) -> Dict:
    """Evaluate a single PDF against its pymupdf4llm gold."""
    import tempfile

    # Convert with edgequake-pdf
    with tempfile.NamedTemporaryFile(suffix=".md", delete=False) as tmp:
        tmp_path = Path(tmp.name)

    if not convert_pdf_with_edgequake(pdf_path, tmp_path):
        return {"error": "Conversion failed"}

    # Read outputs
    try:
        edgequake_output = tmp_path.read_text(encoding="utf-8")
        pymupdf_gold = gold_path.read_text(encoding="utf-8")
    except Exception as e:
        return {"error": f"Read error: {e}"}
    finally:
        tmp_path.unlink(missing_ok=True)

    # Extract sentences
    pred_sentences = extract_sentences(edgequake_output)
    gold_sentences = extract_sentences(pymupdf_gold)

    # Compute scores
    scores = compute_f1_score(pred_sentences, gold_sentences)
    scores["edgequake_chars"] = len(edgequake_output)
    scores["pymupdf_chars"] = len(pymupdf_gold)

    return scores


def main():
    parser = argparse.ArgumentParser(
        description="Compare edgequake-pdf against pymupdf4llm gold standards"
    )
    parser.add_argument(
        "--pdf-dir",
        type=Path,
        required=True,
        help="Directory containing PDFs and .pymupdf.gold.md files",
    )
    parser.add_argument("--only", type=str, help="Only evaluate this PDF (stem name)")

    args = parser.parse_args()

    # Find all PDFs with pymupdf gold files
    pdf_dir = args.pdf_dir
    pdf_files = sorted(pdf_dir.glob("*.pdf"))

    results = []
    for pdf_path in pdf_files:
        if args.only and args.only not in pdf_path.stem:
            continue

        gold_path = pdf_dir / f"{pdf_path.stem}.pymupdf.gold.md"
        if not gold_path.exists():
            print(f"⚠ No pymupdf gold for: {pdf_path.name}")
            continue

        print(f"Evaluating: {pdf_path.stem}")
        scores = evaluate_pdf(pdf_path, gold_path)

        if "error" in scores:
            print(f"  ✗ {scores['error']}")
        else:
            print(
                f"  F1: {scores['f1']:.3f} (P: {scores['precision']:.3f}, R: {scores['recall']:.3f})"
            )
            print(
                f"  Sentences: {scores['pred_count']} pred / {scores['gold_count']} gold"
            )
            results.append((pdf_path.stem, scores))

    # Summary
    if results:
        print("\n" + "=" * 60)
        print("SUMMARY vs pymupdf4llm gold standards")
        print("=" * 60)

        total_f1 = sum(r[1]["f1"] for r in results) / len(results)
        total_p = sum(r[1]["precision"] for r in results) / len(results)
        total_r = sum(r[1]["recall"] for r in results) / len(results)

        print(f"\nAverage F1: {total_f1:.3f}")
        print(f"Average Precision: {total_p:.3f}")
        print(f"Average Recall: {total_r:.3f}")

        print("\nPer-document scores:")
        for name, scores in sorted(results, key=lambda x: x[1]["f1"]):
            print(f"  {name}: F1={scores['f1']:.3f}")


if __name__ == "__main__":
    main()
