#!/usr/bin/env bash
# SPEC-042 — Version feature battle test (official-docs grounded).
#
# Validates pgvector / AGE / PostgreSQL features per tier against 013-version-feature-matrix.
#
# Usage:
#   ./specs/042-update-age-pgvector/e2e/run_version_feature_battle_test.sh [pg16|pg17|pg18|all]
#
# Requires: docker, bash. Images must exist (make postgres-image-build*).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
DOCKER_DIR="$ROOT/edgequake/docker"
PINS="$DOCKER_DIR/extension-pins.sh"
CHECK_PINS="$ROOT/scripts/check_extension_pins.sh"
SUPPORT_042="$ROOT/edgequake/migrations/support/042/apply.sql"
SUPPORT_043="$ROOT/edgequake/migrations/support/043/apply.sql"

PROFILE="${1:-all}"
PGPASSWORD="${POSTGRES_PASSWORD:-edgequake_secret}"

run_profile() {
  local profile="$1"
  local container="edgequake-bt-${profile}-$$"

  # shellcheck source=/dev/null
  EQ_POSTGRES_PROFILE="$profile" source "$PINS"

  local image="$EQ_POSTGRES_IMAGE_TAG"
  echo ""
  echo "========== BATTLE TEST: $profile ($image) =========="

  if ! docker image inspect "$image" >/dev/null 2>&1; then
    case "$profile" in
      pg16) echo "Image $image not found — run: make postgres-image-build" ;;
      pg17) echo "Image $image not found — run: make postgres-image-build-pg17" ;;
      pg18) echo "Image $image not found — run: make postgres-image-build-pg18" ;;
    esac
    return 1
  fi

  cleanup() { docker rm -f "$container" >/dev/null 2>&1 || true; }
  trap cleanup RETURN

  docker run -d --name "$container" \
    -e POSTGRES_USER=edgequake \
    -e POSTGRES_PASSWORD="$PGPASSWORD" \
    -e POSTGRES_DB=edgequake \
    "$image" >/dev/null

  for i in $(seq 1 90); do
    if docker exec -e PGPASSWORD="$PGPASSWORD" "$container" \
      psql -U edgequake -d edgequake -c 'SELECT 1' >/dev/null 2>&1; then
      break
    fi
    if [ "$i" -eq 90 ]; then
      echo "Postgres did not become ready"
      docker logs "$container" 2>&1 | tail -40
      return 1
    fi
    sleep 1
  done

  docker exec -i -e PGPASSWORD="$PGPASSWORD" "$container" \
    psql -U edgequake -d edgequake -v ON_ERROR_STOP=1 <<'SQL'
CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE EXTENSION IF NOT EXISTS btree_gin;
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS age;
LOAD 'age';
ALTER EXTENSION vector UPDATE;
ALTER EXTENSION age UPDATE;
SELECT extname, extversion FROM pg_extension WHERE extname IN ('vector','age') ORDER BY extname;
SQL

  echo "  BT-PV-01: iterative scan GUCs..."
  docker exec -e PGPASSWORD="$PGPASSWORD" "$container" psql -U edgequake -d edgequake -v ON_ERROR_STOP=1 -c \
    "BEGIN; SET LOCAL hnsw.iterative_scan = strict_order; SET LOCAL hnsw.max_scan_tuples = 20000; SET LOCAL ivfflat.iterative_scan = relaxed_order; COMMIT;"

  echo "  BT-PV-03: halfvec type catalog..."
  docker exec -e PGPASSWORD="$PGPASSWORD" "$container" psql -U edgequake -d edgequake -tAc \
    "SELECT 1 FROM pg_type WHERE typname = 'halfvec';" | grep -q 1

  echo "  BT-PV-04: halfvec HNSW insert + filtered ANN..."
  docker exec -e PGPASSWORD="$PGPASSWORD" "$container" psql -U edgequake -d edgequake -v ON_ERROR_STOP=1 -c \
    "CREATE TEMP TABLE bt_halfvec (id int, embedding halfvec(3), tenant text);
     INSERT INTO bt_halfvec VALUES (1,'[1,0,0]','a'),(2,'[0.9,0.1,0]','a'),(3,'[0,1,0]','b');
     CREATE INDEX ON bt_halfvec USING hnsw (embedding halfvec_cosine_ops);
     BEGIN;
     SET LOCAL hnsw.iterative_scan = strict_order;
     SET LOCAL hnsw.ef_search = 40;
     SELECT count(*) FROM (SELECT 1 FROM bt_halfvec WHERE tenant = 'a' ORDER BY embedding <=> '[1,0,0]'::halfvec LIMIT 5) s;
     COMMIT;"

  echo "  BT-PV-02: filtered HNSW ANN micro-benchmark..."
  docker exec -e PGPASSWORD="$PGPASSWORD" "$container" psql -U edgequake -d edgequake -v ON_ERROR_STOP=1 -c \
    "CREATE TEMP TABLE bt_vectors (id int, embedding vector(3), tenant text);
     INSERT INTO bt_vectors VALUES (1,'[1,0,0]','a'),(2,'[0.9,0.1,0]','a'),(3,'[0,1,0]','b');
     CREATE INDEX ON bt_vectors USING hnsw (embedding vector_cosine_ops);
     BEGIN;
     SET LOCAL hnsw.iterative_scan = strict_order;
     SET LOCAL hnsw.ef_search = 40;
     SELECT count(*) FROM (SELECT 1 FROM bt_vectors WHERE tenant = 'a' ORDER BY embedding <=> '[1,0,0]' LIMIT 5) s;
     COMMIT;"

  echo "  BT-AGE-01: Cypher MERGE/MATCH..."
  docker exec -e PGPASSWORD="$PGPASSWORD" "$container" psql -U edgequake -d edgequake -v ON_ERROR_STOP=1 -c \
    "SET search_path = ag_catalog, public;
     SELECT create_graph('bt_spec042');
     SELECT * FROM cypher('bt_spec042', \$\$ MERGE (n:BTProbe {id: 'spec042'}) RETURN n.id \$\$) AS (id agtype);
     SELECT drop_graph('bt_spec042', true);"

  echo "  BT-AGE-02 + tier extversion gates..."
  docker exec -i -e PGPASSWORD="$PGPASSWORD" "$container" \
    psql -U edgequake -d edgequake -v ON_ERROR_STOP=1 <<SQL
DO \$\$
DECLARE
  v_vector text;
  v_age text;
  v_major int;
BEGIN
  SELECT extversion INTO v_vector FROM pg_extension WHERE extname = 'vector';
  SELECT extversion INTO v_age FROM pg_extension WHERE extname = 'age';
  SELECT current_setting('server_version_num')::int / 10000 INTO v_major;
  IF v_vector IS NULL OR string_to_array(v_vector, '.')::int[] < string_to_array('${EQ_PGVECTOR_MIN}', '.')::int[] THEN
    RAISE EXCEPTION 'BT-AGE-02 FAIL pgvector >= ${EQ_PGVECTOR_MIN} (got %)', v_vector;
  END IF;
  IF v_age IS NULL OR string_to_array(v_age, '.')::int[] < string_to_array('${EQ_AGE_MIN}', '.')::int[] THEN
    RAISE EXCEPTION 'BT-AGE-02 FAIL age >= ${EQ_AGE_MIN} (got %)', v_age;
  END IF;
  IF v_major <> ${EQ_POSTGRES_MAJOR} THEN
    RAISE EXCEPTION 'BT-PG FAIL server major % <> expected ${EQ_POSTGRES_MAJOR}', v_major;
  END IF;
END \$\$;
SQL

  case "$profile" in
    pg17)
      echo "  BT-PG-17: confirmed major 17"
      ;;
    pg18)
      echo "  BT-PG-18-01: uuidv7()..."
      docker exec -e PGPASSWORD="$PGPASSWORD" "$container" psql -U edgequake -d edgequake -tAc \
        "SELECT uuidv7() IS NOT NULL;"
      echo "  BT-PG-18-02: confirmed major 18"
      ;;
    pg16)
      echo "  BT-PG-16: uuidv7 absent (expected)..."
      if docker exec -e PGPASSWORD="$PGPASSWORD" "$container" psql -U edgequake -d edgequake -tAc \
        "SELECT uuidv7();" 2>/dev/null; then
        echo "ERROR: uuidv7 should not exist on PG16"
        return 1
      fi
      ;;
  esac

  echo "  BT-M042/M043: bootstrap apply SQL..."
  docker cp "$SUPPORT_042" "${container}:/tmp/042_apply.sql"
  docker cp "$SUPPORT_043" "${container}:/tmp/043_apply.sql"
  docker exec -e PGPASSWORD="$PGPASSWORD" "$container" psql -U edgequake -d edgequake -v ON_ERROR_STOP=1 \
    -c "SELECT extname, extversion FROM pg_extension WHERE extname IN ('vector','age') ORDER BY extname;" \
    | grep -q vector \
    || { echo "ERROR: vector extension missing before M042"; return 1; }
  docker exec -e PGPASSWORD="$PGPASSWORD" "$container" psql -U edgequake -d edgequake -v ON_ERROR_STOP=1 \
    -f /tmp/042_apply.sql
  docker exec -e PGPASSWORD="$PGPASSWORD" "$container" psql -U edgequake -d edgequake -v ON_ERROR_STOP=1 \
    -f /tmp/043_apply.sql

  echo "✓ BATTLE TEST PASSED: $profile (vector>=$EQ_PGVECTOR_MIN, age>=$EQ_AGE_MIN, pg$EQ_POSTGRES_MAJOR)"
}

echo "== BT-PIN: extension pin SSOT =="
chmod +x "$CHECK_PINS"
"$CHECK_PINS" "${PROFILE/all/all}"

# Clean stale battle-test containers from interrupted runs
_stale="$(docker ps -aq --filter "name=edgequake-bt-" 2>/dev/null || true)"
if [ -n "$_stale" ]; then
  echo "$_stale" | xargs docker rm -f >/dev/null 2>&1 || true
fi
unset _stale

case "$PROFILE" in
  all)
    failed=0
    for p in pg16 pg17 pg18; do
      run_profile "$p" || failed=1
    done
    if [ "$failed" -ne 0 ]; then
      echo "One or more profiles failed"
      exit 1
    fi
    echo ""
    echo "✓ ALL PROFILES PASSED — SPEC-042 battle test complete"
    ;;
  pg16|pg17|pg18)
    run_profile "$PROFILE"
    ;;
  *)
    echo "Usage: $0 [pg16|pg17|pg18|all]"
    exit 1
    ;;
esac
