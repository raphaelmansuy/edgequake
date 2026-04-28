#!/usr/bin/env bash
# scripts/install_migration_hooks.sh
#
# PURPOSE: Install a pre-commit git hook that blocks modification of existing
# migration files. Protects against the class of bug fixed in issue #195.
#
# Usage:
#   ./scripts/install_migration_hooks.sh
#
# The hook is installed at .git/hooks/pre-commit.
# If a pre-commit hook already exists, a backup is created.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
HOOK_DIR="$REPO_ROOT/.git/hooks"
HOOK_FILE="$HOOK_DIR/pre-commit"

HOOK_CONTENT='#!/usr/bin/env bash
# Migration immutability pre-commit hook.
# Installed by: scripts/install_migration_hooks.sh
#
# Blocks modification of existing migration files. New files (.sql additions)
# are allowed. Run --no-verify to bypass (e.g., when reverting a broken migration).

STAGED_MODIFIED=$(git diff --cached --name-only --diff-filter=M -- '"'"'edgequake/migrations/*.sql'"'"')

if [[ -n "$STAGED_MODIFIED" ]]; then
  echo ""
  echo "⛔  BLOCKED: You are modifying existing migration file(s):"
  echo ""
  for f in $STAGED_MODIFIED; do
    echo "    $f"
  done
  echo ""
  echo "  Migration files are IMMUTABLE once deployed to any database."
  echo "  SQLx stores SHA-384 checksums in _sqlx_migrations at first apply."
  echo "  Changing a deployed migration file will break all existing deployments."
  echo ""
  echo "  To make schema changes: create a NEW migration file."
  echo "  To bypass this check (e.g., reverting a broken migration):"
  echo "    git commit --no-verify"
  echo ""
  exit 1
fi

# Also run the static checksum check if the lockfile was modified
LOCKFILE_STAGED=$(git diff --cached --name-only -- '"'"'edgequake/migrations/checksums.lock'"'"')
SQL_STAGED=$(git diff --cached --name-only --diff-filter=A -- '"'"'edgequake/migrations/*.sql'"'"')
if [[ -n "$SQL_STAGED" ]]; then
  if [[ -z "$LOCKFILE_STAGED" ]]; then
    echo ""
    echo "⚠️   WARNING: You are adding a new migration file but checksums.lock"
    echo "    has not been updated. Run:"
    echo "      ./scripts/update_migration_checksums.sh"
    echo "    and stage the updated checksums.lock before committing."
    echo ""
    # This is a warning, not a block. The CI will catch it.
  fi
fi

exit 0
'

mkdir -p "$HOOK_DIR"

if [[ -f "$HOOK_FILE" ]]; then
  BACKUP="$HOOK_FILE.bak.$(date +%s)"
  cp "$HOOK_FILE" "$BACKUP"
  echo "Existing hook backed up to: $BACKUP"
fi

echo "$HOOK_CONTENT" > "$HOOK_FILE"
chmod +x "$HOOK_FILE"
echo "Installed pre-commit hook at: $HOOK_FILE"
echo ""
echo "The hook will now block modification of existing migration files."
echo "New migration files (.sql additions) are allowed."
