#!/bin/bash
# EdgeQuake PDF Test Runner
# Unified script to run all PDF extraction tests

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo "======================================"
echo "EdgeQuake PDF Test Suite"
echo "======================================"
echo ""

# Function to run tests and report
run_test_suite() {
    local name="$1"
    local command="$2"
    
    echo -e "${YELLOW}Running $name...${NC}"
    if eval "$command"; then
        echo -e "${GREEN}✓ $name passed${NC}"
        return 0
    else
        echo -e "${RED}✗ $name failed${NC}"
        return 1
    fi
    echo ""
}

# Track failures
FAILED=0

# 1. Run lib tests (fast unit tests)
run_test_suite "Lib Tests" "cargo test --lib --quiet" || FAILED=$((FAILED + 1))

# 2. Run quality evaluation
run_test_suite "Quality Evaluation" "cargo test --test quality_evaluation --quiet" || FAILED=$((FAILED + 1))

# 3. Run integration tests
run_test_suite "Integration Tests" "cargo test --test integration_tests --quiet" || FAILED=$((FAILED + 1))

# 4. Run edge cases
run_test_suite "Edge Cases" "cargo test --test edge_cases_and_complex --quiet" || FAILED=$((FAILED + 1))

# 5. Run comprehensive tests (slow - optional)
if [ "$1" == "--full" ]; then
    echo -e "${YELLOW}Running full comprehensive test suite...${NC}"
    run_test_suite "Comprehensive Tests" "cargo test --test comprehensive_test_data --quiet" || FAILED=$((FAILED + 1))
fi

# Summary
echo ""
echo "======================================"
if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}All test suites passed!${NC}"
    echo "======================================"
    exit 0
else
    echo -e "${RED}$FAILED test suite(s) failed${NC}"
    echo "======================================"
    exit 1
fi
