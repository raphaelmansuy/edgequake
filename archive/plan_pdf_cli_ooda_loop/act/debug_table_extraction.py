#!/usr/bin/env python3
"""Debug script to analyze PDF table block extraction"""

import json
import subprocess
import sys


def main():
    pdf_path = (
        sys.argv[1]
        if len(sys.argv) > 1
        else "plan_pdf_cli_ooda_loop/observe/output/03_tables.pdf"
    )

    # Convert to JSON to see block structure
    cmd = [
        "./edgequake/target/release/edgequake-pdf",
        "convert",
        "-i",
        pdf_path,
        "-o",
        "/tmp/table_debug.md",
    ]

    result = subprocess.run(
        cmd,
        capture_output=True,
        text=True,
        cwd="/Users/raphaelmansuy/Github/03-working/edgequake",
    )

    print("=== EXTRACTION LOG ===")
    for line in result.stderr.split("\n"):
        if "BEFORE processors" in line or "block" in line.lower():
            print(line)

    print("\n=== CONVERTED MARKDOWN (first 50 lines) ===")
    with open("/tmp/table_debug.md") as f:
        for i, line in enumerate(f):
            if i >= 50:
                break
            print(f"{i+1:3d}: {line}", end="")


if __name__ == "__main__":
    main()
