#!/usr/bin/env bash
# scripts/update_migration_checksums.sh
#
# PURPOSE: Regenerate edgequake/migrations/checksums.lock from the current
# on-disk migration files.
#
# Run this script ONLY when:
#   1. Adding a brand-new migration file (append its entry).
#   2. Reverting a broken migration file to its canonical content.
#
# NEVER run this to "fix" a checksum mismatch caused by editing a deployed
# migration file. That would bless the mutation and hide the bug.
#
# Usage:
#   ./scripts/update_migration_checksums.sh          # regenerate full lockfile
#   ./scripts/update_migration_checksums.sh --dry-run  # show what would change

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MIGRATIONS_DIR="$REPO_ROOT/edgequake/migrations"
LOCKFILE="$MIGRATIONS_DIR/checksums.lock"
DRY_RUN=0

for arg in "$@"; do
  [[ "$arg" == "--dry-run" ]] && DRY_RUN=1
done

HEADER="# Migration Immutability Lockfile
# Updated: $(date -u +%Y-%m-%d)
#
# PURPOSE: Every line records the SHA-384 of a migration file at the time it was
# declared stable. This file is the source of truth for the migration-guard CI
# workflow and the local check script.
#
# RULES:
#   - Never modify an existing line once the migration has been deployed.
#   - When a NEW migration file is added, append its checksum here as part of the
#     same PR that adds the migration file.
#   - The check script (scripts/check_migration_checksums.sh) will fail CI if
#     any on-disk file diverges from the checksum recorded here.
#
# FORMAT: <sha384>  <filename>   (two spaces, same as sha384sum output)
#"

NEW_CONTENT="$HEADER"$'\n'

while IFS= read -r -d '' sqlfile; do
  filename=$(basename "$sqlfile")
  hash=$(sha384sum "$sqlfile" | awk '{print $1}')
  NEW_CONTENT+="$hash  $filename"$'\n'
done < <(find "$MIGRATIONS_DIR" -maxdepth 1 -name '*.sql' -print0 | sort -z)

if [[ $DRY_RUN -eq 1 ]]; then
  echo "--- DRY RUN: would write to $LOCKFILE ---"
  echo "$NEW_CONTENT"
  exit 0
fi

echo "$NEW_CONTENT" > "$LOCKFILE"
echo "Updated $LOCKFILE"
echo "Files: $(grep -c '\.sql$' "$LOCKFILE") migrations recorded."
echo ""
echo "Remember: commit checksums.lock together with any new migration file."
