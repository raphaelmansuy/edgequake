#!/usr/bin/env python3
"""Test real dataset PDFs."""

import json
import os
import subprocess
from difflib import SequenceMatcher
from pathlib import Path

# Configuration
REAL_DATASET_DIR = Path("edgequake/crates/edgequake-pdf/test-data/real_dataset")
BINARY = Path("edgequake/target/release/edgequake-pdf")
OUTPUT_DIR = Path("/tmp/pdf_real_dataset_results")

OUTPUT_DIR.mkdir(exist_ok=True)


def extract_pdf(pdf_path: Path, output_path: Path) -> bool:
    """Extract PDF to markdown."""
    try:
        result = subprocess.run(
            [str(BINARY), "convert", "-i", str(pdf_path), "-o", str(output_path)],
            capture_output=True,
            text=True,
            timeout=60,  # Longer timeout for real PDFs
        )
        return result.returncode == 0
    except Exception as e:
        print(f"  ❌ Error: {e}")
        return False


def similarity_score(text1: str, text2: str) -> float:
    """Calculate similarity between two texts."""
    return SequenceMatcher(None, text1, text2).ratio() * 100


def get_pdf_info(pdf_path: Path) -> dict:
    """Get PDF information."""
    try:
        result = subprocess.run(
            [str(BINARY), "info", "-i", str(pdf_path)],
            capture_output=True,
            text=True,
            timeout=10,
        )
        if result.returncode == 0:
            # Parse output
            info = {}
            for line in result.stdout.split("\n"):
                if "Pages:" in line:
                    info["pages"] = int(line.split(":")[1].strip())
                elif "Size:" in line:
                    info["size"] = line.split(":")[1].strip()
            return info
    except Exception as e:
        print(f"  ⚠️  Could not get PDF info: {e}")
    return {}


def test_pdf(pdf_path: Path) -> dict:
    """Test a single PDF."""
    pdf_name = pdf_path.stem
    print(f"\n{'='*60}")
    print(f"📄 Testing: {pdf_name}")
    print(f"{'='*60}")

    # Get PDF info
    info = get_pdf_info(pdf_path)
    if info:
        print(
            f"  📊 Pages: {info.get('pages', 'unknown')}, Size: {info.get('size', 'unknown')}"
        )

    # Find gold standard
    gold_path = pdf_path.parent / f"{pdf_name}.gold.md"

    if not gold_path.exists():
        print(f"  ⚠️  No gold standard found")
        return {"name": pdf_name, "status": "no_gold", "score": 0}

    # Extract PDF
    output_path = OUTPUT_DIR / f"{pdf_name}_extracted.md"
    print(f"  🔄 Extracting... (this may take a while)")
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
    gold_lines = len(gold.splitlines())
    extracted_lines = len(extracted.splitlines())

    print(f"  ✅ Extraction complete")
    print(f"  📊 Similarity score: {score:.1f}%")
    print(f"  📝 Gold: {gold_len:,} chars ({gold_lines} lines)")
    print(f"  📝 Extracted: {extracted_len:,} chars ({extracted_lines} lines)")
    print(f"  📏 Ratio: {extracted_len/gold_len:.2f}x")

    return {
        "name": pdf_name,
        "status": "success",
        "score": score,
        "gold_length": gold_len,
        "extracted_length": extracted_len,
        "gold_lines": gold_lines,
        "extracted_lines": extracted_lines,
        "ratio": extracted_len / gold_len if gold_len > 0 else 0,
        **info,
    }


def main():
    """Run tests on real dataset PDFs."""
    print("\n" + "=" * 60)
    print("Real Dataset PDF Extraction Test")
    print("=" * 60)

    # Find all PDFs
    pdfs = sorted(REAL_DATASET_DIR.glob("*.pdf"))

    print(f"\nFound {len(pdfs)} real-world PDFs to test")

    results = []
    for pdf in pdfs:
        result = test_pdf(pdf)
        results.append(result)

    # Summary
    print("\n" + "=" * 60)
    print("SUMMARY - REAL DATASET")
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

        # Show all results sorted by score
        print(f"\n📋 Detailed Results:")
        for i, r in enumerate(
            sorted(successful, key=lambda x: x["score"], reverse=True), 1
        ):
            print(
                f"  {i}. {r['name']:<35s} {r['score']:6.1f}%  ({r['pages']} pages, {r['extracted_length']:,} chars)"
            )

    # Save results
    report_path = OUTPUT_DIR / "real_dataset_report.json"
    with open(report_path, "w") as f:
        json.dump(
            {
                "total": len(results),
                "successful": len(successful),
                "failed": len(failed),
                "no_gold": len(no_gold),
                "average_score": (
                    sum(r["score"] for r in successful) / len(successful)
                    if successful
                    else 0
                ),
                "results": results,
            },
            f,
            indent=2,
        )

    print(f"\n📝 Full report: {report_path}")
    print("\n" + "=" * 60)


if __name__ == "__main__":
    main()
