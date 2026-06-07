#!/usr/bin/env bash
# SPEC-017 edgequake-pipeline remediation E2E proof runner
# Usage:
#   ./run_pipeline_e2e.sh              # Rust contract + lib tests
#   ./run_pipeline_e2e.sh --playwright # + Playwright documents UI proof
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../../.." && pwd)"
CRATE="$ROOT/edgequake"
OUT_DIR="$(dirname "$0")"
LOG="$OUT_DIR/001-test-run.log"
SCREENSHOTS="$OUT_DIR/screenshots"

mkdir -p "$SCREENSHOTS"
: > "$LOG"

echo "=== edgequake-pipeline SPEC-017 E2E ===" | tee -a "$LOG"
echo "Date: $(date -u +"%Y-%m-%dT%H:%M:%SZ")" | tee -a "$LOG"

run_test() {
  local label="$1"
  shift
  echo "" | tee -a "$LOG"
  echo "--- $label ---" | tee -a "$LOG"
  (cd "$CRATE" && "$@") 2>&1 | tee -a "$LOG"
}

run_test "Compile workspace" cargo build --workspace

run_test "Full pipeline integration (spec017)" \
  cargo test -p edgequake-pipeline --test spec017_full_pipeline_integration

run_test "P0: pipeline contract (spec017)" \
  cargo test -p edgequake-pipeline --test spec017_pipeline_contract

run_test "P0: normalizer unit tests" \
  cargo test -p edgequake-pipeline normalize_entity_name

run_test "Lib tests (edgequake-pipeline)" \
  cargo test -p edgequake-pipeline --lib

run_test "Clippy (edgequake-pipeline)" \
  cargo clippy -p edgequake-pipeline --all-targets -- -D warnings

run_test "Fmt check (edgequake-pipeline)" \
  cargo fmt --all -- --check

if [[ "${1:-}" == "--playwright" ]]; then
  echo "" | tee -a "$LOG"
  echo "--- Playwright documents UI proof ---" | tee -a "$LOG"
  "$(dirname "$0")/run_playwright_proof.sh" 2>&1 | tee -a "$LOG"
fi

echo "" | tee -a "$LOG"
echo "✓ Pipeline SPEC-017 E2E complete — log: $LOG" | tee -a "$LOG"
