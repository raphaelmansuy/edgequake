#!/usr/bin/env python3
"""
Markdown to PDF converter for test data generation.

Converts all markdown files in gold/ directory to PDF format,
placing PDFs in corresponding pdfs/ directory structure.
"""

import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Dict, List, Tuple

# Configuration
REPO_ROOT = Path(__file__).parent
GOLD_DIR = REPO_ROOT / "gold"
PDF_DIR = REPO_ROOT / "pdfs"
EXTRACTED_DIR = REPO_ROOT / "extracted"
DIFFS_DIR = REPO_ROOT / "diffs"


def check_dependencies():
    """Check if required tools are installed."""
    required = ["pandoc"]
    missing = []

    for cmd in required:
        try:
            subprocess.run([cmd, "--version"], capture_output=True, check=True)
        except (FileNotFoundError, subprocess.CalledProcessError):
            missing.append(cmd)

    if missing:
        print(f"Error: Missing required tools: {', '.join(missing)}")
        print("\nInstall with:")
        print("  macOS: brew install pandoc wkhtmltopdf")
        print("  Ubuntu: sudo apt-get install pandoc wkhtmltopdf")
        return False
    return True


def get_markdown_files() -> List[Path]:
    """Get all markdown files from gold directory."""
    md_files = list(GOLD_DIR.rglob("*.md"))
    return sorted(md_files)


def create_pdf(md_file: Path, pdf_file: Path) -> bool:
    """Convert markdown to PDF using pandoc."""
    try:
        pdf_file.parent.mkdir(parents=True, exist_ok=True)

        # Try wkhtmltopdf first
        cmd = [
            "pandoc",
            str(md_file),
            "-o",
            str(pdf_file),
            "--pdf-engine=wkhtmltopdf",
            "--standalone",
            "-V",
            "margin-left=20mm",
            "-V",
            "margin-right=20mm",
            "-V",
            "margin-top=20mm",
            "-V",
            "margin-bottom=20mm",
        ]

        result = subprocess.run(cmd, capture_output=True, text=True, timeout=30)

        if result.returncode == 0:
            return True

        # Try xvfb-run wkhtmltopdf for Linux
        cmd_xvfb = [
            "pandoc",
            str(md_file),
            "-o",
            str(pdf_file),
            "--pdf-engine=xvfb-run wkhtmltopdf",
            "--standalone",
        ]
        result_xvfb = subprocess.run(
            cmd_xvfb, capture_output=True, text=True, timeout=30
        )

        if result_xvfb.returncode == 0:
            return True

        # Try weasyprint
        cmd_weasy = [
            "pandoc",
            str(md_file),
            "-o",
            str(pdf_file),
            "--pdf-engine=weasyprint",
            "--standalone",
        ]
        result_weasy = subprocess.run(
            cmd_weasy, capture_output=True, text=True, timeout=30
        )

        if result_weasy.returncode == 0:
            return True

        # Try default LaTeX engine
        cmd_latex = [
            "pandoc",
            str(md_file),
            "-o",
            str(pdf_file),
            "--pdf-engine=pdflatex",
        ]
        result_latex = subprocess.run(
            cmd_latex, capture_output=True, text=True, timeout=30
        )

        if result_latex.returncode == 0:
            return True

        print(f"Error: {result_latex.stderr[:100]}")
        return False

    except subprocess.TimeoutExpired:
        return False
    except Exception as e:
        print(f"Exception: {e}")
        return False

    doc = SimpleDocTemplate(path, pagesize=letter)
    doc.title = "Simple Text Test"
    styles = getSampleStyleSheet()
    story = []
    story.append(Paragraph("Simple Text Test", styles["Title"]))
    story.append(Spacer(1, 12))
    story.append(
        Paragraph(
            "This is a simple paragraph of text. It should be extracted as a single block of text in Markdown.",
            styles["Normal"],
        )
    )
    story.append(Spacer(1, 12))
    story.append(
        Paragraph(
            "Another paragraph follows. The extractor should maintain the separation between these two paragraphs.",
            styles["Normal"],
        )
    )
    doc.build(story)


def convert_all_documents() -> tuple:
    """Convert all markdown documents to PDF."""
    md_files = get_markdown_files()

    if not md_files:
        print("No markdown files found in gold directory")
        return 0, 0

    successful = 0
    failed = 0

    print(f"Found {len(md_files)} markdown documents")
    print(f"Converting to PDF...\n")

    for i, md_file in enumerate(md_files, 1):
        relative_path = md_file.relative_to(GOLD_DIR)
        pdf_file = PDF_DIR / relative_path.with_suffix(".pdf")

        print(f"[{i:3d}/{len(md_files)}] {relative_path}...", end=" ", flush=True)

        if create_pdf(md_file, pdf_file):
            print("✓")
            successful += 1
        else:
            print("✗")
            failed += 1

    print(f"\nConversion complete: {successful} successful, {failed} failed")
    return successful, failed


def create_directory_structure():
    """Create output directories if they don't exist."""
    PDF_DIR.mkdir(parents=True, exist_ok=True)
    EXTRACTED_DIR.mkdir(parents=True, exist_ok=True)
    DIFFS_DIR.mkdir(parents=True, exist_ok=True)

    for subdir in GOLD_DIR.iterdir():
        if subdir.is_dir():
            (PDF_DIR / subdir.name).mkdir(parents=True, exist_ok=True)
            (EXTRACTED_DIR / subdir.name).mkdir(parents=True, exist_ok=True)


def generate_manifest() -> dict:
    """Generate manifest of all test documents."""
    manifest = {
        "categories": {},
        "total_documents": 0,
    }

    for category_dir in sorted(GOLD_DIR.iterdir()):
        if not category_dir.is_dir():
            continue

        md_files = sorted(category_dir.glob("*.md"))
        category_name = category_dir.name

        manifest["categories"][category_name] = {
            "document_count": len(md_files),
            "documents": [f.stem for f in md_files],
        }
        manifest["total_documents"] += len(md_files)

    return manifest


def main():
    """Main entry point."""
    print("EdgeQuake PDF Test Data Generator")
    print("=" * 50)

    if not check_dependencies():
        return 1

    print("Creating directory structure...")
    create_directory_structure()

    print("Generating manifest...")
    manifest = generate_manifest()
    manifest_file = REPO_ROOT / "MANIFEST.json"
    with open(manifest_file, "w") as f:
        json.dump(manifest, f, indent=2)
    print(f"Manifest saved to {manifest_file.name}")
    print(f"Total documents: {manifest['total_documents']}")

    print()
    successful, failed = convert_all_documents()

    print("\n" + "=" * 50)
    print("Summary:")
    print(f"  Total documents: {len(get_markdown_files())}")
    print(f"  PDF files created: {successful}")
    print(f"  Conversion failures: {failed}")

    if failed == 0:
        print("\n✓ All conversions successful!")
        return 0
    else:
        print(f"\n⚠ {failed} documents failed to convert")
        return 1


if __name__ == "__main__":
    sys.exit(main())
