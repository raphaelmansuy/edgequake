#!/usr/bin/env python3
"""Test all PDFs in test-data directory and compare with gold standards."""

import json
import os
import subprocess
from difflib import SequenceMatcher
from pathlib import Path

# Configuration
PDF_DIR = Path("edgequake/crates/edgequake-pdf/test-data")
BINARY = Path("edgequake/target/release/edgequake-pdf")
OUTPUT_DIR = Path("/tmp/pdf_test_results")

OUTPUT_DIR.mkdir(exist_ok=True)


def extract_pdf(pdf_path: Path, output_path: Path) -> bool:
    """Extract PDF to markdown."""
    try:
        result = subprocess.run(
            [str(BINARY), "convert", "-i", str(pdf_path), "-o", str(output_path)],
            capture_output=True,
            text=True,
            timeout=30,
        )
        return result.returncode == 0
    except Exception as e:
        print(f"  ❌ Error: {e}")
        return False


def similarity_score(text1: str, text2: str) -> float:
    """Calculate similarity between two texts."""
    return SequenceMatcher(None, text1, text2).ratio() * 100


def test_pdf(pdf_path: Path) -> dict:
    """Test a single PDF."""
    pdf_name = pdf_path.stem
    print(f"\n📄 Testing: {pdf_name}")

    # Find gold standard
    gold_path = pdf_path.parent / f"{pdf_name}.gold.md"
    if not gold_path.exists():
        gold_path = pdf_path.parent / f"{pdf_name}.md"

    if not gold_path.exists():
        print(f"  ⚠️  No gold standard found")
        return {"name": pdf_name, "status": "no_gold", "score": 0}

    # Extract PDF
    output_path = OUTPUT_DIR / f"{pdf_name}_extracted.md"
    print(f"  🔄 Extracting...")
    if not extract_pdf(pdf_path, output_path):
        print(f"  ❌ Extraction failed")
        return {"name": pdf_name, "status": "failed", "score": 0}

    # Read results
    with open(output_path, "r", encoding="utf-8") as f:
        extracted = f.read()

    with open(gold_path, "r", encoding="utf-8") as f:
        gold = f.read()

    # Calculate metrics
    score = similarity_score(extracted, gold)
    gold_len = len(gold)
    extracted_len = len(extracted)

    print(f"  ✅ Score: {score:.1f}%")
    print(f"  📊 Gold: {gold_len} chars, Extracted: {extracted_len} chars")

    return {
        "name": pdf_name,
        "status": "success",
        "score": score,
        "gold_length": gold_len,
        "extracted_length": extracted_len,
        "ratio": extracted_len / gold_len if gold_len > 0 else 0,
    }


def main():
    """Run tests on all PDFs."""
    print("=" * 60)
    print("PDF Extraction Test Suite")
    print("=" * 60)

    # Find all numbered PDFs
    pdfs = sorted(PDF_DIR.glob("[0-9][0-9][0-9]_*.pdf"))

    print(f"\n Found {len(pdfs)} PDFs to test")

    results = []
    for pdf in pdfs:
        result = test_pdf(pdf)
        results.append(result)

    # Summary
    print("\n" + "=" * 60)
    print("SUMMARY")
    print("=" * 60)

    successful = [r for r in results if r["status"] == "success"]
    failed = [r for r in results if r["status"] == "failed"]
    no_gold = [r for r in results if r["status"] == "no_gold"]

    print(f"\n✅ Successful: {len(successful)}/{len(results)}")
    print(f"❌ Failed: {len(failed)}/{len(results)}")
    print(f"⚠️  No Gold: {len(no_gold)}/{len(results)}")

    if successful:
        avg_score = sum(r["score"] for r in successful) / len(successful)
        print(f"\n📊 Average similarity score: {avg_score:.1f}%")

        # Show best and worst
        best = max(successful, key=lambda x: x["score"])
        worst = min(successful, key=lambda x: x["score"])

        print(f"\n🏆 Best: {best['name']} ({best['score']:.1f}%)")
        print(f"📉 Worst: {worst['name']} ({worst['score']:.1f}%)")

    # Save results
    report_path = OUTPUT_DIR / "test_report.json"
    with open(report_path, "w") as f:
        json.dump(
            {
                "total": len(results),
                "successful": len(successful),
                "failed": len(failed),
                "no_gold": len(no_gold),
                "results": results,
            },
            f,
            indent=2,
        )

    print(f"\n📝 Full report: {report_path}")
    print("\n" + "=" * 60)


if __name__ == "__main__":
    main()
