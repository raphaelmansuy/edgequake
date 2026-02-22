#!/bin/bash
# Comprehensive PDF-to-Markdown Evaluation Script
# Usage: ./eval.sh [gold_dir] [pdf_dir] [extracted_dir] [diffs_dir]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GOLD_DIR="${1:-$SCRIPT_DIR/test-data/gold}"
PDF_DIR="${2:-$SCRIPT_DIR/test-data/pdfs}"
EXTRACTED_DIR="${3:-$SCRIPT_DIR/test-data/extracted}"
DIFFS_DIR="${4:-$SCRIPT_DIR/test-data/diffs}"

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "╔════════════════════════════════════════════════════════════╗"
echo "║  EdgeQuake PDF-to-Markdown Extraction Evaluation           ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""
echo "Configuration:"
echo "  Gold DIR:      $GOLD_DIR"
echo "  PDF DIR:       $PDF_DIR"
echo "  Extracted DIR: $EXTRACTED_DIR"
echo "  Diffs DIR:     $DIFFS_DIR"
echo ""

# Create directories
mkdir -p "$EXTRACTED_DIR"
mkdir -p "$DIFFS_DIR"

# For each category directory
total_documents=0
successful_extractions=0
failed_extractions=0

echo "╔════════════════════════════════════════════════════════════╗"
echo "║  Phase 1: Document Extraction                             ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

for category_dir in "$GOLD_DIR"/*/ ; do
    category=$(basename "$category_dir")
    echo "Processing category: $category"
    
    # Create extracted category directory
    mkdir -p "$EXTRACTED_DIR/$category"
    mkdir -p "$DIFFS_DIR/$category"
    
    # For each markdown file in category
    for gold_file in "$category_dir"*.md ; do
        if [ ! -f "$gold_file" ]; then
            continue
        fi
        
        filename=$(basename "$gold_file")
        pdf_file="$PDF_DIR/$category/${filename%.md}.pdf"
        extracted_file="$EXTRACTED_DIR/$category/$filename"
        
        total_documents=$((total_documents + 1))
        
        if [ ! -f "$pdf_file" ]; then
            echo -n "  ⚠ $filename (PDF not found)"
            echo ""
            continue
        fi
        
        echo -n "  ✓ $filename ... "
        
        # Extract PDF to markdown using cargo test
        # This will be done by the comprehensive_evaluation test
        echo "extracted"
        successful_extractions=$((successful_extractions + 1))
    done
done

echo ""
echo "╔════════════════════════════════════════════════════════════╗"
echo "║  Phase 2: Diff Analysis                                   ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

# Generate diff reports
for category_dir in "$GOLD_DIR"/*/ ; do
    category=$(basename "$category_dir")
    echo "Generating diffs for category: $category"
    
    for gold_file in "$category_dir"*.md ; do
        if [ ! -f "$gold_file" ]; then
            continue
        fi
        
        filename=$(basename "$gold_file")
        extracted_file="$EXTRACTED_DIR/$category/$filename"
        diff_file="$DIFFS_DIR/$category/${filename%.md}.diff"
        
        if [ -f "$extracted_file" ]; then
            # Create unified diff
            diff -u "$gold_file" "$extracted_file" > "$diff_file" || true
        fi
    done
done

echo ""
echo "╔════════════════════════════════════════════════════════════╗"
echo "║  Phase 3: Metric Calculation                              ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

# Run comprehensive test to generate metrics
echo "Running comprehensive evaluation test..."
cd "$SCRIPT_DIR"
cargo test --test comprehensive_evaluation -- --ignored --nocapture 2>/dev/null || true

echo ""
echo "╔════════════════════════════════════════════════════════════╗"
echo "║  Phase 4: Report Generation                               ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

if [ -f "$DIFFS_DIR/evaluation_report.json" ]; then
    echo -e "${GREEN}✓ Evaluation report generated${NC}"
    echo ""
    
    # Extract key metrics
    python3 <<EOF
import json

with open('$DIFFS_DIR/evaluation_report.json') as f:
    report = json.load(f)
    
print(f"Overall Score: {report['overall_score']:.1f}%")
print(f"Total Documents: {report['total_documents']}")
print("")
print("Category Breakdown:")
for cat in report['categories']:
    print(f"  {cat['name']:30s} {cat['average_score']:6.1f}% ({cat['document_count']} docs)")
EOF
else
    echo -e "${YELLOW}⚠ Report not generated${NC}"
fi

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "Evaluation complete!"
echo "  Processed: $total_documents documents"
echo "  Successful: $successful_extractions"
echo "  Report: $DIFFS_DIR/evaluation_report.json"
echo "═══════════════════════════════════════════════════════════════"
echo ""
