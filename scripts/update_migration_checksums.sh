#!/usr/bin/env bash
# scripts/update_migration_checksums.sh
#
# PURPOSE: Append-only update of edgequake/migrations/checksums.lock.
#
# SPEC-150 WP-6: existing lock lines are IMMUTABLE. This script only:
#   1. Appends checksums for new numbered `NNN_*.sql` files.
#   2. Appends checksums for new `support/**/*.sql` files.
#
# It REFUSES to change an existing line (exit 1).
#
# Usage:
#   ./scripts/update_migration_checksums.sh
#   ./scripts/update_migration_checksums.sh --dry-run

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MIGRATIONS_DIR="$REPO_ROOT/edgequake/migrations"
LOCKFILE="$MIGRATIONS_DIR/checksums.lock"
DRY_RUN=0

for arg in "$@"; do
  [[ "$arg" == "--dry-run" ]] && DRY_RUN=1
done

if [[ ! -f "$LOCKFILE" ]]; then
  echo "ERROR: $LOCKFILE missing — create an initial lock manually once."
  exit 1
fi

hash_file() {
  sha384sum "$1" | awk '{print $1}'
}

lock_hash_for() {
  # $1 = filename or relative path as stored in lock
  local file="$1"
  awk -v f="$file" '$2 == f { print $1; exit }' "$LOCKFILE"
}

NEW_LINES=()
DRIFT=0

consider() {
  local filepath="$1"
  local key="$2"
  local hash
  hash=$(hash_file "$filepath")
  local existing
  existing=$(lock_hash_for "$key" || true)
  if [[ -n "$existing" ]]; then
    if [[ "$existing" != "$hash" ]]; then
      echo "REFUSE: $key hash changed (lock is append-only)"
      echo "  lock: $existing"
      echo "  file: $hash"
      DRIFT=1
    fi
  else
    NEW_LINES+=("$hash  $key")
  fi
}

while IFS= read -r -d '' sqlfile; do
  consider "$sqlfile" "$(basename "$sqlfile")"
done < <(find "$MIGRATIONS_DIR" -maxdepth 1 -name '*.sql' -print0 | sort -z)

if [[ -d "$MIGRATIONS_DIR/support" ]]; then
  while IFS= read -r -d '' sqlfile; do
    rel="${sqlfile#"$MIGRATIONS_DIR"/}"
    consider "$sqlfile" "$rel"
  done < <(find "$MIGRATIONS_DIR/support" -type f -name '*.sql' -print0 | sort -z)
fi

if [[ $DRIFT -ne 0 ]]; then
  echo ""
  echo "Aborting: existing lock entries must not change (SPEC-150 LAW-150-4)."
  exit 1
fi

if [[ ${#NEW_LINES[@]} -eq 0 ]]; then
  echo "No new migration/support files to append."
  exit 0
fi

echo "Appending ${#NEW_LINES[@]} new entr(y/ies):"
for line in "${NEW_LINES[@]}"; do
  echo "  $line"
done

if [[ $DRY_RUN -eq 1 ]]; then
  echo "(dry-run — lockfile unchanged)"
  exit 0
fi

printf '%s\n' "${NEW_LINES[@]}" >> "$LOCKFILE"
echo "Updated $LOCKFILE (append-only)."
