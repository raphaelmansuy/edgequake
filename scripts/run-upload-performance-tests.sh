#!/usr/bin/env bash
#
# Run upload performance tests with progressive load testing
#
# Usage:
#   ./run-upload-performance-tests.sh [frontend|backend|both|quick]
#
# Options:
#   frontend  - Run Playwright E2E performance tests only
#   backend   - Run Rust backend performance tests only
#   both      - Run all performance tests (default)
#   quick     - Run quick smoke test (reduced load)

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Test mode
MODE="${1:-both}"

# ============================================================================
# Helper Functions
# ============================================================================

print_header() {
    echo ""
    echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}  $1${NC}"
    echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
    echo ""
}

print_info() {
    echo -e "${GREEN}ℹ${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

check_service() {
    local service=$1
    local url=$2
    
    if curl -s -f "$url" > /dev/null 2>&1; then
        print_success "$service is running at $url"
        return 0
    else
        print_error "$service is not responding at $url"
        return 1
    fi
}

# ============================================================================
# Pre-flight Checks
# ============================================================================

preflight_checks() {
    print_header "Pre-flight Checks"
    
    local all_ok=true
    
    # Check if backend is running
    if [ "$MODE" = "frontend" ] || [ "$MODE" = "both" ]; then
        if ! check_service "Backend API" "http://localhost:8080/health"; then
            print_warning "Backend not running. Starting backend..."
            cd "$PROJECT_ROOT"
            make backend > /tmp/edgequake-backend.log 2>&1 &
            sleep 5
            
            if check_service "Backend API" "http://localhost:8080/health"; then
                print_success "Backend started successfully"
            else
                print_error "Failed to start backend. Check /tmp/edgequake-backend.log"
                all_ok=false
            fi
        fi
    fi
    
    # Check if frontend is running (for Playwright)
    if [ "$MODE" = "frontend" ] || [ "$MODE" = "both" ]; then
        if ! check_service "Frontend" "http://localhost:3001"; then
            print_warning "Frontend not running. Starting frontend..."
            cd "$PROJECT_ROOT/edgequake_webui"
            npm run dev -- --port 3001 > /tmp/edgequake-frontend.log 2>&1 &
            sleep 10
            
            if check_service "Frontend" "http://localhost:3001"; then
                print_success "Frontend started successfully"
            else
                print_error "Failed to start frontend. Check /tmp/edgequake-frontend.log"
                all_ok=false
            fi
        fi
    fi
    
    # Check database (optional but recommended)
    if pg_isready -h localhost > /dev/null 2>&1; then
        print_success "PostgreSQL is running"
    else
        print_warning "PostgreSQL not detected - using in-memory storage"
    fi
    
    if [ "$all_ok" = false ]; then
        print_error "Pre-flight checks failed. Please fix issues and try again."
        exit 1
    fi
    
    print_success "All pre-flight checks passed"
}

# ============================================================================
# Run Frontend Tests
# ============================================================================

run_frontend_tests() {
    print_header "Running Frontend Performance Tests (Playwright)"
    
    cd "$PROJECT_ROOT/edgequake_webui"
    
    print_info "Installing dependencies..."
    npm install --silent
    
    print_info "Installing Playwright browsers..."
    npx playwright install --with-deps chromium
    
    print_info "Running upload performance tests..."
    if [ "$MODE" = "quick" ]; then
        # Quick mode - run only warmup and light load
        npx playwright test upload-performance-progressive.spec.ts -g "Phase 0|Phase 1" \
            --reporter=list
    else
        # Full test suite
        npx playwright test upload-performance-progressive.spec.ts \
            --reporter=list,html
    fi
    
    local exit_code=$?
    
    if [ $exit_code -eq 0 ]; then
        print_success "Frontend performance tests completed successfully"
        
        # Show report location
        if [ "$MODE" != "quick" ] && [ -f "test-results/upload-performance-report.txt" ]; then
            print_info "Performance report: test-results/upload-performance-report.txt"
            print_info "HTML report: npx playwright show-report"
        fi
    else
        print_error "Frontend performance tests failed with exit code $exit_code"
        return $exit_code
    fi
}

# ============================================================================
# Run Backend Tests
# ============================================================================

run_backend_tests() {
    print_header "Running Backend Performance Tests (Rust)"
    
    cd "$PROJECT_ROOT/edgequake/crates/edgequake-api"
    
    print_info "Building test binary..."
    cargo build --test e2e_upload_performance --quiet
    
    print_info "Running upload performance tests..."
    if [ "$MODE" = "quick" ]; then
        # Quick mode - only warmup phase with reduced load
        print_warning "Quick mode: Running reduced test suite"
        cargo test --test e2e_upload_performance test_progressive_load_performance -- \
            --ignored --nocapture --test-threads=1 2>&1 | head -n 100
    else
        # Full test suite
        cargo test --test e2e_upload_performance -- \
            --ignored --nocapture --test-threads=1
    fi
    
    local exit_code=$?
    
    if [ $exit_code -eq 0 ]; then
        print_success "Backend performance tests completed successfully"
    else
        print_error "Backend performance tests failed with exit code $exit_code"
        return $exit_code
    fi
}

# ============================================================================
# Main Execution
# ============================================================================

main() {
    print_header "Upload Performance Testing - Progressive Load"
    
    echo "Mode: $MODE"
    echo "Start Time: $(date '+%Y-%m-%d %H:%M:%S')"
    echo ""
    
    # Run pre-flight checks
    preflight_checks
    
    # Execute tests based on mode
    case "$MODE" in
        frontend)
            run_frontend_tests
            ;;
        backend)
            run_backend_tests
            ;;
        both)
            run_frontend_tests
            echo ""
            run_backend_tests
            ;;
        quick)
            print_warning "Quick mode: Running reduced test suite"
            run_frontend_tests
            echo ""
            run_backend_tests
            ;;
        *)
            print_error "Invalid mode: $MODE"
            echo "Usage: $0 [frontend|backend|both|quick]"
            exit 1
            ;;
    esac
    
    # Summary
    print_header "Test Execution Complete"
    echo "End Time: $(date '+%Y-%m-%d %H:%M:%S')"
    echo ""
    
    print_success "Performance testing completed!"
    echo ""
    echo "Next steps:"
    echo "  1. Review performance reports in test-results/"
    echo "  2. Compare metrics with baseline expectations"
    echo "  3. Investigate any performance degradation"
    echo "  4. Update baselines if hardware/config changed"
    echo ""
    echo "For detailed documentation, see:"
    echo "  docs/upload-performance-testing.md"
    echo ""
}

# Trap errors and cleanup
trap 'print_error "Test execution failed"; exit 1' ERR

# Run main
main

exit 0
