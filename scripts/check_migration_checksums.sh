#!/usr/bin/env bash
# scripts/check_migration_checksums.sh
#
# PURPOSE: Verify that every migration SQL file matches the canonical SHA-384
# checksum recorded in edgequake/migrations/checksums.lock.
#
# Exits with code 1 (fails CI) if any file has been modified.
# Exits with code 0 if all files match.
#
# Usage:
#   ./scripts/check_migration_checksums.sh                 # from repo root
#   ./scripts/check_migration_checksums.sh --verbose       # extra output
#
# Why this matters:
#   SQLx stores SHA-384(file_bytes) in _sqlx_migrations on first apply.
#   Any byte change to a deployed migration file produces a different checksum
#   and causes startup to fail with:
#     "migration N was previously applied but has been modified"
#   This script catches that class of error at commit/CI time, before deployment.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MIGRATIONS_DIR="$REPO_ROOT/edgequake/migrations"
LOCKFILE="$MIGRATIONS_DIR/checksums.lock"
VERBOSE=0

for arg in "$@"; do
  [[ "$arg" == "--verbose" || "$arg" == "-v" ]] && VERBOSE=1
done

log() { [[ $VERBOSE -eq 1 ]] && echo "$*" || true; }

if [[ ! -f "$LOCKFILE" ]]; then
  echo "ERROR: checksums.lock not found at $LOCKFILE"
  echo "  Run: ./scripts/update_migration_checksums.sh to generate it."
  exit 1
fi

FAILED=0
CHECKED=0
MISSING=0

# --- Check: every file in the lockfile exists and matches ---
while IFS= read -r line; do
  # Skip blank lines and comment lines
  [[ -z "$line" || "$line" =~ ^[[:space:]]*# ]] && continue

  expected_hash=$(echo "$line" | awk '{print $1}')
  filename=$(echo "$line" | awk '{print $2}')
  filepath="$MIGRATIONS_DIR/$filename"

  if [[ ! -f "$filepath" ]]; then
    echo "MISSING: $filename (recorded in lockfile but not found on disk)"
    MISSING=$((MISSING + 1))
    FAILED=$((FAILED + 1))
    continue
  fi

  actual_hash=$(sha384sum "$filepath" | awk '{print $1}')
  CHECKED=$((CHECKED + 1))

  if [[ "$actual_hash" != "$expected_hash" ]]; then
    echo "MODIFIED: $filename"
    echo "  expected: $expected_hash"
    echo "  actual:   $actual_hash"
    FAILED=$((FAILED + 1))
  else
    log "OK: $filename"
  fi
done < "$LOCKFILE"

# --- Check: every .sql file on disk appears in the lockfile ---
# (catches new migration files added without updating the lockfile)
NEW_FILES=0
while IFS= read -r -d '' sqlfile; do
  basename_sql=$(basename "$sqlfile")
  if ! grep -q "[[:space:]]${basename_sql}$" "$LOCKFILE" 2>/dev/null; then
    echo "UNLOCKED: $basename_sql (new migration not recorded in checksums.lock)"
    echo "  Run: ./scripts/update_migration_checksums.sh to add it."
    NEW_FILES=$((NEW_FILES + 1))
    FAILED=$((FAILED + 1))
  fi
done < <(find "$MIGRATIONS_DIR" -maxdepth 1 -name '*.sql' -print0 | sort -z)

echo ""
echo "Migration checksum check:"
echo "  Checked : $CHECKED"
echo "  Modified: $FAILED (excluding missing/new)"
echo "  Missing : $MISSING"
echo "  Unlocked: $NEW_FILES"

if [[ $FAILED -gt 0 ]]; then
  echo ""
  echo "FAIL: Migration immutability check failed."
  echo ""
  echo "  Migration files are IMMUTABLE once deployed."
  echo "  If you need schema changes, create a NEW numbered migration file."
  echo "  If you are adding a new migration, run:"
  echo "    ./scripts/update_migration_checksums.sh"
  exit 1
fi

echo "PASS: All $CHECKED migration files match their locked checksums."
exit 0
