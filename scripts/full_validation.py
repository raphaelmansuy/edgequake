#!/usr/bin/env python3
"""
Full codebase validation scanning both frontend and backend.
Provides combined metrics for the entire EdgeQuake codebase.
"""

import json
import subprocess
import sys


def run_validation(code_dir: str, docs_file: str) -> dict:
    """Run validation on a code directory and return JSON results."""
    result = subprocess.run(
        [
            "python3",
            ".github/skills/doc-traceability-validator/scripts/validate_features.py",
            "--code-dir",
            code_dir,
            "--docs-file",
            docs_file,
            "--output-json",
            f'/tmp/validation_{code_dir.replace("/", "_")}.json',
        ],
        capture_output=True,
        text=True,
    )

    try:
        with open(f'/tmp/validation_{code_dir.replace("/", "_")}.json') as f:
            return json.load(f)
    except:
        return None


def main():
    # Scan frontend
    print("ca Scanning frontend (edgequake_webui/src)...")
    frontend = run_validation("edgequake_webui/src", "docs/features.md")

