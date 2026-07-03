#!/usr/bin/env bash
# ============================================================================
# SPEC-041 — Migration 078 JSON Operator Fix — E2E Test Runner
# Issue: https://github.com/raphaelmansuy/edgequake/issues/273
#
# Usage:
#   bash specs/041-fix-migration/e2e/run_all.sh
#   DB_URL='postgres://...' bash specs/041-fix-migration/e2e/run_all.sh
#
# Evidence: specs/041-fix-migration/e2e/evidence/
# Exit: 0 = all pass, 1 = failure
# ============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"
EVIDENCE_DIR="${SCRIPT_DIR}/evidence"
M078="${REPO_ROOT}/edgequake/migrations/078_age_child_workspace_stats.sql"

mkdir -p "${EVIDENCE_DIR}"

: "${DB_URL:=postgresql://edgequake:edgequake_secret@localhost:5432/edgequake}"

PASS=0
FAIL=0
SKIP=0
RESULTS=()
SUMMARY="${EVIDENCE_DIR}/run_all_summary.txt"

ts() { date -u '+%Y-%m-%dT%H:%M:%SZ'; }

record() {
  local status="$1" name="$2"
  RESULTS+=("${status}|${name}")
  case "${status}" in
    PASS) ((PASS++)) || true ;;
    FAIL) ((FAIL++)) || true ;;
    SKIP) ((SKIP++)) || true ;;
  esac
}

psql_cmd() {
  if docker ps --format '{{.Names}}' 2>/dev/null | grep -qx 'edgequake-postgres'; then
    local use_stdin=false
    local sql_file=""
    local -a args=()
    for arg in "$@"; do
      if [[ "$use_stdin" == true ]]; then
        sql_file="$arg"
        use_stdin=false
        continue
      fi
      if [[ "$arg" == "-f" ]]; then
        use_stdin=true
        continue
      fi
      args+=("$arg")
    done
    if [[ -n "$sql_file" ]]; then
      if ((${#args[@]})); then
        docker exec -i edgequake-postgres psql -U edgequake -d edgequake -v ON_ERROR_STOP=1 "${args[@]}" < "$sql_file"
      else
        docker exec -i edgequake-postgres psql -U edgequake -d edgequake -v ON_ERROR_STOP=1 < "$sql_file"
      fi
    else
      docker exec edgequake-postgres psql -U edgequake -d edgequake -v ON_ERROR_STOP=1 "$@"
    fi
  elif command -v psql >/dev/null 2>&1; then
    psql "${DB_URL}" -v ON_ERROR_STOP=1 "$@"
  else
    echo "ERROR: no psql and edgequake-postgres container not running" >&2
    return 1
  fi
}

run_group() {
  local name="$1"
  shift
  echo ""
  echo "--- ${name} ---"
  "$@"
}

echo "============================================================"
echo "SPEC-041 E2E — Migration 078 operator fix — $(ts)"
echo "REPO: ${REPO_ROOT}"
echo "DB:   ${DB_URL%%@*}@***"
echo "============================================================"

# ============================================================
# GROUP 1: Offline static gates (no DB)
# ============================================================
run_group "GROUP 1: Static operator grep (REQ-041-04)" \
  bash "${SCRIPT_DIR}/verify_no_invalid_json_operators.sh" \
  && record PASS "G1 no ->>> in migrations" \
  || record FAIL "G1 no ->>> in migrations"

run_group "GROUP 1b: Checksum lock" \
  bash "${REPO_ROOT}/scripts/check_migration_checksums.sh" \
  && record PASS "G1b checksums.lock" \
  || record FAIL "G1b checksums.lock"

# ============================================================
# GROUP 2: DB — index definition audit on default graph
# ============================================================
if psql_cmd -c "SELECT 1" >/dev/null 2>&1; then
  AGE=$(psql_cmd -t -A -c "SELECT COUNT(*) FROM pg_extension WHERE extname='age';" || echo "0")
  if [[ "${AGE}" == "1" ]]; then
    run_group "GROUP 2: Index definition audit (REQ-041-07)" \
      psql_cmd -f "${SCRIPT_DIR}/verify_m078_indexes.sql" \
      && record PASS "G2 indexdef audit" \
      || record FAIL "G2 indexdef audit"
  else
    echo "SKIP GROUP 2: AGE not installed"
    record SKIP "G2 indexdef audit (no AGE)"
  fi
else
  echo "SKIP GROUP 2-4: database unavailable"
  record SKIP "G2-G4 database unavailable"
fi

# ============================================================
# GROUP 3: M078 apply on isolated test graph (prod failure path)
# ============================================================
if psql_cmd -c "SELECT 1" >/dev/null 2>&1; then
  AGE=$(psql_cmd -t -A -c "SELECT COUNT(*) FROM pg_extension WHERE extname='age';" || echo "0")
  if [[ "${AGE}" == "1" ]]; then
    LOG="${EVIDENCE_DIR}/g3_apply_m078.log"
    {
      echo "== setup =="
      psql_cmd -f "${SCRIPT_DIR}/apply_m078_setup.sql"
      echo "== first apply =="
      psql_cmd -f "${M078}"
      echo "== verify =="
      psql_cmd -f "${SCRIPT_DIR}/apply_m078_verify.sql"
      echo "== idempotent re-apply =="
      psql_cmd -f "${M078}"
      echo "== verify after re-apply =="
      psql_cmd -f "${SCRIPT_DIR}/apply_m078_verify.sql"
    } >"${LOG}" 2>&1 && {
      echo "  ✓ G3 apply log: ${LOG}"
      tail -8 "${LOG}"
      record PASS "G3 M078 apply on Node graph"
    } || {
      echo "  ✗ G3 failed — see ${LOG}"
      tail -20 "${LOG}"
      record FAIL "G3 M078 apply on Node graph"
    }
  else
    record SKIP "G3 M078 apply (no AGE)"
  fi
fi

# ============================================================
# GROUP 4: Concurrent script syntax (offline file grep)
# ============================================================
run_group "GROUP 4: Concurrent script operator check" \
  bash -c "grep -Fe \"->>''workspace_id''\" '${REPO_ROOT}/edgequake/migrations/support/078/concurrent.sql' >/dev/null && ! grep -Fe '->>>' '${REPO_ROOT}/edgequake/migrations/support/078/concurrent.sql' >/dev/null" \
  && record PASS "G4 concurrent.sql operators" \
  || record FAIL "G4 concurrent.sql operators"

# ============================================================
# GROUP 5: M078 file content law
# ============================================================
run_group "GROUP 5: M078 matches graph_lifecycle SSOT pattern" \
  bash -c "grep -Fe \"->>''workspace_id''\" '${M078}' >/dev/null && grep -Fe \"->>''tenant_id''\" '${M078}' >/dev/null && ! grep -Fe '->>>' '${M078}' >/dev/null" \
  && record PASS "G5 M078 SSOT alignment" \
  || record FAIL "G5 M078 SSOT alignment"

# ============================================================
# Summary
# ============================================================
{
  echo "SPEC-041 E2E Summary — $(ts)"
  echo "PASS=${PASS} FAIL=${FAIL} SKIP=${SKIP}"
  echo ""
  for r in "${RESULTS[@]}"; do echo "$r"; done
} | tee "${SUMMARY}"

echo ""
echo "Evidence: ${EVIDENCE_DIR}/"
echo "============================================================"

if [[ "${FAIL}" -gt 0 ]]; then
  echo "FAIL: ${FAIL} test(s) failed"
  exit 1
fi

echo "PASS: all ${PASS} tests passed (${SKIP} skipped)"
exit 0
