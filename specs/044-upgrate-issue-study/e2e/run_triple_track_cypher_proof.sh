#!/usr/bin/env bash
# SPEC-044 — Triple-track Cypher parameter battle test (PG16 / PG17 / PG18).
#
# Grounded by:
#   https://age.apache.org/age-manual/master/advanced/prepared_statements.html
#   https://age.apache.org/download/
#   edgequake/docker/extension-pins.sh
#
# Usage:
#   ./specs/044-upgrate-issue-study/e2e/run_triple_track_cypher_proof.sh [pg16|pg17|pg18|all]
#
# Env:
#   SKIP_IMAGE_BUILD=1   — skip make postgres-image-build*
#   SKIP_RUST_TESTS=1    — SQL probes only (BT-044-TT-01–07, 12)
#   PGPASSWORD         — default edgequake_secret
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
SPEC_DIR="$ROOT/specs/044-upgrate-issue-study"
E2E_DIR="$SPEC_DIR/e2e"
PINS="$ROOT/edgequake/docker/extension-pins.sh"
SQL_CONTRACT="$E2E_DIR/sql/cypher_param_contract.sql"
REPORT_DIR="$E2E_DIR/reports"
CYPHER_EXEC="$ROOT/edgequake/crates/edgequake-storage/src/adapters/postgres/graph/helpers/cypher_exec.rs"

PROFILE="${1:-all}"
PGPASSWORD="${POSTGRES_PASSWORD:-edgequake_secret}"
export PGPASSWORD

RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

mkdir -p "$REPORT_DIR"

log() { echo -e "${BLUE}$*${NC}"; }
pass() { echo -e "${GREEN}PASS${NC}: $*"; }
fail() { echo -e "${RED}FAIL${NC}: $*"; exit 1; }

if [ "${SKIP_IMAGE_BUILD:-0}" != "1" ]; then
  log "→ Building postgres images (pg16 + pg17 + pg18)..."
  make -C "$ROOT" postgres-image-build postgres-image-build-pg17 postgres-image-build-pg18 --no-print-directory
fi

log "→ BT-044-TT-00: extension pin SSOT"
chmod +x "$ROOT/scripts/check_extension_pins.sh"
"$ROOT/scripts/check_extension_pins.sh" "${PROFILE/all/all}"
pass "extension-pins.sh"

log "→ BT-044-TT-11: source contract (no inline agtype third arg)"
if grep -q "params_lit}'::agtype" "$CYPHER_EXEC" 2>/dev/null; then
  fail "cypher_exec.rs still uses inline ::agtype (P0a not applied)"
fi
pass "cypher_exec.rs source contract"

run_profile() {
  local profile="$1"
  local report="$REPORT_DIR/${profile}-cypher-report.txt"
  local container="edgequake-spec044-${profile}-$$"
  local host_port

  docker rm -f "edgequake-spec044-${profile}" >/dev/null 2>&1 || true

  # shellcheck source=/dev/null
  EQ_POSTGRES_PROFILE="$profile" source "$PINS"

  local image="$EQ_POSTGRES_IMAGE_TAG"
  local pg_major="$EQ_POSTGRES_MAJOR"
  local age_min="$EQ_AGE_MIN"

  log ""
  log "========== SPEC-044 TRIPLE-TRACK: $profile (PG$pg_major, AGE>=$age_min) =========="
  log "    Image: $image"
  log "    Report: $report"

  if ! docker image inspect "$image" >/dev/null 2>&1; then
    case "$profile" in
      pg16) fail "Image $image missing — run: make postgres-image-build" ;;
      pg17) fail "Image $image missing — run: make postgres-image-build-pg17" ;;
      pg18) fail "Image $image missing — run: make postgres-image-build-pg18" ;;
    esac
  fi

  host_port=$((55440 + pg_major))

  cleanup() { docker rm -f "$container" >/dev/null 2>&1 || true; }
  trap cleanup RETURN

  if ! docker run -d --name "$container" \
    -p "${host_port}:5432" \
    -e POSTGRES_USER=edgequake \
    -e POSTGRES_PASSWORD="$PGPASSWORD" \
    -e POSTGRES_DB=edgequake \
    "$image" >/dev/null 2>&1; then
    # Fallback: ephemeral host port when fixed mapping is taken.
    host_port="$(python3 -c 'import socket; s=socket.socket(); s.bind(("",0)); print(s.getsockname()[1]); s.close()')"
    docker run -d --name "$container" \
      -p "${host_port}:5432" \
      -e POSTGRES_USER=edgequake \
      -e POSTGRES_PASSWORD="$PGPASSWORD" \
      -e POSTGRES_DB=edgequake \
      "$image" >/dev/null
  fi

  for i in $(seq 1 90); do
    if docker exec -e PGPASSWORD="$PGPASSWORD" "$container" \
      psql -U edgequake -d edgequake -c 'SELECT 1' >/dev/null 2>&1; then
      break
    fi
    if [ "$i" -eq 90 ]; then
      docker logs "$container" 2>&1 | tail -30
      fail "$profile: postgres not ready"
    fi
    sleep 1
  done

  log "→ $profile: bootstrap extensions (vector + age)"
  docker exec -i -e PGPASSWORD="$PGPASSWORD" "$container" \
    psql -U edgequake -d edgequake -v ON_ERROR_STOP=1 <<'EOSQL'
CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE EXTENSION IF NOT EXISTS btree_gin;
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
DO $$ BEGIN
  CREATE EXTENSION IF NOT EXISTS age;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;
EOSQL

  local sql_contract_rendered
  sql_contract_rendered="$(sed \
    -e "s/@expected_pg_major@/${pg_major}/g" \
    -e "s/@expected_age_min@/${age_min}/g" \
    "$SQL_CONTRACT")"

  {
    echo "SPEC-044 triple-track report — profile=$profile"
    echo "date=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "image=$image"
    echo "pg_major=$pg_major age_min=$age_min"
    echo "host_port=$host_port"
    echo ""
    docker exec -e PGPASSWORD="$PGPASSWORD" "$container" \
      psql -U edgequake -d edgequake -c \
      "SELECT version(); SELECT extname, extversion FROM pg_extension WHERE extname IN ('age','vector') ORDER BY extname;"
    echo ""
    echo "=== cypher_param_contract.sql ==="
    echo "$sql_contract_rendered" | docker exec -i -e PGPASSWORD="$PGPASSWORD" "$container" \
      psql -U edgequake -d edgequake -v ON_ERROR_STOP=1 -f -
  } 2>&1 | tee "$report"
  contract_status="${PIPESTATUS[0]}"
  if [ "${contract_status:-1}" -ne 0 ]; then
    fail "$profile: cypher_param_contract.sql failed (exit ${contract_status})"
  fi

  if ! grep -q "BT-044-TT-04 PASS" "$report"; then
    fail "$profile: BT-044-TT-04 negative probe missing"
  fi
  if ! grep -q "BT-044-TT-05 PASS" "$report"; then
    fail "$profile: BT-044-TT-05 delete probe missing"
  fi
  pass "$profile SQL probes (BT-044-TT-01–07, 12)"

  if [ "${SKIP_RUST_TESTS:-0}" = "1" ]; then
    log "SKIP Rust tests (SKIP_RUST_TESTS=1)"
    return 0
  fi

  export POSTGRES_USER=edgequake
  export POSTGRES_PASSWORD="$PGPASSWORD"
  export POSTGRES_DB=edgequake
  export POSTGRES_HOST=127.0.0.1
  export POSTGRES_PORT="$host_port"

  log "→ $profile: BT-044-TT-08 spec022 postgres Cypher CRUD"
  cd "$ROOT/edgequake"
  if cargo test -p edgequake-storage --features postgres \
    --test spec022_cypher_prepared_postgres spec022_postgres_cypher -- --nocapture 2>&1 | tee -a "$report"; then
    if grep -q "SKIP spec022_postgres_cypher_prepared" "$report"; then
      fail "$profile: spec022 integration SKIPPED"
    fi
  else
    fail "$profile: spec022_cypher_prepared_postgres"
  fi
  pass "$profile spec022"

  log "→ $profile: BT-044-TT-08b spec044 compensation graph rollback"
  if ! cargo test -p edgequake-storage --features postgres \
    --test spec044_compensation_postgres spec044_compensation -- --nocapture 2>&1 | tee -a "$report"; then
    fail "$profile: spec044_compensation_postgres"
  fi
  if grep -q "SKIP spec044_compensation" "$report"; then
    fail "$profile: spec044 compensation SKIPPED"
  fi
  pass "$profile spec044 compensation"

  log "→ $profile: BT-044-TT-09 postgres_integration graph CRUD"
  if ! cargo test -p edgequake-storage --features postgres \
    --test postgres_integration test_postgres_age_basic_operations -- --nocapture 2>&1 | tee -a "$report"; then
    fail "$profile: test_postgres_age_basic_operations"
  fi
  pass "$profile postgres_integration CRUD"

  log "→ $profile: BT-044-TT-10 storage_backend_contract"
  if ! cargo test -p edgequake-storage --features postgres \
    --test storage_backend_contract postgres_backend_graph_batch_upsert_contract -- --nocapture 2>&1 | tee -a "$report"; then
    fail "$profile: storage_backend_contract"
  fi
  pass "$profile storage_backend_contract"

  echo "✓ SPEC-044 TRIPLE-TRACK PASSED: $profile" | tee -a "$report"
}

case "$PROFILE" in
  pg16) run_profile pg16 ;;
  pg17) run_profile pg17 ;;
  pg18) run_profile pg18 ;;
  all)
    run_profile pg16
    run_profile pg17
    run_profile pg18
    ;;
  *)
    echo "Usage: $0 [pg16|pg17|pg18|all]"
    exit 1
    ;;
esac

echo ""
echo -e "${GREEN}== SPEC-044 triple-track battle test COMPLETE ($PROFILE) ==${NC}"
echo "Reports: $REPORT_DIR/"
