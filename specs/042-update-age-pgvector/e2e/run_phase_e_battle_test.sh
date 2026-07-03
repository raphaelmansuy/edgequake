#!/usr/bin/env bash
# SPEC-042-E — Phase E feature battle test (E-01…E-04 acceptance probes).
#
# Probes:
#   BT-PV-04  — halfvec HNSW insert + filtered ANN (E-01.7)
#   E-02.7    — AGE RLS cross-tenant isolation (PG17+)
#   E-03.5    — uuidv7() timestamp-ordered IDs (PG18)
#   E-04.6    — AGE load_labels_from_file COPY loader (PG17+)
#
# Usage:
#   ./specs/042-update-age-pgvector/e2e/run_phase_e_battle_test.sh [pg17|pg18|all]
#
# Requires: docker, bash. Images from make postgres-image-build*.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
DOCKER_DIR="$ROOT/edgequake/docker"
PINS="$DOCKER_DIR/extension-pins.sh"
SUPPORT_081="$ROOT/edgequake/migrations/support/081/apply.sql"

PROFILE="${1:-all}"
PGPASSWORD="${POSTGRES_PASSWORD:-edgequake_secret}"

psql_i() {
  local container="$1"
  shift
  docker exec -i -e PGPASSWORD="$PGPASSWORD" "$container" \
    psql -U edgequake -d edgequake -v ON_ERROR_STOP=1 "$@"
}

psql_scalar() {
  local container="$1"
  local sql="$2"
  docker exec -i -e PGPASSWORD="$PGPASSWORD" "$container" \
    psql -U edgequake -d edgequake -v ON_ERROR_STOP=1 -tAc "$sql" | tail -1 | tr -d '[:space:]'
}

run_pg17_plus_probes() {
  local container="$1"

  echo "  E-02.7: AGE RLS cross-tenant isolation..."
  psql_i "$container" <<'SQL'
LOAD 'age';
SET search_path = ag_catalog, public;
SELECT create_graph('phase_e_rls');
SELECT * FROM cypher('phase_e_rls', $$ CREATE (a:RLSProbe {id: 'a1', tenant_id: 'tenant_a'}) $$) AS (v agtype);
SELECT * FROM cypher('phase_e_rls', $$ CREATE (b:RLSProbe {id: 'b1', tenant_id: 'tenant_b'}) $$) AS (v agtype);
SQL

  docker cp "$SUPPORT_081" "${container}:/tmp/081_apply.sql"
  psql_i "$container" -f /tmp/081_apply.sql

  psql_i "$container" <<'SQL'
DO $$
BEGIN
  IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'eq_rls_tester') THEN
    CREATE ROLE eq_rls_tester LOGIN PASSWORD 'test' NOBYPASSRLS;
  END IF;
END$$;
GRANT CONNECT ON DATABASE edgequake TO eq_rls_tester;
GRANT USAGE ON SCHEMA phase_e_rls TO eq_rls_tester;
GRANT SELECT ON phase_e_rls._ag_label_vertex TO eq_rls_tester;
SQL

  local count_a count_b
  count_a="$(docker exec -i -e PGPASSWORD=test "$container" \
    psql -U eq_rls_tester -d edgequake -v ON_ERROR_STOP=1 -tAc \
    "SELECT set_config('edgequake.tenant_id', 'tenant_a', false); SELECT count(*)::int FROM phase_e_rls._ag_label_vertex;" | tail -1 | tr -d '[:space:]')"
  count_b="$(docker exec -i -e PGPASSWORD=test "$container" \
    psql -U eq_rls_tester -d edgequake -v ON_ERROR_STOP=1 -tAc \
    "SELECT set_config('edgequake.tenant_id', 'tenant_b', false); SELECT count(*)::int FROM phase_e_rls._ag_label_vertex;" | tail -1 | tr -d '[:space:]')"

  if [ "$count_a" != "1" ] || [ "$count_b" != "1" ]; then
    echo "ERROR: E-02.7 FAIL — tenant_a=$count_a tenant_b=$count_b (expected 1 each with RLS)"
    return 1
  fi
  echo "    RLS isolation OK (eq_rls_tester sees a=$count_a b=$count_b)"

  echo "  E-04.6: AGE COPY loader (load_labels_from_file)..."
  docker exec -e PGPASSWORD="$PGPASSWORD" "$container" bash -c 'mkdir -p /tmp/age && cat > /tmp/age/phase_e_vertices.csv <<CSV
name,tenant_id
Alpha,tenant_a
Beta,tenant_a
Gamma,tenant_b
CSV'
  psql_i "$container" <<'SQL'
LOAD 'age';
SET search_path = ag_catalog, public;
SELECT load_labels_from_file('phase_e_copy', 'BulkNode', 'phase_e_vertices.csv', false);
SQL
  local copy_count
  copy_count="$(psql_scalar "$container" "LOAD 'age'; SET search_path = ag_catalog, public; SELECT count(*)::int FROM cypher('phase_e_copy', \$\$ MATCH (n:BulkNode) RETURN n \$\$) AS (n agtype);")"
  if [ "$copy_count" != "3" ]; then
    echo "ERROR: E-04.6 FAIL — expected 3 vertices, got $copy_count"
    return 1
  fi
  echo "    COPY loader OK ($copy_count vertices)"
}

run_profile() {
  local profile="$1"
  local container="edgequake-phasee-${profile}-$$"

  # shellcheck source=/dev/null
  EQ_POSTGRES_PROFILE="$profile" source "$PINS"

  local image="$EQ_POSTGRES_IMAGE_TAG"
  echo ""
  echo "========== PHASE E BATTLE TEST: $profile ($image) =========="

  if ! docker image inspect "$image" >/dev/null 2>&1; then
    echo "Image $image not found — build with make postgres-image-build*"
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

  psql_i "$container" <<'SQL'
CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS age;
LOAD 'age';
ALTER EXTENSION vector UPDATE;
ALTER EXTENSION age UPDATE;
SQL

  echo "  BT-PV-04: halfvec HNSW insert + filtered ANN..."
  psql_i "$container" <<'SQL'
CREATE TEMP TABLE bt_halfvec (id int, embedding halfvec(3), tenant text);
INSERT INTO bt_halfvec VALUES (1,'[1,0,0]','a'),(2,'[0.9,0.1,0]','a'),(3,'[0,1,0]','b');
CREATE INDEX ON bt_halfvec USING hnsw (embedding halfvec_cosine_ops);
BEGIN;
SET LOCAL hnsw.iterative_scan = strict_order;
SET LOCAL hnsw.ef_search = 40;
SELECT count(*) FROM (SELECT 1 FROM bt_halfvec WHERE tenant = 'a' ORDER BY embedding <=> '[1,0,0]'::halfvec LIMIT 5) s;
COMMIT;
SQL

  case "$profile" in
    pg18)
      echo "  E-03.5: uuidv7() version nibble..."
      local uuid_sample version_nibble
      uuid_sample="$(psql_scalar "$container" "SELECT uuidv7()::text;")"
      version_nibble="${uuid_sample:14:1}"
      if [ "$version_nibble" != "7" ]; then
        echo "ERROR: E-03.5 FAIL — expected uuidv7 nibble 7 at pos 14, got '$version_nibble' in $uuid_sample"
        return 1
      fi
      echo "    uuidv7 sample: $uuid_sample (version nibble=$version_nibble)"
      run_pg17_plus_probes "$container" || return 1
      ;;
    pg17)
      run_pg17_plus_probes "$container" || return 1
      ;;
    *)
      echo "  SKIP E-02/E-03/E-04 tier probes on $profile (PG16 — app-level isolation only)"
      ;;
  esac

  echo "✓ PHASE E BATTLE TEST PASSED: $profile"
}

_stale="$(docker ps -aq --filter "name=edgequake-phasee-" 2>/dev/null || true)"
if [ -n "$_stale" ]; then
  echo "$_stale" | xargs docker rm -f >/dev/null 2>&1 || true
fi
unset _stale

case "$PROFILE" in
  all)
    failed=0
    for p in pg17 pg18; do
      run_profile "$p" || failed=1
    done
    if [ "$failed" -ne 0 ]; then
      echo "PHASE E BATTLE TEST FAILED"
      exit 1
    fi
    ;;
  pg17|pg18)
    run_profile "$PROFILE"
    ;;
  *)
    echo "Usage: $0 [pg17|pg18|all]"
    exit 1
    ;;
esac

echo ""
echo "== All Phase E battle tests passed =="
