#!/usr/bin/env bash
# SPEC-017 edgequake-query remediation E2E proof runner
# Usage:
#   ./run_query_e2e.sh                  # Rust contract tests
#   ./run_query_e2e.sh --playwright     # + Playwright query UI (live stack)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../../.." && pwd)"
CRATE="$ROOT/edgequake"
OUT_DIR="$(dirname "$0")"
LOG="$OUT_DIR/001-test-run.log"
SCREENSHOTS="$OUT_DIR/screenshots"

mkdir -p "$SCREENSHOTS"
: > "$LOG"

echo "=== edgequake-query SPEC-017 E2E ===" | tee -a "$LOG"
echo "Date: $(date -u +"%Y-%m-%dT%H:%M:%SZ")" | tee -a "$LOG"

run_test() {
  local label="$1"
  shift
  echo "" | tee -a "$LOG"
  echo "--- $label ---" | tee -a "$LOG"
  (cd "$CRATE" && "$@") 2>&1 | tee -a "$LOG"
}

run_test "Compile workspace" cargo build --workspace

run_test "P0: SPEC-017 query pipeline contract (5)" \
  cargo test -p edgequake-query --test spec017_query_pipeline_contract

run_test "P0: keyword validation tests" \
  cargo test -p edgequake-query --test keyword_validation_tests

run_test "P1: search quality tests" \
  cargo test -p edgequake-query --test search_quality_tests

run_test "P1: chunk ranking + hybrid (e2e_sota_engine)" \
  cargo test -p edgequake-query --test e2e_sota_engine chunk_ranking

run_test "Lib unit tests" cargo test -p edgequake-query --lib

run_test "P0: API query production path contract" \
  cargo test -p edgequake-api --test spec017_query_production_path_contract

run_test "API Ollama lib smoke" \
  cargo test -p edgequake-api --lib ollama

if [[ "${1:-}" == "--playwright" ]]; then
  "$OUT_DIR/run_playwright_proof.sh" 2>&1 | tee -a "$LOG"
fi

echo "" | tee -a "$LOG"
echo "✓ edgequake-query SPEC-017 E2E complete" | tee -a "$LOG"
