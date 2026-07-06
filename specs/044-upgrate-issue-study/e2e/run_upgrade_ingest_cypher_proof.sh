#!/usr/bin/env bash
# SPEC-044: Post-upgrade ingest + Cypher compensation battle test (single-host).
#
# For PG16/PG17/PG18 matrix use the triple-track runner (release gate):
#   make spec044-battle-test-all
#   ./specs/044-upgrate-issue-study/e2e/run_triple_track_cypher_proof.sh all
#
# This script: dev/single Postgres instance (e.g. make postgres-start).
#   BT-044-02/06 — parameterized Cypher CRUD (spec022)
#   BT-044-18     — compensation delete_node path (storage lib tests)
#   BT-044-27/34  — post-upgrade health SQL
#   BT-044-32     — ingest persister contract (when API+DB available)
#
# Usage:
#   export DATABASE_URL=postgres://edgequake:edgequake@localhost/edgequake
#   make postgres-start   # if needed
#   ./specs/044-upgrate-issue-study/e2e/run_upgrade_ingest_cypher_proof.sh
#
# Optional:
#   SKIP_HEALTH_SQL=1     — skip psql health gates
#   SKIP_INGEST_E2E=1     — skip API ingest (storage-only mode)
#   EDGEQUAKE_API_URL=... — for ingest E2E (default http://localhost:8080)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
SPEC_DIR="$ROOT/specs/044-upgrate-issue-study"
cd "$ROOT/edgequake"

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

pass() { echo -e "${GREEN}PASS${NC}: $*"; }
fail() { echo -e "${RED}FAIL${NC}: $*"; exit 1; }

echo "== SPEC-044: upgrade ingest Cypher battle test =="
echo "   Root: $ROOT"
echo "   Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"

# ── Gate 0: Postgres connectivity ───────────────────────────────────────────
# spec022 uses POSTGRES_PASSWORD (see tests/support/postgres_test_config.rs).
# Accept DATABASE_URL and derive discrete vars when unset.
if [[ -z "${POSTGRES_PASSWORD:-}" && -n "${DATABASE_URL:-}" ]]; then
  # postgres://user:pass@host:port/db
  if [[ "$DATABASE_URL" =~ postgres://([^:]+):([^@]+)@([^:/]+):?([0-9]*)/([^?]+) ]]; then
    export POSTGRES_USER="${BASH_REMATCH[1]}"
    export POSTGRES_PASSWORD="${BASH_REMATCH[2]}"
    export POSTGRES_HOST="${BASH_REMATCH[3]}"
    [[ -n "${BASH_REMATCH[4]}" ]] && export POSTGRES_PORT="${BASH_REMATCH[4]}"
    export POSTGRES_DB="${BASH_REMATCH[5]}"
  fi
fi
if [[ -z "${POSTGRES_PASSWORD:-}" ]]; then
  echo "POSTGRES_PASSWORD or DATABASE_URL required."
  echo "Example: export DATABASE_URL=postgres://edgequake:edgequake@localhost/edgequake"
  echo "Or:      export POSTGRES_PASSWORD=edgequake POSTGRES_USER=edgequake"
  echo "Start DB: make postgres-start (from repo root)"
  fail "Postgres credentials required for battle test"
fi
export DATABASE_URL="${DATABASE_URL:-postgres://${POSTGRES_USER:-edgequake}:${POSTGRES_PASSWORD}@${POSTGRES_HOST:-localhost}:${POSTGRES_PORT:-5432}/${POSTGRES_DB:-edgequake}}"
pass "Postgres credentials configured"

# ── Gate 1: spec022 parameterized Cypher (BT-044-02, BT-044-06) ─────────────
echo ""
echo "== BT-044-02/06: spec022 parameterized Cypher CRUD =="
if cargo test -p edgequake-storage --features postgres \
  --test spec022_cypher_prepared_postgres spec022_postgres_cypher -- --nocapture 2>&1 | tee /tmp/spec044-spec022.log; then
  if grep -q "SKIP spec022_postgres_cypher_prepared" /tmp/spec044-spec022.log; then
    fail "spec022 postgres integration was SKIPPED — battle test requires live Postgres"
  fi
  pass "spec022_cypher_prepared_postgres"
else
  fail "spec022_cypher_prepared_postgres — fix cypher_*_bound before release"
fi

# ── Gate 2: Source contract (BT-044-04) ──────────────────────────────────────
echo ""
echo "== BT-044-04: no inline agtype literal in bound Cypher builder =="
CYPHER_EXEC="$ROOT/edgequake/crates/edgequake-storage/src/adapters/postgres/graph/helpers/cypher_exec.rs"
if grep -q "params_lit}'::agtype" "$CYPHER_EXEC" 2>/dev/null; then
  fail "cypher_exec.rs still uses inline '::agtype third arg (SPEC-044 P0 not applied)"
fi
if ! grep -q 'cypher_query_bound' "$CYPHER_EXEC"; then
  fail "cypher_query_bound missing"
fi
pass "cypher_exec.rs contract (no inline agtype third arg)"

# ── Gate 3: Compensation unit tests (BT-044-18, BT-044-22, BT-044-23) ───────
echo ""
echo "== BT-044-18/22/23: compensation saga tests =="
cargo test -p edgequake-storage --lib compensation -- --nocapture
pass "compensation unit tests"

# ── Gate 4: Merger tests (BT-044-32) ─────────────────────────────────────────
echo ""
echo "== BT-044-32: merger batch tests =="
cargo test -p edgequake-pipeline --lib merger -- --nocapture
pass "merger unit tests"

# ── Gate 5: Post-upgrade health SQL (BT-044-27, BT-044-34) ───────────────────
if [[ "${SKIP_HEALTH_SQL:-0}" != "1" ]]; then
  echo ""
  echo "== BT-044-27/34: post_upgrade_health.sql =="
  if command -v psql >/dev/null 2>&1; then
    psql "$DATABASE_URL" -v ON_ERROR_STOP=1 \
      -f "$SPEC_DIR/e2e/sql/post_upgrade_health.sql"
    pass "post_upgrade_health.sql"
  else
    echo "WARN: psql not found — skipping health SQL"
  fi
else
  echo "SKIP: health SQL (SKIP_HEALTH_SQL=1)"
fi

# ── Gate 6: Optional API ingest E2E ──────────────────────────────────────────
if [[ "${SKIP_INGEST_E2E:-0}" != "1" ]]; then
  API_URL="${EDGEQUAKE_API_URL:-http://localhost:8080}"
  echo ""
  echo "== BT-044-32: API health + ingest contract (optional) =="
  if curl -sf "${API_URL}/health" >/dev/null 2>&1; then
    curl -sf "${API_URL}/health" | python3 -m json.tool | head -15
    if cargo test -p edgequake-api --test e2e_spec021_ingestion_persister -- --nocapture 2>&1 | tail -20; then
      pass "e2e_spec021_ingestion_persister"
    else
      echo "WARN: ingestion persister E2E failed — check backend logs"
    fi
  else
    echo "SKIP: API not reachable at ${API_URL} (set EDGEQUAKE_API_URL or start make dev-bg)"
  fi
else
  echo "SKIP: ingest E2E (SKIP_INGEST_E2E=1)"
fi

# ── Gate 7: Static spec022 source contracts ───────────────────────────────────
echo ""
echo "== BT-044: spec022 static source contracts =="
cargo test -p edgequake-storage --test spec022_cypher_prepared_postgres \
  spec022_nodes_ops spec022_edges_ops spec022_cypher_exec -- --nocapture
pass "spec022 static contracts"

echo ""
echo -e "${GREEN}== SPEC-044 battle test COMPLETE ==${NC}"
echo "Next: deploy P0 fix, run soak ingest, monitor quarantine logs for 24h."
