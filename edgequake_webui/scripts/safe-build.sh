#!/bin/bash
# ==============================================================================
# Safe Build Script for EdgeQuake WebUI
# 
# This script prevents CPU overload during builds by:
# 1. Cleaning caches before build
# 2. Using nice to lower priority
# 3. Setting memory limits for Node.js
# 4. Running in background with timeout protection
# 5. Monitoring resource usage
#
# Usage:
#   ./scripts/safe-build.sh          # Normal build
#   ./scripts/safe-build.sh --watch  # Dev mode with monitoring
# ==============================================================================

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
LOG_FILE="$PROJECT_DIR/build.log"
MAX_BUILD_TIME=${BUILD_TIMEOUT:-300}  # 5 minutes default

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log() {
    echo -e "${BLUE}[$(date '+%H:%M:%S')]${NC} $1"
}

success() {
    echo -e "${GREEN}[$(date '+%H:%M:%S')] ✓${NC} $1"
}

warn() {
    echo -e "${YELLOW}[$(date '+%H:%M:%S')] ⚠${NC} $1"
}

error() {
    echo -e "${RED}[$(date '+%H:%M:%S')] ✗${NC} $1"
}

cleanup() {
    log "Cleaning caches..."
    rm -rf "$PROJECT_DIR/.next" 2>/dev/null || true
    rm -rf "$PROJECT_DIR/node_modules/.cache" 2>/dev/null || true
    rm -f "$PROJECT_DIR/tsconfig.tsbuildinfo" 2>/dev/null || true
    success "Caches cleaned"
}

check_dependencies() {
    log "Checking dependencies..."
    
    if ! command -v node &> /dev/null; then
        error "Node.js not found"
        exit 1
    fi
    
    if [ ! -d "$PROJECT_DIR/node_modules" ]; then
        warn "node_modules not found, installing..."
        cd "$PROJECT_DIR"
        npm install
    fi
    
    success "Dependencies ready"
}

run_typecheck() {
    log "Running TypeScript type check..."
    cd "$PROJECT_DIR"
    
    # Run tsc with timeout
    if timeout 60 npx tsc --noEmit; then
        success "TypeScript check passed"
        return 0
    else
        error "TypeScript check failed"
        return 1
    fi
}

run_build() {
    log "Starting Next.js build with resource limits..."
    cd "$PROJECT_DIR"
    
    # Set Node.js memory limit (4GB max)
    export NODE_OPTIONS="--max-old-space-size=4096"
    
    # Run with nice (low priority) and timeout
    if nice -n 10 timeout $MAX_BUILD_TIME npx next build 2>&1 | tee "$LOG_FILE"; then
        success "Build completed successfully!"
        return 0
    else
        EXIT_CODE=$?
        if [ $EXIT_CODE -eq 124 ]; then
            error "Build timed out after ${MAX_BUILD_TIME}s"
        else
            error "Build failed with exit code $EXIT_CODE"
        fi
        return $EXIT_CODE
    fi
}

run_build_monitored() {
    log "Starting monitored build..."
    
    # Start monitoring in background
    (
        while true; do
            sleep 5
            # Check for high CPU usage from node processes
            NODE_CPU=$(ps aux | grep -E "node|next" | grep -v grep | awk '{sum += $3} END {print sum}')
            if [ -n "$NODE_CPU" ] && [ "${NODE_CPU%.*}" -gt 200 ]; then
                warn "High CPU detected: ${NODE_CPU}%"
            fi
        done
    ) &
    MONITOR_PID=$!
    
    # Run the actual build
    run_build
    BUILD_EXIT=$?
    
    # Stop monitoring
    kill $MONITOR_PID 2>/dev/null || true
    
    return $BUILD_EXIT
}

# Main execution
main() {
    echo ""
    echo "======================================"
    echo "  EdgeQuake WebUI Safe Build Script"
    echo "======================================"
    echo ""
    
    cd "$PROJECT_DIR"
    
    case "${1:-}" in
        --clean)
            cleanup
            ;;
        --check)
            check_dependencies
            run_typecheck
            ;;
        --watch)
            check_dependencies
            cleanup
            run_build_monitored
            ;;
        *)
            check_dependencies
            cleanup
            run_typecheck
            run_build
            ;;
    esac
}

main "$@"
