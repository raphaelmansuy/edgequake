#!/usr/bin/env bash
# Verify edgequake-postgres image ships all required extensions at expected versions.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=extension-pins.sh
source "$SCRIPT_DIR/extension-pins.sh"

IMAGE="${1:-edgequake-postgres:local}"
CONTAINER="edgequake-postgres-verify-$$"
PGPASSWORD="${POSTGRES_PASSWORD:-edgequake_secret}"
MAX_WAIT_SECS="${MAX_WAIT_SECS:-90}"

cleanup() {
  docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
}
trap cleanup EXIT

echo "Verifying image: $IMAGE"

# Remove stale verify containers from interrupted runs.
_stale="$(docker ps -aq --filter "name=edgequake-postgres-verify-" 2>/dev/null || true)"
if [ -n "$_stale" ]; then
  echo "$_stale" | xargs docker rm -f >/dev/null 2>&1 || true
fi
unset _stale

docker run -d --name "$CONTAINER" \
  -e POSTGRES_USER=edgequake \
  -e POSTGRES_PASSWORD="$PGPASSWORD" \
  -e POSTGRES_DB=edgequake \
  "$IMAGE" >/dev/null

wait_for_postgres() {
  local i
  for i in $(seq 1 "$MAX_WAIT_SECS"); do
    if ! docker inspect -f '{{.State.Running}}' "$CONTAINER" 2>/dev/null | grep -q '^true$'; then
      echo "ERROR: verify container exited before Postgres became ready (attempt $i/${MAX_WAIT_SECS})"
      docker logs "$CONTAINER" 2>&1 | tail -80 || true
      return 1
    fi

    if docker exec -e PGPASSWORD="$PGPASSWORD" "$CONTAINER" \
      psql -U edgequake -d edgequake -v ON_ERROR_STOP=1 -c 'SELECT 1' >/dev/null 2>&1; then
      echo "Postgres ready after ${i}s"
      return 0
    fi

    sleep 1
  done

  echo "ERROR: Postgres did not accept connections within ${MAX_WAIT_SECS}s"
  docker logs "$CONTAINER" 2>&1 | tail -80 || true
  return 1
}

wait_for_postgres

# Optional vectorscale (SPEC-070 pg18-vectorscale profile)
VECTORSCALE_SQL=""
if [ -n "${EQ_PGVECTORSCALE_MIN:-}" ]; then
  VECTORSCALE_SQL=$(cat <<'VSQL'
CREATE EXTENSION IF NOT EXISTS vectorscale CASCADE;
VSQL
)
fi

AGE_SQL=""
AGE_ASSERT=""
if [ -n "${EQ_AGE_MIN:-}" ]; then
  AGE_SQL=$(cat <<'ASQL'
DO $$
BEGIN
  CREATE EXTENSION IF NOT EXISTS age;
EXCEPTION WHEN OTHERS THEN
  RAISE EXCEPTION 'Apache AGE failed to load: %', SQLERRM;
END $$;
ASQL
)
  AGE_ASSERT=$(cat <<ASQL
  IF v_age IS NULL OR string_to_array(v_age, '.')::int[] < string_to_array('${EQ_AGE_MIN}', '.')::int[] THEN
    RAISE EXCEPTION 'Apache AGE must be >= ${EQ_AGE_MIN} (got %)', v_age;
  END IF;
ASQL
)
else
  AGE_ASSERT=$(cat <<'ASQL'
  IF EXISTS (SELECT 1 FROM pg_available_extensions WHERE name = 'age') THEN
    RAISE EXCEPTION 'product image must not ship Apache AGE';
  END IF;
ASQL
)
fi

docker exec -e PGPASSWORD="$PGPASSWORD" "$CONTAINER" psql -U edgequake -d edgequake -v ON_ERROR_STOP=1 <<SQL
CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE EXTENSION IF NOT EXISTS btree_gin;
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
${VECTORSCALE_SQL}
${AGE_SQL}

SELECT extname, extversion
FROM pg_extension
WHERE extname IN ('vector', 'age', 'pg_trgm', 'btree_gin', 'uuid-ossp', 'vectorscale')
ORDER BY extname;

DO \$\$
DECLARE
  v_vector text;
  v_age text;
  v_vs text;
BEGIN
  SELECT extversion INTO v_vector FROM pg_extension WHERE extname = 'vector';
  SELECT extversion INTO v_age FROM pg_extension WHERE extname = 'age';

  IF v_vector IS NULL OR string_to_array(v_vector, '.')::int[] < string_to_array('${EQ_PGVECTOR_MIN}', '.')::int[] THEN
    RAISE EXCEPTION 'pgvector must be >= ${EQ_PGVECTOR_MIN} (got %)', v_vector;
  END IF;
  ${AGE_ASSERT}
  IF '${EQ_PGVECTORSCALE_MIN}' <> '' THEN
    SELECT extversion INTO v_vs FROM pg_extension WHERE extname = 'vectorscale';
    IF v_vs IS NULL OR string_to_array(v_vs, '.')::int[] < string_to_array('${EQ_PGVECTORSCALE_MIN}', '.')::int[] THEN
      RAISE EXCEPTION 'vectorscale must be >= ${EQ_PGVECTORSCALE_MIN} (got %)', v_vs;
    END IF;
  END IF;
END \$\$;
SQL

echo "✓ All EdgeQuake PostgreSQL extensions verified on $IMAGE"
