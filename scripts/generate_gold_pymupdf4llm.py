#!/usr/bin/env python3
"""
Generate gold standard markdown files using pymupdf4llm.

This script converts PDFs to markdown using pymupdf4llm, which is
considered a reference implementation for PDF-to-markdown conversion.

Usage:
    python generate_gold_pymupdf4llm.py --pdf-dir ./test-data/real_dataset
    python generate_gold_pymupdf4llm.py --pdf ./test-data/real_dataset/one_tool_2512.20957v2.pdf

The output files are named with .pymupdf.gold.md suffix.
"""

import argparse
import os
import sys
from pathlib import Path
from typing import List

try:
    import pymupdf4llm
except ImportError:
    print("Error: pymupdf4llm not installed. Run: pip install -U pymupdf4llm")
    sys.exit(1)


def convert_pdf_to_markdown(pdf_path: Path) -> str:
    """Convert a PDF file to markdown using pymupdf4llm.

    Args:
        pdf_path: Path to the PDF file

    Returns:
        Markdown text content
    """
    # Use pymupdf4llm with settings optimized for RAG
    md_text = pymupdf4llm.to_markdown(
        str(pdf_path),
        header=True,  # Include headers
        footer=True,  # Include footers (for references, page numbers)
        write_images=False,  # Don't write image files
        embed_images=False,  # Don't embed images
        force_text=True,  # Force text extraction even for OCR pages
        show_progress=False,  # No progress bar
    )
    return md_text


def process_pdf(pdf_path: Path, output_dir: Path = None) -> Path:
    """Process a single PDF and write the gold standard.

    Args:
        pdf_path: Path to the PDF file
        output_dir: Optional output directory (defaults to same as PDF)

    Returns:
        Path to the generated gold file
    """
    if output_dir is None:
        output_dir = pdf_path.parent

    # Generate output filename
    stem = pdf_path.stem
    gold_path = output_dir / f"{stem}.pymupdf.gold.md"

    print(f"Converting: {pdf_path.name}")

    try:
        md_text = convert_pdf_to_markdown(pdf_path)

        # Write the gold file
        gold_path.write_text(md_text, encoding="utf-8")
        print(f"  ✓ Wrote: {gold_path.name} ({len(md_text)} chars)")

        return gold_path

    except Exception as e:
        print(f"  ✗ Error: {e}")
        return None


def process_directory(pdf_dir: Path, output_dir: Path = None) -> List[Path]:
    """Process all PDFs in a directory.

    Args:
        pdf_dir: Directory containing PDF files
        output_dir: Optional output directory

    Returns:
        List of generated gold file paths
    """
    pdf_files = sorted(pdf_dir.glob("*.pdf"))

    if not pdf_files:
        print(f"No PDF files found in {pdf_dir}")
        return []

    print(f"Found {len(pdf_files)} PDF files")
    print("-" * 60)

    results = []
    for pdf_path in pdf_files:
        result = process_pdf(pdf_path, output_dir)
        if result:
            results.append(result)

    print("-" * 60)
    print(f"Generated {len(results)}/{len(pdf_files)} gold files")

    return results


def main():
    parser = argparse.ArgumentParser(
        description="Generate gold standard markdown files using pymupdf4llm"
    )
    parser.add_argument("--pdf", type=Path, help="Path to a single PDF file to convert")
    parser.add_argument(
        "--pdf-dir", type=Path, help="Directory containing PDF files to convert"
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        help="Output directory for gold files (default: same as input)",
    )

    args = parser.parse_args()

    if not args.pdf and not args.pdf_dir:
        parser.error("Either --pdf or --pdf-dir must be specified")

    if args.pdf and args.pdf_dir:
        parser.error("Cannot specify both --pdf and --pdf-dir")

    if args.pdf:
        if not args.pdf.exists():
            print(f"Error: PDF file not found: {args.pdf}")
            sys.exit(1)
        process_pdf(args.pdf, args.output_dir)

    elif args.pdf_dir:
        if not args.pdf_dir.is_dir():
            print(f"Error: Directory not found: {args.pdf_dir}")
            sys.exit(1)
        process_directory(args.pdf_dir, args.output_dir)


if __name__ == "__main__":
    main()
