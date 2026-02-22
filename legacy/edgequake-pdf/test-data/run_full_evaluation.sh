#!/bin/bash
# Run full evaluation protocol: generate PDFs, extract, evaluate, and report
set -e
ROOT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT_DIR"

echo "1) Generate PDFs from gold/ (simple fallback generator)"
python3 generate_simple_pdfs.py

echo "2) Run Rust comprehensive evaluation (ignored test)"
# This runs the test which will extract PDFs and produce diffs and a JSON report
cd ..
cargo test --package edgequake-pdf --test comprehensive_evaluation -- --ignored --nocapture

echo "Evaluation complete. Reports are in test-data/diffs/evaluation_report.json and evaluation_report.json"
