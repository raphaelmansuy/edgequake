#!/usr/bin/env bash
# SPEC-017 edgequake-api remediation E2E proof runner
# Usage:
#   ./run_api_e2e.sh                  # Rust contract + integration tests
#   ./run_api_e2e.sh --playwright     # + Playwright UI proof (live stack)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../../../.." && pwd)"
CRATE="$ROOT/edgequake"
OUT_DIR="$(dirname "$0")"
LOG="$OUT_DIR/001-test-run.log"
SCREENSHOTS="$OUT_DIR/screenshots"

mkdir -p "$SCREENSHOTS"
: > "$LOG"

echo "=== edgequake-api SPEC-017 E2E ===" | tee -a "$LOG"
echo "Date: $(date -u +"%Y-%m-%dT%H:%M:%SZ")" | tee -a "$LOG"

run_test() {
  local label="$1"
  shift
  echo "" | tee -a "$LOG"
  echo "--- $label ---" | tee -a "$LOG"
  (cd "$CRATE" && "$@") 2>&1 | tee -a "$LOG"
}

run_test "Compile workspace" cargo build --workspace

run_test "P0: API DRY/SOLID source contract (8)" \
  cargo test -p edgequake-api --test spec017_api_contract

run_test "P0: query production path (SOTA only)" \
  cargo test -p edgequake-api --test spec017_query_production_path_contract

run_test "P0: workspace pipeline integration" \
  cargo test -p edgequake-api --test e2e_workspace_pipeline_integration

run_test "P1: query/chat routing parity" \
  cargo test -p edgequake-api --test e2e_query_routing_parity

run_test "P1: workspace provider ingestion" \
  cargo test -p edgequake-api --test e2e_workspace_provider_ingestion

run_test "P1: query error mapping smoke" \
  cargo test -p edgequake-api --test e2e_query test_query_partial_llm_override_returns_bad_request

run_test "P3: conversation HTTP contract" \
  cargo test -p edgequake-api --test spec017_conversation_http_contract

run_test "Lib unit tests (edgequake-api)" \
  cargo test -p edgequake-api --lib

run_test "Clippy (workspace)" \
  cargo clippy --workspace --all-targets -- -D warnings

run_test "Fmt check (workspace)" \
  cargo fmt --all -- --check

if [[ "${1:-}" == "--playwright" ]]; then
  echo "" | tee -a "$LOG"
  echo "--- Playwright API UI proof ---" | tee -a "$LOG"
  "$(dirname "$0")/run_playwright_proof.sh" 2>&1 | tee -a "$LOG"
fi

echo "" | tee -a "$LOG"
echo "✓ edgequake-api SPEC-017 E2E complete — log: $LOG" | tee -a "$LOG"
