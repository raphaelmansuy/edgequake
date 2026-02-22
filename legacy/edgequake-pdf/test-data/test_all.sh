#!/bin/bash
# Systematic test of all PDF files

set -e

BIN="cargo run --release --bin edgequake-pdf --"
OUTPUT_DIR="output"

mkdir -p "$OUTPUT_DIR"

echo "================================"
echo "PDF Conversion Test Suite"
echo "================================"
echo ""

# Test each numbered PDF systematically
for pdf in $(ls -1 [0-9][0-9][0-9]_*.pdf 2>/dev/null | sort); do
    basename="${pdf%.pdf}"
    output="$OUTPUT_DIR/${basename}.md"
    
    echo "Testing: $pdf"
    echo "  → Info..."
    $BIN info -i "$pdf" | head -10
    
    echo "  → Converting..."
    $BIN convert -i "$pdf" -o "$output"
    
    # Show first 20 lines of output
    echo "  → Preview (first 20 lines):"
    head -20 "$output" | sed 's/^/    /'
    
    echo ""
    echo "---"
    echo ""
done

echo "All tests completed!"
echo "Results in: $OUTPUT_DIR/"
