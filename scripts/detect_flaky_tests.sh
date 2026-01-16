#!/bin/bash
# Flaky Test Detection Script
# Purpose: Run tests multiple times to detect non-deterministic failures
# Usage: ./detect_flaky_tests.sh [iterations] [package]

set -e

ITERATIONS=${1:-3}
PACKAGE=${2:-"all"}
RESULTS_DIR="test-results/flaky-detection"

mkdir -p "$RESULTS_DIR"

echo "=================================================="
echo "Flaky Test Detection"
echo "Iterations: $ITERATIONS"
echo "Package: $PACKAGE"
echo "=================================================="

# Function to run tests and capture results
run_test_iteration() {
    local iter=$1
    local package=$2
    local output_file="$RESULTS_DIR/iteration_${iter}.txt"
    
    echo "Running iteration $iter..."
    
    if [ "$package" == "all" ]; then
        cd edgequake && cargo test --all 2>&1 | tee "$output_file"
    else
        cd edgequake && cargo test --package "$package" 2>&1 | tee "$output_file"
    fi
    
    # Extract failed tests
    grep "FAILED" "$output_file" | sort > "$RESULTS_DIR/failed_${iter}.txt" || true
    
    # Count results
    PASSED=$(grep -c "test result: ok" "$output_file" || echo "0")
    FAILED=$(grep -c "FAILED" "$output_file" || echo "0")
    
    echo "Iteration $iter: $PASSED passed, $FAILED failed"
    
    cd ..
}

# Run multiple iterations
for i in $(seq 1 $ITERATIONS); do
    run_test_iteration $i "$PACKAGE"
done

# Analyze results for flakiness
echo ""
echo "=================================================="
echo "Analyzing Results"
echo "=================================================="

# Find tests that failed inconsistently
echo "" > "$RESULTS_DIR/flaky_candidates.txt"
echo "" > "$RESULTS_DIR/consistent_failures.txt"

# Get unique failed tests across all iterations
cat $RESULTS_DIR/failed_*.txt 2>/dev/null | sort | uniq > "$RESULTS_DIR/all_failed.txt"

if [ -s "$RESULTS_DIR/all_failed.txt" ]; then
    while IFS= read -r test; do
        # Count how many times this test failed
        FAIL_COUNT=$(grep -l "$test" $RESULTS_DIR/failed_*.txt 2>/dev/null | wc -l | tr -d ' ')
        
        if [ "$FAIL_COUNT" -lt "$ITERATIONS" ] && [ "$FAIL_COUNT" -gt 0 ]; then
            echo "FLAKY: $test (failed $FAIL_COUNT/$ITERATIONS)" >> "$RESULTS_DIR/flaky_candidates.txt"
        elif [ "$FAIL_COUNT" -eq "$ITERATIONS" ]; then
            echo "CONSISTENT: $test (failed $FAIL_COUNT/$ITERATIONS)" >> "$RESULTS_DIR/consistent_failures.txt"
        fi
    done < "$RESULTS_DIR/all_failed.txt"
fi

# Report
echo ""
echo "=================================================="
echo "Results Summary"
echo "=================================================="

FLAKY_COUNT=$(grep -c "FLAKY:" "$RESULTS_DIR/flaky_candidates.txt" 2>/dev/null || echo "0")
CONSISTENT_COUNT=$(grep -c "CONSISTENT:" "$RESULTS_DIR/consistent_failures.txt" 2>/dev/null || echo "0")

echo "Flaky tests detected: $FLAKY_COUNT"
echo "Consistent failures: $CONSISTENT_COUNT"

if [ "$FLAKY_COUNT" -gt 0 ]; then
    echo ""
    echo "⚠️  FLAKY TESTS DETECTED:"
    cat "$RESULTS_DIR/flaky_candidates.txt"
    echo ""
    echo "These tests need investigation - they pass/fail non-deterministically"
fi

if [ "$CONSISTENT_COUNT" -gt 0 ]; then
    echo ""
    echo "❌ CONSISTENT FAILURES:"
    cat "$RESULTS_DIR/consistent_failures.txt"
fi

# Generate JSON report
cat > "$RESULTS_DIR/report.json" << EOF
{
  "timestamp": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
  "iterations": $ITERATIONS,
  "package": "$PACKAGE",
  "flaky_count": $FLAKY_COUNT,
  "consistent_failures": $CONSISTENT_COUNT,
  "status": "$([ "$FLAKY_COUNT" -eq 0 ] && echo 'clean' || echo 'flaky_detected')"
}
EOF

echo ""
echo "Full report saved to: $RESULTS_DIR/"

# Exit with error if flaky tests found
if [ "$FLAKY_COUNT" -gt 0 ]; then
    echo "::error::Flaky tests detected! See $RESULTS_DIR/flaky_candidates.txt"
    exit 1
fi

echo "✅ No flaky tests detected!"
exit 0
