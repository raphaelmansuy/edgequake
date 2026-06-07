#!/usr/bin/env bash
# EdgeQuake Migration 038 — source_ids / source_id index apply wrapper
#
# Canonical ops script for production-safe index migration (SPEC-006).
# sqlx auto-applies 038_add_source_ids_gin_indexes.sql on server start;
# use this script for preflight, CONCURRENTLY rebuilds, verify, and rollback.
#
# Usage:
#   export DATABASE_URL="postgres://..."
#   ./edgequake/scripts/migrations/apply_038.sh --help
#   ./edgequake/scripts/migrations/apply_038.sh --dry-run
#   ./edgequake/scripts/migrations/apply_038.sh --apply
#   ./edgequake/scripts/migrations/apply_038.sh --apply --concurrent
#   ./edgequake/scripts/migrations/apply_038.sh --verify
#   ./edgequake/scripts/migrations/apply_038.sh --rollback [--yes]
#
# Exit codes: 0 success | 1 usage/validation | 2 psql/SQL failure
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
EDGEQUAKE_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
SUPPORT_DIR="$EDGEQUAKE_ROOT/migrations/support/038"
MAIN_MIGRATION="$EDGEQUAKE_ROOT/migrations/038_add_source_ids_gin_indexes.sql"

DRY_RUN=false
APPLY=false
CONCURRENT=false
ROLLBACK=false
VERIFY=false
ASSUME_YES=false

usage() {
  cat <<'EOF'
EdgeQuake Migration 038 — source_ids index package

Modes (pick one):
  --dry-run     Pre-flight checks only (read-only, safe on production)
  --apply       Create indexes (standard transactional DO block)
  --verify      Confirm indexes exist after apply
  --rollback    Drop indexes only (no data loss)

Options:
  --concurrent  With --apply: use CREATE INDEX CONCURRENTLY (large graphs)
  --yes         Skip confirmation prompt for --apply / --rollback

Environment:
  DATABASE_URL  Required PostgreSQL connection string

Docs:
  edgequake/docs/migrations/038-source-ids-indexes.md
EOF
}

log() { printf '[%s] %s\n' "$(date -u +%H:%M:%S)" "$*"; }

die() { log "ERROR: $*"; exit 1; }

for arg in "$@"; do
  case "$arg" in
    --dry-run) DRY_RUN=true ;;
    --apply) APPLY=true ;;
    --concurrent) CONCURRENT=true ;;
    --rollback) ROLLBACK=true ;;
    --verify) VERIFY=true ;;
    --yes) ASSUME_YES=true ;;
    --help|-h) usage; exit 0 ;;
    *) die "Unknown argument: $arg (try --help)" ;;
  esac
done

mode_count=0
$DRY_RUN && mode_count=$((mode_count + 1))
$APPLY && mode_count=$((mode_count + 1))
$ROLLBACK && mode_count=$((mode_count + 1))
$VERIFY && mode_count=$((mode_count + 1))

if [[ $mode_count -ne 1 ]]; then
  usage
  die "Specify exactly one mode: --dry-run | --apply | --verify | --rollback"
fi

[[ -n "${DATABASE_URL:-}" ]] || die "DATABASE_URL is not set"
command -v psql >/dev/null 2>&1 || die "psql not found on PATH"

APPLY_SQL="$SUPPORT_DIR/apply.sql"

for f in \
  "$SUPPORT_DIR/preflight.sql" \
  "$SUPPORT_DIR/apply.sql" \
  "$SUPPORT_DIR/concurrent.sql" \
  "$SUPPORT_DIR/rollback.sql" \
  "$SUPPORT_DIR/verify.sql" \
  "$MAIN_MIGRATION"
do
  [[ -f "$f" ]] || die "Missing migration file: $f"
done

run_sql() {
  local file="$1"
  log "psql -f ${file#$EDGEQUAKE_ROOT/}"
  if ! psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f "$file"; then
    die "SQL failed: $(basename "$file")"
  fi
}

confirm() {
  local prompt="$1"
  if $ASSUME_YES; then
    return 0
  fi
  read -r -p "$prompt [y/N] " reply
  [[ "$reply" =~ ^[Yy]$ ]]
}

# Mask credentials in logs
safe_url="${DATABASE_URL%%@*}@***"
log "Migration 038 — target $safe_url"

log "Testing database connection..."
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -c "SELECT 1 AS connection_ok;" >/dev/null \
  || die "Cannot connect to database"

if $ROLLBACK; then
  confirm "Rollback will DROP migration 038 indexes (data unchanged). Continue?" || exit 0
  run_sql "$SUPPORT_DIR/rollback.sql"
  log "Rollback complete"
  exit 0
fi

run_sql "$SUPPORT_DIR/preflight.sql"

if $DRY_RUN; then
  log "Dry-run complete — no schema changes"
  exit 0
fi

if $VERIFY; then
  run_sql "$SUPPORT_DIR/verify.sql"
  log "Verification passed"
  exit 0
fi

if $APPLY; then
  if $CONCURRENT; then
    confirm "Apply CONCURRENTLY (non-transactional, safe for large graphs)?" || exit 0
    run_sql "$SUPPORT_DIR/concurrent.sql"
  else
    confirm "Apply migration 038 indexes (size-aware, idempotent)?" || exit 0
    log "Setting large-graph threshold from EDGEQUAKE_MIGRATION_LARGE_GRAPH_THRESHOLD"
    psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -c \
      "SELECT set_config('edgequake.migration_large_graph_threshold', '${EDGEQUAKE_MIGRATION_LARGE_GRAPH_THRESHOLD:-500000}', false);"
    run_sql "$APPLY_SQL"
  fi
  run_sql "$SUPPORT_DIR/verify.sql"
  log "Apply + verify complete"
  exit 0
fi
