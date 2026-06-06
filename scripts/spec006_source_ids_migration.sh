#!/usr/bin/env bash
# SPEC-006 P3/P5: verify migration 038 package (marker + size-aware apply SSOT).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MIG_DIR="$ROOT/edgequake/migrations"
SUPPORT="$MIG_DIR/support/038"
APPLY_SCRIPT="$ROOT/edgequake/scripts/migrations/apply_038.sh"

REQUIRED=(
  "$MIG_DIR/038_add_source_ids_gin_indexes.sql"
  "$SUPPORT/apply.sql"
  "$SUPPORT/preflight.sql"
  "$SUPPORT/concurrent.sql"
  "$SUPPORT/rollback.sql"
  "$SUPPORT/verify.sql"
)

for f in "${REQUIRED[@]}"; do
  if [[ ! -f "$f" ]]; then
    echo "Missing migration file: $f"
    exit 1
  fi
done

rg -q 'migration_bootstrap|apply_038' "$MIG_DIR/038_add_source_ids_gin_indexes.sql" || {
  echo "Migration 038 must be a marker deferring DDL to bootstrap/ops"
  exit 1
}

rg -q 'source_ids_gin|vertex_source_id' "$SUPPORT/apply.sql" || {
  echo "support/038/apply.sql missing source_ids index definitions"
  exit 1
}

rg -q 'migration_large_graph_threshold' "$SUPPORT/apply.sql" || {
  echo "support/038/apply.sql missing size-aware threshold gate"
  exit 1
}

rg -q 'CREATE INDEX CONCURRENTLY' "$SUPPORT/concurrent.sql" || {
  echo "Concurrent migration missing CONCURRENTLY indexes"
  exit 1
}

rg -q 'DROP INDEX IF EXISTS' "$SUPPORT/rollback.sql" || {
  echo "Rollback migration missing DROP INDEX"
  exit 1
}

rg -q 'verification failed' "$SUPPORT/verify.sql" || {
  echo "Verify script missing failure gate"
  exit 1
}

rg -q 'support/038/apply.sql' "$ROOT/edgequake/crates/edgequake-api/src/state/migration_bootstrap.rs" || {
  echo "migration_bootstrap.rs must include_str support/038/apply.sql"
  exit 1
}

if [[ ! -f "$APPLY_SCRIPT" ]]; then
  echo "Missing apply script: $APPLY_SCRIPT"
  exit 1
fi

chmod +x "$APPLY_SCRIPT" "$ROOT/scripts/spec006_apply_migration_038.sh" 2>/dev/null || true

echo "✓ SPEC-006: migration 038 package (marker + size-aware apply SSOT)"
