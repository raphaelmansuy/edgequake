#!/usr/bin/env python3
"""Generate comprehensive test report combining all test results."""

import json
from datetime import datetime
from pathlib import Path


def load_results():
    """Load all test results."""
    synthetic_path = Path("/tmp/pdf_test_results/test_report.json")
    real_path = Path("/tmp/pdf_real_dataset_results/real_dataset_report.json")

    synthetic = json.load(open(synthetic_path))
    real_dataset = json.load(open(real_path))

    return synthetic, real_dataset


def generate_markdown_report(synthetic, real_dataset):
    """Generate comprehensive markdown report."""

    report = []
    report.append("# EdgeQuake PDF Extraction Test Report")
    report.append(f"\n**Generated:** {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    report.append("\n---\n")

    # Executive Summary
    report.append("## Executive Summary\n")

    total_tests = synthetic["total"] + real_dataset["total"]
    total_successful = synthetic["successful"] + real_dataset["successful"]
    total_failed = synthetic["failed"] + real_dataset["failed"]

    report.append(f"- **Total PDFs Tested:** {total_tests}")
    report.append(
        f"- **Successful Extractions:** {total_successful} ({total_successful/total_tests*100:.1f}%)"
    )
    report.append(
        f"- **Failed Extractions:** {total_failed} ({total_failed/total_tests*100:.1f}%)"
    )
    report.append("\n")

    # Synthetic Test Results
    report.append("## Synthetic Test Dataset (39 PDFs)\n")
    report.append(f"These are generated PDFs designed to test specific features.\n")
    report.append(
        f"\n- **Success Rate:** {synthetic['successful']}/{synthetic['total']} ({synthetic['successful']/synthetic['total']*100:.1f}%)"
    )
    report.append(
        f"- **Average Similarity:** {sum(r['score'] for r in synthetic['results'] if r['status'] == 'success') / synthetic['successful']:.1f}%\n"
    )

    # Top performers
    successful_synthetic = [r for r in synthetic["results"] if r["status"] == "success"]
    top_10 = sorted(successful_synthetic, key=lambda x: x["score"], reverse=True)[:10]

    report.append("\n### Top 10 Performing Tests\n")
    report.append("| Rank | Test Name | Score | Size |")
    report.append("|------|-----------|-------|------|")
    for i, r in enumerate(top_10, 1):
        report.append(
            f"| {i} | `{r['name']}` | {r['score']:.1f}% | {r['extracted_length']:,} chars |"
        )

    # Bottom performers
    bottom_10 = sorted(successful_synthetic, key=lambda x: x["score"])[:10]

    report.append("\n### Bottom 10 Performing Tests\n")
    report.append("| Rank | Test Name | Score | Size |")
    report.append("|------|-----------|-------|------|")
    for i, r in enumerate(bottom_10, 1):
        report.append(
            f"| {i} | `{r['name']}` | {r['score']:.1f}% | {r['extracted_length']:,} chars |"
        )

    # Score distribution
    report.append("\n### Score Distribution\n")
    excellent = len([r for r in successful_synthetic if r["score"] >= 80])
    good = len([r for r in successful_synthetic if 60 <= r["score"] < 80])
    acceptable = len([r for r in successful_synthetic if 40 <= r["score"] < 60])
    poor = len([r for r in successful_synthetic if r["score"] < 40])

    report.append(f"- **Excellent (≥80%):** {excellent} tests")
    report.append(f"- **Good (60-79%):** {good} tests")
    report.append(f"- **Acceptable (40-59%):** {acceptable} tests")
    report.append(f"- **Poor (<40%):** {poor} tests")

    # Real Dataset Results
    report.append("\n---\n")
    report.append("## Real-World Dataset (5 PDFs)\n")
    report.append("These are actual academic papers from arXiv.\n")
    report.append(
        f"\n- **Success Rate:** {real_dataset['successful']}/{real_dataset['total']} ({real_dataset['successful']/real_dataset['total']*100:.1f}%)"
    )
    report.append(f"- **Average Similarity:** {real_dataset['average_score']:.1f}%\n")

    report.append("\n### Detailed Results\n")
    report.append("| Rank | Document | Score | Pages | Size | Ratio |")
    report.append("|------|----------|-------|-------|------|-------|")

    real_sorted = sorted(
        real_dataset["results"], key=lambda x: x.get("score", 0), reverse=True
    )
    for i, r in enumerate(real_sorted, 1):
        if r["status"] == "success":
            report.append(
                f"| {i} | `{r['name']}` | {r['score']:.1f}% | {r.get('pages', 'N/A')} | {r['extracted_length']:,} chars | {r['ratio']:.2f}x |"
            )

    # Analysis
    report.append("\n---\n")
    report.append("## Analysis\n")

    report.append("\n### Strengths\n")
    report.append("- Successfully extracts text from multi-column layouts")
    report.append("- Handles nested lists and structured content well")
    report.append("- Good performance on simple tables")
    report.append("- Maintains basic formatting (bold, headings)")

    report.append("\n### Areas for Improvement\n")
    report.append("- Math formulas extraction (0% success)")
    report.append("- Encrypted/password-protected PDFs (0% success)")
    report.append("- Complex Unicode mappings")
    report.append("- Rotated text and overlapping layers")
    report.append("- Vector graphics with text on paths")

    report.append("\n### Recommendations\n")
    report.append(
        "1. **Math Support:** Integrate MathML or LaTeX extraction for mathematical content"
    )
    report.append("2. **Encryption:** Add support for password-protected PDFs")
    report.append(
        "3. **Text Orientation:** Improve handling of rotated and transformed text"
    )
    report.append(
        "4. **Unicode:** Enhance character mapping for special characters and symbols"
    )
    report.append(
        "5. **Layout Analysis:** Refine multi-column and complex layout detection"
    )

    # Technical Details
    report.append("\n---\n")
    report.append("## Technical Details\n")
    report.append("\n### Test Environment\n")
    report.append("- **Framework:** EdgeQuake PDF Extraction")
    report.append("- **Backend:** lopdf (Rust)")
    report.append("- **Test Data:**")
    report.append(f"  - Synthetic: {synthetic['total']} generated PDFs")
    report.append(f"  - Real-world: {real_dataset['total']} academic papers")

    report.append("\n### Scoring Methodology\n")
    report.append(
        "- **Similarity Score:** SequenceMatcher ratio between gold standard and extracted text"
    )
    report.append("- **Scale:** 0-100% (higher is better)")
    report.append("- **Gold Standards:** Manually verified markdown files")

    report.append("\n---\n")
    report.append("\n*End of Report*\n")

    return "\n".join(report)


def main():
    """Generate and save comprehensive report."""
    print("Generating comprehensive test report...")

    synthetic, real_dataset = load_results()

    report = generate_markdown_report(synthetic, real_dataset)

    # Save report
    output_path = Path("PDF_TEST_REPORT.md")
    with open(output_path, "w") as f:
        f.write(report)

    print(f"\n✅ Report generated: {output_path.absolute()}")

    # Also print to console
    print("\n" + "=" * 60)
    print(report)
    print("=" * 60)


if __name__ == "__main__":
    main()
