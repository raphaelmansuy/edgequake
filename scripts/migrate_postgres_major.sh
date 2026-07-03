#!/usr/bin/env bash
# SPEC-042 — PostgreSQL major-version migration procedure (multi-major SSOT).
#
# Logical dump from source, restore to target (PG16/17/18), extension alignment, postflight.
#
# Usage:
#   ./scripts/migrate_postgres_major.sh \
#     --source-url "postgres://edgequake:secret@localhost:5432/edgequake" \
#     --target-url "postgres://edgequake:secret@localhost:5433/edgequake" \
#     --dump-file /tmp/edgequake-pg16.dump
#
#   ./scripts/migrate_postgres_major.sh --dry-run --source-url ... --target-url ...
#
# Requires: pg_dump, pg_restore, psql, curl (optional postflight)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PINS="$ROOT/edgequake/docker/extension-pins.sh"
DRY_RUN=0
SOURCE_URL=""
TARGET_URL=""
DUMP_FILE=""

usage() {
  cat <<EOF
Usage: $0 --source-url URL --target-url URL [--dump-file PATH] [--dry-run]

  --source-url   Source DATABASE_URL (e.g. PG16)
  --target-url   Target DATABASE_URL (PG16, PG17, or PG18 — pins auto-detected)
  --dump-file    Path for pg_dump output (default: /tmp/edgequake-major-migrate-YYYYMMDD.dump)
  --dry-run      Preflight only — no dump/restore
EOF
  exit "${1:-0}"
}

while [ $# -gt 0 ]; do
  case "$1" in
    --source-url) SOURCE_URL="$2"; shift 2 ;;
    --target-url) TARGET_URL="$2"; shift 2 ;;
    --dump-file) DUMP_FILE="$2"; shift 2 ;;
    --dry-run) DRY_RUN=1; shift ;;
    -h|--help) usage 0 ;;
    *) echo "Unknown option: $1"; usage 1 ;;
  esac
done

[ -n "$SOURCE_URL" ] && [ -n "$TARGET_URL" ] || usage 1
DUMP_FILE="${DUMP_FILE:-/tmp/edgequake-major-migrate-$(date +%Y%m%d).dump}"

# Detect target PG major and load matching extension pins.
_raw_major() {
  psql "$1" -tAc "SELECT (current_setting('server_version_num')::int / 10000)" 2>/dev/null | tr -d '[:space:]'
}
TARGET_MAJOR="$(_raw_major "$TARGET_URL")"
case "$TARGET_MAJOR" in
  16) EQ_POSTGRES_PROFILE=pg16 ;;
  17) EQ_POSTGRES_PROFILE=pg17 ;;
  18) EQ_POSTGRES_PROFILE=pg18 ;;
  *)
    echo "ERROR: unsupported target PostgreSQL major: ${TARGET_MAJOR:-unknown}"
    exit 1
    ;;
esac
# shellcheck source=/dev/null
source "$PINS"

log() { echo "== migrate_postgres_major: $*"; }

# Host pg_dump/psql may be older than server (e.g. Homebrew 14 vs PG16).
# Fall back to matching postgres Docker client with --network host.
pg_server_major() {
  psql "$1" -tAc "SELECT (current_setting('server_version_num')::int / 10000)"
}

client_pg_major() {
  psql --version 2>/dev/null | sed -n 's/.*) \([0-9]*\).*/\1/p' | head -1
}

run_psql() {
  local url=$1
  shift
  local server_major client_major
  server_major=$(pg_server_major "$url")
  client_major=$(client_pg_major)
  if [ -n "$client_major" ] && [ "$client_major" -ge "$server_major" ]; then
    psql "$url" "$@"
  else
    _docker_psql "$url" "$@"
  fi
}

run_psql_file() {
  local url=$1 file=$2
  local server_major client_major
  server_major=$(pg_server_major "$url")
  client_major=$(client_pg_major)
  if [ -n "$client_major" ] && [ "$client_major" -ge "$server_major" ]; then
    psql "$url" -v ON_ERROR_STOP=1 -f "$file"
  else
    _docker_psql "$url" -v ON_ERROR_STOP=1 < "$file"
  fi
}

_docker_psql() {
  local url=$1
  shift
  local server_major pgpass host port user db
  server_major=$(pg_server_major "$url")
  pgpass=$(printf '%s' "$url" | sed -E 's|^[^:]+://[^:]+:([^@]+)@.*|\1|')
  user=$(printf '%s' "$url" | sed -E 's|^[^:]+://([^:]+):.*|\1|')
  host=$(printf '%s' "$url" | sed -E 's|^[^:]+://[^@]+@([^:/]+).*|\1|')
  port=$(printf '%s' "$url" | sed -E 's|^[^:]+://[^@]+@[^:]+:([0-9]+)/.*|\1|')
  db=$(printf '%s' "$url" | sed -E 's|^[^:]+://[^/]+/([^?]*).*|\1|')
  port=${port:-5432}
  docker run --rm --network host -i -e PGPASSWORD="$pgpass" "postgres:${server_major}-bookworm" \
    psql -h "$host" -p "$port" -U "$user" -d "$db" "$@"
}

run_pg_dump() {
  local url=$1 out=$2
  local server_major client_major
  server_major=$(pg_server_major "$url")
  client_major=$(client_pg_major)
  if [ -n "$client_major" ] && [ "$client_major" -ge "$server_major" ]; then
    pg_dump -Fc -v -f "$out" "$url"
  else
    log "host psql $client_major < server $server_major — using docker pg_dump"
    local pgpass host port user db
    pgpass=$(printf '%s' "$url" | sed -E 's|^[^:]+://[^:]+:([^@]+)@.*|\1|')
    user=$(printf '%s' "$url" | sed -E 's|^[^:]+://([^:]+):.*|\1|')
    host=$(printf '%s' "$url" | sed -E 's|^[^:]+://[^@]+@([^:/]+).*|\1|')
    port=$(printf '%s' "$url" | sed -E 's|^[^:]+://[^@]+@[^:]+:([0-9]+)/.*|\1|')
    db=$(printf '%s' "$url" | sed -E 's|^[^:]+://[^/]+/([^?]*).*|\1|')
    port=${port:-5432}
    docker run --rm --network host -e PGPASSWORD="$pgpass" "postgres:${server_major}-bookworm" \
      pg_dump -Fc -v -h "$host" -p "$port" -U "$user" -d "$db" > "$out"
  fi
}

run_pg_restore() {
  local url=$1 dump=$2
  local server_major client_major
  server_major=$(pg_server_major "$url")
  client_major=$(client_pg_major)
  if [ -n "$client_major" ] && [ "$client_major" -ge "$server_major" ]; then
    pg_restore -v --no-owner --role=edgequake -d "$url" "$dump" \
      2>&1 | tee /tmp/edgequake-pg-restore.log || {
        log "pg_restore returned non-zero (often OK — review log for fatal errors)"
      }
  else
    log "host psql $client_major < server $server_major — using docker pg_restore"
    local pgpass host port user db
    pgpass=$(printf '%s' "$url" | sed -E 's|^[^:]+://[^:]+:([^@]+)@.*|\1|')
    user=$(printf '%s' "$url" | sed -E 's|^[^:]+://([^:]+):.*|\1|')
    host=$(printf '%s' "$url" | sed -E 's|^[^:]+://[^@]+@([^:/]+).*|\1|')
    port=$(printf '%s' "$url" | sed -E 's|^[^:]+://[^@]+@[^:]+:([0-9]+)/.*|\1|')
    db=$(printf '%s' "$url" | sed -E 's|^[^:]+://[^/]+/([^?]*).*|\1|')
    port=${port:-5432}
    docker run --rm --network host -i -e PGPASSWORD="$pgpass" "postgres:${server_major}-bookworm" \
      pg_restore -v --no-owner --role="$user" -h "$host" -p "$port" -U "$user" -d "$db" \
      < "$dump" 2>&1 | tee /tmp/edgequake-pg-restore.log || {
        log "pg_restore returned non-zero (often OK — review log for fatal errors)"
      }
  fi
}

preflight() {
  log "preflight source"
  run_psql "$SOURCE_URL" -v ON_ERROR_STOP=1 -c "SELECT version();"
  run_psql "$SOURCE_URL" -v ON_ERROR_STOP=1 -c \
    "SELECT extname, extversion FROM pg_extension WHERE extname IN ('vector','age') ORDER BY 1;"
  run_psql "$SOURCE_URL" -v ON_ERROR_STOP=1 -c \
    "SELECT coalesce(max(version), 0) AS latest_sqlx FROM _sqlx_migrations;" 2>/dev/null || \
    echo "  (no _sqlx_migrations yet)"

  log "preflight target (PG${TARGET_MAJOR} — expected AGE >= ${EQ_AGE_MIN})"
  run_psql "$TARGET_URL" -v ON_ERROR_STOP=1 -c "SELECT version();"
}

prepare_target_extensions() {
  log "prepare target extensions"
  run_psql "$TARGET_URL" -v ON_ERROR_STOP=1 -c "CREATE EXTENSION IF NOT EXISTS vector;"
  run_psql "$TARGET_URL" -v ON_ERROR_STOP=1 -c "CREATE EXTENSION IF NOT EXISTS pg_trgm;"
  run_psql "$TARGET_URL" -v ON_ERROR_STOP=1 -c "CREATE EXTENSION IF NOT EXISTS btree_gin;"
  run_psql "$TARGET_URL" -v ON_ERROR_STOP=1 -c 'CREATE EXTENSION IF NOT EXISTS "uuid-ossp";'
  run_psql "$TARGET_URL" -v ON_ERROR_STOP=1 -c "CREATE EXTENSION IF NOT EXISTS age;"
  run_psql "$TARGET_URL" -v ON_ERROR_STOP=1 -c "LOAD 'age';"
}

dump_source() {
  log "pg_dump → $DUMP_FILE"
  run_pg_dump "$SOURCE_URL" "$DUMP_FILE"
  ls -lh "$DUMP_FILE"
}

restore_target() {
  log "pg_restore from $DUMP_FILE"
  run_pg_restore "$TARGET_URL" "$DUMP_FILE"
}

align_extensions() {
  log "M042/M043 apply (extension catalog + reindex)"
  run_psql_file "$TARGET_URL" "$ROOT/edgequake/migrations/support/042/apply.sql"
  run_psql_file "$TARGET_URL" "$ROOT/edgequake/migrations/support/043/apply.sql"
}

postflight() {
  log "postflight extension versions"
  local tmp
  tmp=$(mktemp)
  cat >"$tmp" <<SQL
SELECT extname, extversion,
       (SELECT default_version FROM pg_available_extensions e2 WHERE e2.name = e.extname) AS shipped
FROM pg_extension e
WHERE extname IN ('vector', 'age')
ORDER BY 1;

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
  RAISE NOTICE 'postflight OK — vector=%, age=%', v_vector, v_age;
END \$\$;
SQL
  run_psql_file "$TARGET_URL" "$tmp"
  rm -f "$tmp"

  run_psql "$TARGET_URL" -v ON_ERROR_STOP=1 -c \
    "SELECT count(*) AS graph_count FROM ag_catalog.ag_graph;" 2>/dev/null || true
  run_psql "$TARGET_URL" -c \
    "SELECT coalesce(max(version), 0) AS latest_sqlx FROM _sqlx_migrations;" 2>/dev/null || \
    echo "  (no _sqlx_migrations on target)"
}

preflight
[ "$DRY_RUN" -eq 1 ] && { log "dry-run complete"; exit 0; }

prepare_target_extensions
dump_source
restore_target
align_extensions
postflight

log "✓ major migration complete"
log "Next: export DATABASE_URL=\"$TARGET_URL\" && make backend-bg"
log "Verify: curl -sf http://localhost:8080/health | jq '.operational.migration'"
