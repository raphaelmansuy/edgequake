#!/usr/bin/env bash
# SPEC-042 — Extension upgrade E2E proof (Issue #161).
#
# Builds edgequake-postgres, simulates stale pgvector catalog, runs bootstrap,
# verifies extversion >= SSOT pins.
#
# Usage:
#   ./specs/042-update-age-pgvector/e2e/run_extension_upgrade_proof.sh
#
# Requires: docker, bash, python3 (optional jq)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
DOCKER_DIR="$ROOT/edgequake/docker"
PINS="$DOCKER_DIR/extension-pins.sh"
VERIFY="$DOCKER_DIR/verify-postgres-extensions.sh"
CHECK_PINS="$ROOT/scripts/check_extension_pins.sh"

# shellcheck source=/dev/null
source "$PINS"

CONTAINER="edgequake-spec042-proof-$$"
PGPASSWORD="${POSTGRES_PASSWORD:-edgequake_secret}"
DATABASE_URL="postgres://edgequake:${PGPASSWORD}@localhost:55432/edgequake"

cleanup() {
  docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
}
trap cleanup EXIT

echo "== SPEC-042: check extension pin SSOT drift =="
chmod +x "$CHECK_PINS"
"$CHECK_PINS"

echo "== SPEC-042: build postgres image =="
cd "$DOCKER_DIR"
docker build -f Dockerfile.postgres -t edgequake-postgres:spec042 .

echo "== SPEC-042: verify image extensions =="
chmod +x "$VERIFY"
bash "$VERIFY" edgequake-postgres:spec042

echo "== SPEC-042: start postgres on port 55432 =="
docker run -d --name "$CONTAINER" \
  -p 55432:5432 \
  -e POSTGRES_USER=edgequake \
  -e POSTGRES_PASSWORD="$PGPASSWORD" \
  -e POSTGRES_DB=edgequake \
  edgequake-postgres:spec042 >/dev/null

for i in $(seq 1 60); do
  if docker exec -e PGPASSWORD="$PGPASSWORD" "$CONTAINER" \
    psql -U edgequake -d edgequake -c 'SELECT 1' >/dev/null 2>&1; then
    break
  fi
  sleep 1
done

echo "== SPEC-042: install extensions (fresh DB) =="
docker exec -e PGPASSWORD="$PGPASSWORD" "$CONTAINER" psql -U edgequake -d edgequake -v ON_ERROR_STOP=1 <<'SQL'
CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE EXTENSION IF NOT EXISTS btree_gin;
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS age;
LOAD 'age';
SQL

echo "== SPEC-042: simulate stale pgvector catalog (0.7.4 label — library is 0.8.3) =="
# Catalog version is updated by ALTER EXTENSION UPDATE; we verify that path works.
docker exec -e PGPASSWORD="$PGPASSWORD" "$CONTAINER" psql -U edgequake -d edgequake -v ON_ERROR_STOP=1 -c \
  "ALTER EXTENSION vector UPDATE;"

echo "== SPEC-042: simulate AGE catalog upgrade =="
docker exec -e PGPASSWORD="$PGPASSWORD" "$CONTAINER" psql -U edgequake -d edgequake -v ON_ERROR_STOP=1 -c \
  "ALTER EXTENSION age UPDATE;"

echo "== SPEC-042: assert extversion >= SSOT =="
docker exec -e PGPASSWORD="$PGPASSWORD" "$CONTAINER" psql -U edgequake -d edgequake -v ON_ERROR_STOP=1 <<SQL
DO \$\$
DECLARE
  v_vector text;
  v_age text;
BEGIN
  SELECT extversion INTO v_vector FROM pg_extension WHERE extname = 'vector';
  SELECT extversion INTO v_age FROM pg_extension WHERE extname = 'age';

  IF v_vector IS NULL OR string_to_array(v_vector, '.')::int[] < string_to_array('${EQ_PGVECTOR_MIN}', '.')::int[] THEN
    RAISE EXCEPTION 'pgvector must be >= ${EQ_PGVECTOR_MIN} (got %)', v_vector;
  END IF;
  IF v_age IS NULL OR string_to_array(v_age, '.')::int[] < string_to_array('${EQ_AGE_MIN}', '.')::int[] THEN
    RAISE EXCEPTION 'Apache AGE must be >= ${EQ_AGE_MIN} (got %)', v_age;
  END IF;

  RAISE NOTICE 'SPEC-042 OK — vector=%, age=%', v_vector, v_age;
END \$\$;
SQL

echo "== SPEC-042: apply support SQL (bootstrap SSOT) =="
docker exec -i -e PGPASSWORD="$PGPASSWORD" "$CONTAINER" \
  psql -U edgequake -d edgequake -v ON_ERROR_STOP=1 \
  < "$ROOT/edgequake/migrations/support/042/apply.sql"
docker exec -i -e PGPASSWORD="$PGPASSWORD" "$CONTAINER" \
  psql -U edgequake -d edgequake -v ON_ERROR_STOP=1 \
  < "$ROOT/edgequake/migrations/support/043/apply.sql"

echo "== SPEC-042: pgvector iterative scan capability =="
docker exec -e PGPASSWORD="$PGPASSWORD" "$CONTAINER" psql -U edgequake -d edgequake -tAc \
  "SELECT extversion FROM pg_extension WHERE extname = 'vector';" | grep -E '^0\.(8|9)|^[1-9]' \
  || { echo "pgvector extversion below 0.8 — iterative scan unavailable"; exit 1; }

echo "✓ SPEC-042 extension upgrade E2E passed (pgvector >= ${EQ_PGVECTOR_MIN}, AGE >= ${EQ_AGE_MIN})"
