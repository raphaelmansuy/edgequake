#!/usr/bin/env bash
# SPEC-042-B — E2E proof: PG16 sample data → PG18 via migrate_postgres_major.sh
#
# Spins up PG16 + PG18 containers, seeds minimal schema, runs migration script.
#
# Usage: ./specs/042-update-age-pgvector/e2e/run_pg18_migration_procedure.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
DOCKER_DIR="$ROOT/edgequake/docker"
PGPASSWORD="${POSTGRES_PASSWORD:-edgequake_secret}"
PG16_PORT=55432
PG18_PORT=55433
PG16_URL="postgres://edgequake:${PGPASSWORD}@localhost:${PG16_PORT}/edgequake"
PG18_URL="postgres://edgequake:${PGPASSWORD}@localhost:${PG18_PORT}/edgequake"
DUMP="/tmp/edgequake-spec042b-$(date +%s).dump"

cleanup() {
  docker rm -f edgequake-spec042-pg16 edgequake-spec042-pg18 >/dev/null 2>&1 || true
}
trap cleanup EXIT

echo "== SPEC-042-B: build PG16 + PG18 images =="
docker build -f "$DOCKER_DIR/Dockerfile.postgres" -t edgequake-postgres:spec042-pg16 "$DOCKER_DIR"
docker build -f "$DOCKER_DIR/Dockerfile.postgres.pg18" -t edgequake-postgres:spec042-pg18 "$DOCKER_DIR"

echo "== SPEC-042-B: start PG16 source =="
docker run -d --name edgequake-spec042-pg16 \
  -p "${PG16_PORT}:5432" \
  -e POSTGRES_USER=edgequake \
  -e POSTGRES_PASSWORD="$PGPASSWORD" \
  -e POSTGRES_DB=edgequake \
  edgequake-postgres:spec042-pg16 >/dev/null

echo "== SPEC-042-B: start PG18 target =="
docker run -d --name edgequake-spec042-pg18 \
  -p "${PG18_PORT}:5432" \
  -e POSTGRES_USER=edgequake \
  -e POSTGRES_PASSWORD="$PGPASSWORD" \
  -e POSTGRES_DB=edgequake \
  edgequake-postgres:spec042-pg18 >/dev/null

wait_pg() {
  local c=$1
  for i in $(seq 1 60); do
    if docker exec -e PGPASSWORD="$PGPASSWORD" "$c" \
      psql -U edgequake -d edgequake -c 'SELECT 1' >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done
  echo "Postgres not ready: $c"; exit 1
}
wait_pg edgequake-spec042-pg16
wait_pg edgequake-spec042-pg18

echo "== SPEC-042-B: seed PG16 (extensions + marker table) =="
docker exec -e PGPASSWORD="$PGPASSWORD" edgequake-spec042-pg16 \
  psql -U edgequake -d edgequake -v ON_ERROR_STOP=1 -c "CREATE EXTENSION IF NOT EXISTS vector;"
docker exec -e PGPASSWORD="$PGPASSWORD" edgequake-spec042-pg16 \
  psql -U edgequake -d edgequake -v ON_ERROR_STOP=1 -c "CREATE EXTENSION IF NOT EXISTS age;"
docker exec -e PGPASSWORD="$PGPASSWORD" edgequake-spec042-pg16 \
  psql -U edgequake -d edgequake -v ON_ERROR_STOP=1 -c "LOAD 'age';"
docker exec -e PGPASSWORD="$PGPASSWORD" edgequake-spec042-pg16 \
  psql -U edgequake -d edgequake -v ON_ERROR_STOP=1 \
  -c "CREATE TABLE IF NOT EXISTS spec042b_marker (id serial PRIMARY KEY, note text);"
docker exec -e PGPASSWORD="$PGPASSWORD" edgequake-spec042-pg16 \
  psql -U edgequake -d edgequake -v ON_ERROR_STOP=1 \
  -c "INSERT INTO spec042b_marker (note) VALUES ('pg16-seed');"

echo "== SPEC-042-B: run migration procedure =="
chmod +x "$ROOT/scripts/migrate_postgres_major.sh"
"$ROOT/scripts/migrate_postgres_major.sh" \
  --source-url "$PG16_URL" \
  --target-url "$PG18_URL" \
  --dump-file "$DUMP"

echo "== SPEC-042-B: verify seed data on PG18 =="
docker exec -e PGPASSWORD="$PGPASSWORD" edgequake-spec042-pg18 \
  psql -U edgequake -d edgequake -tAc "SELECT count(*) FROM spec042b_marker WHERE note = 'pg16-seed';" \
  | grep -qx '1' || { echo "Seed data missing after restore"; exit 1; }

docker exec -e PGPASSWORD="$PGPASSWORD" edgequake-spec042-pg18 \
  psql -U edgequake -d edgequake -tAc \
  "SELECT extversion FROM pg_extension WHERE extname = 'age';" | grep -E '^1\.(7|[89]|[1-9][0-9])' \
  || { echo "AGE extversion not >= 1.7.0 on PG18"; exit 1; }

echo "✓ SPEC-042-B PG16→PG18 migration procedure E2E passed"
rm -f "$DUMP"
