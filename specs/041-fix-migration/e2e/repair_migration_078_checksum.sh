#!/usr/bin/env bash
# SPEC-041 — Repair _sqlx_migrations checksum for M078 after in-place fix.
# For v0.13.2 installs that applied M078 successfully (no Node table at upgrade).
#
# Usage: ./repair_migration_078_checksum.sh [DATABASE_URL]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
DB_URL="${1:-${DATABASE_URL:-postgresql://edgequake:edgequake_secret@localhost:5432/edgequake}}"

# Canonical checksum from checksums.lock (post SPEC-041 fix)
NEW_CHECKSUM="ae80e5a9566697893557b5dc16c8501a2d7bd43bf443208390ec017fa409eb8c120b0d1cb3b118ae02e806bdf88d1201"

psql_cmd() {
  if docker ps --format '{{.Names}}' 2>/dev/null | grep -qx 'edgequake-postgres'; then
    docker exec edgequake-postgres psql -U edgequake -d edgequake -v ON_ERROR_STOP=1 "$@"
  else
    psql "$DB_URL" -v ON_ERROR_STOP=1 "$@"
  fi
}

echo "== SPEC-041 checksum repair for M078 =="

EXISTS=$(psql_cmd -t -A -c "SELECT COUNT(*) FROM _sqlx_migrations WHERE version = 78 AND success = true;")

if [[ "${EXISTS}" == "0" ]]; then
  echo "INFO: M078 not recorded — no checksum repair needed (blocked install or fresh)"
  exit 0
fi

CURRENT=$(psql_cmd -t -A -c "SELECT encode(checksum, 'hex') FROM _sqlx_migrations WHERE version = 78;")

if [[ "${CURRENT}" == "${NEW_CHECKSUM}" ]]; then
  echo "PASS: M078 checksum already matches SPEC-041 canonical value"
  exit 0
fi

echo "Updating M078 checksum:"
echo "  from: ${CURRENT}"
echo "  to:   ${NEW_CHECKSUM}"

psql_cmd -c "
UPDATE _sqlx_migrations
SET checksum = decode('${NEW_CHECKSUM}', 'hex')
WHERE version = 78 AND success = true;
"

echo "PASS: M078 checksum repaired. Restart backend to apply fixed migration body (idempotent)."
