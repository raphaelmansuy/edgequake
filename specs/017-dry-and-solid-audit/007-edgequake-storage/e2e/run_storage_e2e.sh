#!/usr/bin/env bash
# SPEC-017 edgequake-storage remediation E2E proof runner
# Usage:
#   ./run_storage_e2e.sh                      # Rust contract tests (memory)
#   ./run_storage_e2e.sh --with-postgres      # + postgres contract tests
#   ./run_storage_e2e.sh --playwright         # + Playwright UI proof (live stack)
#   ./run_storage_e2e.sh --with-postgres --playwright
#
# Postgres: run `make db-start` first (or `make dev-bg`). Env auto-loaded from /tmp/edgequake-db-url.
# Without a running edgequake-postgres container, `--with-postgres` will fail (pool timeout).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../../.." && pwd)"
CRATE="$ROOT/edgequake"
WEBUI="$ROOT/edgequake_webui"
OUT_DIR="$(dirname "$0")"
LOG="$OUT_DIR/001-test-run.log"
SCREENSHOTS="$OUT_DIR/screenshots"

mkdir -p "$SCREENSHOTS"
: > "$LOG"

echo "=== edgequake-storage SPEC-017 E2E ===" | tee -a "$LOG"
echo "Date: $(date -u +"%Y-%m-%dT%H:%M:%SZ")" | tee -a "$LOG"

# DRY: one place to align contract tests with make-managed postgres (first-principles: same DB as dev).
spec017_load_postgres_env() {
  if [[ -f /tmp/edgequake-db-url ]]; then
    local parsed
    parsed="$(python3 - <<'PY'
import os, urllib.parse
url = open("/tmp/edgequake-db-url").read().strip()
p = urllib.parse.urlparse(url)
print(p.username or "", p.password or "", p.path.lstrip("/") or "", p.port or 5432)
PY
)"
    read -r _user _pass _db _port <<< "$parsed"
    export POSTGRES_USER="${POSTGRES_USER:-${_user:-edgequake}}"
    export POSTGRES_PASSWORD="${POSTGRES_PASSWORD:-$_pass}"
    export POSTGRES_DB="${POSTGRES_DB:-${_db:-edgequake}}"
    export POSTGRES_PORT="${POSTGRES_PORT:-$_port}"
    echo "Postgres env from /tmp/edgequake-db-url (port ${POSTGRES_PORT})" | tee -a "$LOG"
  fi
  export POSTGRES_USER="${POSTGRES_USER:-edgequake}"
  export POSTGRES_DB="${POSTGRES_DB:-edgequake}"
  export POSTGRES_HOST="${POSTGRES_HOST:-localhost}"
  export POSTGRES_PORT="${POSTGRES_PORT:-5432}"
}

run_test() {
  local label="$1"
  shift
  echo "" | tee -a "$LOG"
  echo "--- $label ---" | tee -a "$LOG"
  (cd "$CRATE" && "$@") 2>&1 | tee -a "$LOG"
}

run_test "Compile workspace" cargo build --workspace

run_test "P0: graph workspace contract (memory)" \
  cargo test -p edgequake-storage --test storage_backend_contract

run_test "P0: memory graph workspace parity" \
  cargo test -p edgequake-storage --test memory_graph_workspace_parity

run_test "P1: conversation storage contract (memory)" \
  cargo test -p edgequake-storage --test conversation_backend_contract

run_test "P1: MetadataFilter shared predicate" \
  cargo test -p edgequake-storage --test metadata_filter_predicate

run_test "P1: memory PDF + conversation parity" \
  cargo test -p edgequake-storage --test memory_subsystem_parity

run_test "P1: MetadataFilter SQL builder (lib)" \
  cargo test -p edgequake-storage metadata_filter_sql --lib

run_test "Lib unit tests" cargo test -p edgequake-storage --lib

run_test "P2: cross-backend E2E contracts (memory)" \
  cargo test -p edgequake-storage --test backend_e2e_contract

run_test "P2-22: e2e_storage_backends (memory)" \
  cargo test -p edgequake-storage --test e2e_storage_backends

run_test "P3: GraphStorage ISP capability contract" \
  cargo test -p edgequake-storage --test graph_isp_contract

run_test "P3: conversation HTTP contract (memory API)" \
  cargo test -p edgequake-api --test spec017_conversation_http_contract

run_test "P1-12: postgres graph helpers (lib)" \
  cargo test -p edgequake-storage --features postgres --lib test_dollar_quote

run_test "P1: MetadataFilter DRY contract" \
  cargo test -p edgequake-storage --test metadata_filter_dry_contract

run_test "P1: API memory conversation service contract" \
  cargo test -p edgequake-api --features postgres --lib test_memory_conversation_service_roundtrip

if [[ "${1:-}" == "--with-postgres" ]] || [[ "${2:-}" == "--with-postgres" ]]; then
  if [[ ! -f /tmp/edgequake-db-url ]]; then
    echo "→ No edgequake DB URL file — starting postgres via make db-start" | tee -a "$LOG"
    (cd "$ROOT" && make db-start --no-print-directory)
  fi
  spec017_load_postgres_env
  if [[ -z "${POSTGRES_PASSWORD:-}" ]]; then
    echo "POSTGRES_PASSWORD required for postgres contract tests (set env or run make postgres-start)" | tee -a "$LOG"
    exit 1
  fi
  run_test "P0: postgres graph workspace contract" \
    cargo test -p edgequake-storage --test storage_backend_contract --features postgres
  run_test "P2: cross-backend E2E contracts (postgres)" \
    cargo test -p edgequake-storage --test backend_e2e_contract --features postgres
  run_test "P3: postgres GraphStorage ISP contract" \
    cargo test -p edgequake-storage --test graph_isp_contract --features postgres
  run_test "P1: postgres conversation contract" \
    cargo test -p edgequake-storage --test conversation_backend_contract --features postgres
fi

if [[ "${1:-}" == "--playwright" ]] || [[ "${2:-}" == "--playwright" ]]; then
  echo "" | tee -a "$LOG"
  echo "--- Playwright UI proofs (dashboard + conversations) ---" | tee -a "$LOG"
  "$(dirname "$0")/run_playwright_proof.sh" 2>&1 | tee -a "$LOG"
fi

echo "" | tee -a "$LOG"
echo "=== E2E PASSED ===" | tee -a "$LOG"
echo "Log: $LOG" | tee -a "$LOG"
echo "Playwright PNGs (when run): $SCREENSHOTS/*.png" | tee -a "$LOG"
