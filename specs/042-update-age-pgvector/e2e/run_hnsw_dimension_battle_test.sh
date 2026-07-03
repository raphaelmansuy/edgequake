#!/usr/bin/env bash
# SPEC-042 / GitHub #275 — HNSW dimension guard battle test.
#
# Probes:
#   BT-275-01 — vector(3072) HNSW with vector_cosine_ops FAILS (pgvector max 2000)
#   BT-275-02 — M071 apply path: promote to halfvec(3072) + halfvec HNSW succeeds
#   BT-275-03 — dim > 4000 skips ANN index (sequential scan fallback)
#
# Usage:
#   ./specs/042-update-age-pgvector/e2e/run_hnsw_dimension_battle_test.sh [pg16|pg17|pg18|all]
#
# Requires: docker, bash, postgres images (make postgres-image-build*).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
DOCKER_DIR="$ROOT/edgequake/docker"
PINS="$DOCKER_DIR/extension-pins.sh"
M071_SQL="$ROOT/edgequake/migrations/071_hnsw_optimize.sql"

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

run_profile() {
  local profile="$1"
  local container="edgequake-bt275-${profile}-$$"

  # shellcheck source=/dev/null
  EQ_POSTGRES_PROFILE="$profile" source "$PINS"
  local image="$EQ_POSTGRES_IMAGE_TAG"

  echo ""
  echo "========== BT-275 HNSW DIMENSION: $profile ($image) =========="

  if ! docker image inspect "$image" >/dev/null 2>&1; then
    echo "Image $image not found — run make postgres-image-build*"
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

  psql_i "$container" -c "CREATE EXTENSION IF NOT EXISTS vector; ALTER EXTENSION vector UPDATE;"

  echo "  BT-275-01: vector(3072) + vector_cosine_ops HNSW must fail..."
  set +e
  psql_i "$container" <<'SQL'
CREATE TABLE public.eq_bt275_fail_vectors (
  id text PRIMARY KEY,
  embedding vector(3072) NOT NULL
);
INSERT INTO eq_bt275_fail_vectors (id, embedding)
VALUES ('v1', (SELECT array_fill(0.1::float4, ARRAY[3072])::vector(3072)));
CREATE INDEX eq_bt275_fail_vectors_embedding_idx
  ON public.eq_bt275_fail_vectors USING hnsw (embedding vector_cosine_ops);
SQL
  local fail_exit=$?
  set -e
  psql_i "$container" -c "DROP TABLE IF EXISTS public.eq_bt275_fail_vectors CASCADE;" >/dev/null 2>&1 || true
  if [ "$fail_exit" -eq 0 ]; then
    echo "ERROR: BT-275-01 FAIL — vector HNSW on 3072-d should have failed"
    return 1
  fi
  echo "    vector HNSW on dim=3072 correctly rejected"

  echo "  BT-275-02: M071 migration promotes vector(3072) → halfvec HNSW..."
  psql_i "$container" -c "DROP TABLE IF EXISTS public.eq_bt275_vectors CASCADE;"
  psql_i "$container" <<'SQL'
CREATE TABLE public.eq_bt275_vectors (
  id text PRIMARY KEY,
  embedding vector(3072) NOT NULL,
  metadata jsonb DEFAULT '{}'::jsonb,
  created_at timestamptz NOT NULL DEFAULT now()
);
INSERT INTO eq_bt275_vectors (id, embedding)
VALUES ('v1', (SELECT array_fill(0.1::float4, ARRAY[3072])::vector(3072)));
SQL

  docker cp "$M071_SQL" "${container}:/tmp/071_hnsw_optimize.sql"
  psql_i "$container" -f /tmp/071_hnsw_optimize.sql

  local udt idx_op
  udt="$(psql_scalar "$container" \
    "SELECT udt_name FROM information_schema.columns WHERE table_name='eq_bt275_vectors' AND column_name='embedding';")"
  idx_op="$(psql_scalar "$container" \
    "SELECT indexdef FROM pg_indexes WHERE tablename='eq_bt275_vectors' AND indexname='eq_bt275_vectors_embedding_idx';")"

  if [ "$udt" != "halfvec" ]; then
    echo "ERROR: BT-275-02 FAIL — expected halfvec column, got udt=$udt"
    return 1
  fi
  if ! echo "$idx_op" | grep -qi 'halfvec_cosine_ops'; then
    echo "ERROR: BT-275-02 FAIL — expected halfvec_cosine_ops index, got: $idx_op"
    return 1
  fi
  if ! echo "$idx_op" | grep -q "ef_construction='32'"; then
    echo "ERROR: BT-275-02 FAIL — expected ef_construction='32' in index, got: $idx_op"
    return 1
  fi

  local ann_count
  ann_count="$(psql_scalar "$container" \
    "SELECT count(*)::int FROM (SELECT 1 FROM eq_bt275_vectors ORDER BY embedding <=> (SELECT embedding FROM eq_bt275_vectors LIMIT 1) LIMIT 3) s;")"
  if [ "$ann_count" != "1" ]; then
    echo "ERROR: BT-275-02 FAIL — ANN query returned $ann_count rows (expected 1)"
    return 1
  fi
  echo "    M071 promoted halfvec(3072) + HNSW OK (ANN rows=$ann_count)"

  echo "  BT-275-03: dim > 4000 skips HNSW (no index)..."
  psql_i "$container" -c "DROP TABLE IF EXISTS public.eq_bt275_huge CASCADE;"
  psql_i "$container" <<'SQL'
CREATE TABLE public.eq_bt275_huge_vectors (
  id text PRIMARY KEY,
  embedding vector(5000) NOT NULL
);
INSERT INTO eq_bt275_huge_vectors (id, embedding)
VALUES ('h1', (SELECT array_fill(0.2::float4, ARRAY[5000])::vector(5000)));
SQL
  docker cp "$M071_SQL" "${container}:/tmp/071_hnsw_optimize.sql"
  psql_i "$container" -f /tmp/071_hnsw_optimize.sql

  local huge_idx_count
  huge_idx_count="$(psql_scalar "$container" \
    "SELECT count(*)::int FROM pg_indexes WHERE tablename='eq_bt275_huge_vectors' AND indexdef ILIKE '%hnsw%';")"
  if [ "$huge_idx_count" != "0" ]; then
    echo "ERROR: BT-275-03 FAIL — expected no HNSW on dim=5000, found $huge_idx_count indexes"
    return 1
  fi
  echo "    dim=5000 correctly has no HNSW index"

  echo "✓ BT-275 PASSED: $profile"
}

_stale="$(docker ps -aq --filter "name=edgequake-bt275-" 2>/dev/null || true)"
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
      echo "BT-275 HNSW dimension battle test FAILED"
      exit 1
    fi
    echo ""
    echo "✓ ALL PROFILES PASSED — BT-275 HNSW dimension guard"
    ;;
  pg16|pg17|pg18)
    run_profile "$PROFILE"
    ;;
  *)
    echo "Usage: $0 [pg16|pg17|pg18|all]"
    exit 1
    ;;
esac
