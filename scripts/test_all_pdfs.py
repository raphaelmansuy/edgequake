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
        print(f"  ca Error: {e}")
        return False


def similarity_score(text1: str, text2: str) -> float:
    """Calculate similarity between two texts."""
    return SequenceMatcher(None, text1, text2).ratio() * 100


def test_pdf(pdf_path: Path) -> dict:
    """Test a single PDF."""
    pdf_name = pdf_path.stem
