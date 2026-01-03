#!/usr/bin/env python3
"""
PDF-to-Markdown Evaluation Protocol

This script evaluates the fidelity of PDF-to-Markdown conversion by comparing
extracted Markdown against gold standard references using diff-based analysis.

Test Protocol:
1. Gold Standard: .gold.md files are the single source of truth
2. Extracted: .md files are the conversion results
3. Comparison: Unified diff analysis with quantitative metrics
4. Metrics: Text preservation, formatting preservation, structural fidelity
5. Report: JSON output for report.py to generate HTML

Usage: python3 eval_protocol.py
Output: evaluation_results.json
"""

import difflib
import json
import os
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Tuple


def calculate_diff_metrics(gold_text: str, extracted_text: str) -> Dict:
    """Calculate diff-based metrics between gold and extracted text."""

    gold_lines = gold_text.splitlines(keepends=True)
    extracted_lines = extracted_text.splitlines(keepends=True)

    # Unified diff
    diff = list(
        difflib.unified_diff(
            gold_lines,
            extracted_lines,
            fromfile="gold",
            tofile="extracted",
            lineterm="",
            n=3,
        )
    )

    # Basic metrics
    total_gold_lines = len(gold_lines)
    total_extracted_lines = len(extracted_lines)

    # Count added/removed lines from diff
    added_lines = sum(
        1 for line in diff if line.startswith("+") and not line.startswith("+++")
    )
    removed_lines = sum(
        1 for line in diff if line.startswith("-") and not line.startswith("---")
    )

    # Text similarity
    similarity = difflib.SequenceMatcher(None, gold_text, extracted_text).ratio()

    # Character count preservation
    gold_chars = len(gold_text.strip())
    extracted_chars = len(extracted_text.strip())
    char_preservation = extracted_chars / gold_chars if gold_chars > 0 else 0

    return {
        "total_gold_lines": total_gold_lines,
        "total_extracted_lines": total_extracted_lines,
        "added_lines": added_lines,
        "removed_lines": removed_lines,
        "similarity_ratio": similarity,
        "gold_chars": gold_chars,
        "extracted_chars": extracted_chars,
        "char_preservation": char_preservation,
        "diff": diff[:50],  # First 50 lines of diff for inspection
    }


def evaluate_file(gold_path: Path, extracted_path: Path) -> Dict:
    """Evaluate a single file pair."""

    try:
        with open(gold_path, "r", encoding="utf-8") as f:
            gold_text = f.read()

        with open(extracted_path, "r", encoding="utf-8") as f:
            extracted_text = f.read()

        metrics = calculate_diff_metrics(gold_text, extracted_text)

        # Determine quality score (0-100)
        similarity = metrics["similarity_ratio"]
        char_preserve = metrics["char_preservation"]

        # Weighted score: 60% similarity, 40% char preservation
        quality_score = (similarity * 0.6 + min(char_preserve, 1.0) * 0.4) * 100

        return {
            "filename": gold_path.stem.replace(".gold", ""),
            "gold_path": str(gold_path),
            "extracted_path": str(extracted_path),
            "metrics": metrics,
            "quality_score": round(quality_score, 1),
            "status": (
                "excellent"
                if quality_score >= 90
                else (
                    "good"
                    if quality_score >= 80
                    else "acceptable" if quality_score >= 60 else "poor"
                )
            ),
        }

    except Exception as e:
        return {
            "filename": gold_path.stem.replace(".gold", ""),
            "error": str(e),
            "quality_score": 0,
            "status": "error",
        }


def main():
    """Main evaluation function."""

    test_dir = Path(__file__).parent

    # Find all gold standard files
    gold_files = list(test_dir.glob("*.gold.md"))
    gold_files.sort()

    results = []
    total_score = 0
    valid_tests = 0

    print(f"Starting evaluation of {len(gold_files)} test cases...")

    for gold_path in gold_files:
        # Find corresponding extracted file
        extracted_name = gold_path.name.replace(".gold.md", ".md")
        extracted_path = test_dir / extracted_name

        if not extracted_path.exists():
            print(f"Warning: Extracted file not found: {extracted_path}")
            continue

        print(f"Evaluating: {gold_path.name} vs {extracted_name}")

        result = evaluate_file(gold_path, extracted_path)
        results.append(result)

        if "error" not in result:
            total_score += result["quality_score"]
            valid_tests += 1

    # Summary
    avg_score = total_score / valid_tests if valid_tests > 0 else 0

    summary = {
        "timestamp": datetime.now().isoformat(),
        "total_tests": len(gold_files),
        "valid_tests": valid_tests,
        "average_quality_score": round(avg_score, 1),
        "results": results,
    }

    # Save JSON report
    output_path = test_dir / "evaluation_results.json"
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(summary, f, indent=2, ensure_ascii=False)

    print(f"\nEvaluation complete!")
    print(f"Results saved to: {output_path}")
    print(f"Average Quality Score: {avg_score:.1f}/100")

    # Categorize results
    excellent = sum(1 for r in results if r.get("status") == "excellent")
    good = sum(1 for r in results if r.get("status") == "good")
    acceptable = sum(1 for r in results if r.get("status") == "acceptable")
    poor = sum(1 for r in results if r.get("status") == "poor")

    print("\nBreakdown:")
    print(f"  Excellent (90-100): {excellent}")
    print(f"  Good (80-89): {good}")
    print(f"  Acceptable (60-79): {acceptable}")
    print(f"  Poor (0-59): {poor}")


if __name__ == "__main__":
    main()
