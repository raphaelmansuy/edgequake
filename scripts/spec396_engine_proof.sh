#!/usr/bin/env bash
# SPEC-396: prove iw2 DISTINCT ON belt, W3 missing-spine honesty, 21000 cursor-hold.
# Fail-closed: DATABASE_URL required; SKIP/ignored is not a pass.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MEAS="${ROOT}/specs/139-issue-migration/measurements"
mkdir -p "${MEAS}"

if [[ -z "${DATABASE_URL:-}" ]] && [[ -f /tmp/edgequake-db-url ]]; then
  export DATABASE_URL="$(tr -d '\n' </tmp/edgequake-db-url)"
fi

if [[ -z "${DATABASE_URL:-}" ]]; then
  echo "FAIL: DATABASE_URL unset (and /tmp/edgequake-db-url missing) — unfakable e2e cannot run"
  exit 1
fi

export EDGEQUAKE_REQUIRE_POSTGRES_TESTS=1

echo "SPEC-396 migrate engine proof"
echo "DATABASE_URL set: yes"
echo "EDGEQUAKE_REQUIRE_POSTGRES_TESTS=1"

fail_if_skip() {
  local file="$1"
  if grep -E 'SKIP:|skipped' "${file}" | grep -v 'filtered out' >/dev/null; then
    echo "FAIL: skip/ignored in ${file}"
    grep -E 'SKIP:|skipped' "${file}" || true
    exit 1
  fi
}

require_line() {
  local file="$1"
  local pat="$2"
  if ! grep -E "${pat}" "${file}" >/dev/null; then
    echo "FAIL: missing unfakable fact '${pat}' in ${file}"
    exit 1
  fi
}

{
  echo "=== lib contract_spec396_* (no Postgres required for source scans) ==="
  cd "${ROOT}/edgequake"
  cargo test -p edgequake-storage --lib contract_spec396 -- --nocapture
} 2>&1 | tee "${MEAS}/e2e396-lib.txt"
require_line "${MEAS}/e2e396-lib.txt" 'test result: ok\.'

{
  echo "=== E2E-396-01..05 + DROP SQL fail-closed (Postgres required) ==="
  cd "${ROOT}/edgequake"
  cargo test -p edgequake-storage --features postgres --test e2e_spec396_engine \
    -- --nocapture --test-threads=1
} 2>&1 | tee "${MEAS}/e2e396-engine.txt"
fail_if_skip "${MEAS}/e2e396-engine.txt"
# 1 sync contract + 5 async e2e = 6
require_line "${MEAS}/e2e396-engine.txt" 'test result: ok\. 6 passed; 0 failed; 0 ignored'

{
  echo "=== source guards (DISTINCT ON + hold-cursor Err) ==="
  grep -n "DISTINCT ON" \
    "${ROOT}/edgequake/crates/edgequake-storage/src/migration_engine/fleet_embedding_backfill.rs"
  grep -n "hold_cursor\|21000" \
    "${ROOT}/edgequake/crates/edgequake-storage/src/migration_engine/fleet_embedding_backfill.rs" \
    | head -40
} 2>&1 | tee "${MEAS}/e2e396-source-guard.txt"
require_line "${MEAS}/e2e396-source-guard.txt" 'DISTINCT ON'

echo "SPEC-396 proof PASS"
echo "artifacts: ${MEAS}/e2e396-*.txt"
